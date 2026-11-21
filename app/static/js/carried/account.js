/* Knowlu app: the launch's account check and the upgrade sheet (note §5 "Sign-in, upgrade overlay, subscribe"; plan §5
   T20). Settings draws Account, Membership and Delete my data itself (T16); this file draws none of them.

   Once a launch, on the console only (KnowluBoot.onConsole), account_status is asked. An install that predates accounts
   (needs_account) gets the `upgrade` sheet, upgraded in place: never re-onboarded and never asked about a folder; it
   ends in attach_account, which writes the vault's cloud config beside the files already there. A signed-in vault is
   asked about privacy instead (KnowluReconsent.ask, reconsent.js).

   The sheet never traps anyone behind it (spec §5.1, D4): Not now and Escape dismiss it for the session, a sign-in that
   cannot reach the account service stands it down for the session, and the shell holds it back while Settings is the
   route. Two doors, as the wizard has: Continue with Google, then an emailed code; no password. The two plans appear
   only for an account the service does not call entitled, Yearly first, each with the lines checkout_offer answers set
   as text (v0.4.1 note D9: the app holds no price); a plan button opens the payment page and waits for the
   entitlement.

   window.KnowluAccount.after(fn) runs fn once the launch's account and privacy check has settled (at once if it has):
   mail's first open waits on it (M10). */
(function () {
  "use strict";

  /* account.rs's UNREACHABLE: every transport failure there reads "{UNREACHABLE} (…)" */
  var UNREACHABLE = "the account service could not be reached";
  var META = "Your work stays exactly where it is. Sign in and Knowlu picks up where it left off. Today's page is underneath and still works; ranking never needed an account.";
  /* why the plan buttons wait (note D9), the wizard's two sentences; any other refusal is said as given */
  var NO_CONNECTION = "Prices could not be loaded. Check your connection, then try again.";
  var UPDATE_KNOWLU = "Prices could not be loaded. Update Knowlu, then try again.";

  var ACCOUNT = { checked: false, waiting: [] };
  var UPGRADE_DISMISSED = false;     /* Not now or Escape, for this session */
  var UPGRADE_UNREACHABLE = false;   /* the account service could not be reached, for this session */
  var UP_BUSY = false;               /* a sign-in is in flight: two presses are one browser tab */
  /* offer: the server's offer while the sheet holds one, null until it answers; offerSeq drops a late answer;
     checkoutBusy: a plan press until open_checkout answers, so one payment page opens at a time */
  var UP = { root: null, api: null, done: false, offer: null, offerSeq: 0, checkoutBusy: false };
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
      '<div class="sh-doors" id="up-subscribe" hidden>' +
      '<div><button class="btn" type="button" id="up-sub-year" disabled>Yearly</button><p class="sh-p" id="up-sub-year-lines"></p></div>' +
      '<div><button class="obtn" type="button" id="up-sub-month" disabled>Every 4 weeks</button><p class="sh-p" id="up-sub-month-lines"></p></div>' +
      '<p class="sh-hint" id="up-sub-note"></p>' +
      '<div class="sh-line" id="up-sub-wait" hidden><span class="sh-hint" id="up-sub-why" aria-live="polite"></span><button class="obtn" type="button" id="up-sub-retry">Try again</button></div>' +
      "</div></div>" +
      '<p class="sh-err" id="up-error" role="status"></p>' +
      '<div class="sh-acts"><button class="tbtn decline" type="button" id="up-later">Not now</button></div></div>';
  }
  function renderUpgrade(host, api) {
    UP.api = api;
    UP.done = false;
    UP.offer = null;
    UP.checkoutBusy = false;
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
    UP.offer = null;
    UP.checkoutBusy = false;
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

  /* what both doors do once a session exists: ask first, and show the plans only on a not-entitled answer, asking for
     the offer then. Not now or Escape during the browser round trip closed the sheet: a late answer then does nothing. */
  function afterUpgradeSignIn() {
    if (!UP.root) { return null; }
    say("");
    el("up-code-row").hidden = true;
    return checkEntitled().then(function (yes) {
      if (!UP.root) { return null; }
      if (!yes) { el("up-subscribe").hidden = false; return askOffer(); }
      return finishUpgrade();
    });
  }

  /* ---- the offer (note D9): checkout_offer's plans and lines, drawn as text; until one is held both buttons wait ---- */
  /* the answer as the sheet holds it, { offer, why }: the two plans this build sells, each with its whole cents and its
     lines; without both there is no offer. An answer the host never gave, or a refusal that starts with UNREACHABLE's
     clause, asks for the connection; any other refusal is said as given. */
  function offerFrom(r) {
    if (!r || !r.ok) {
      var e = KnowluData.last("checkout_offer") === r ? String((r && r.error) || "") : "";
      return { offer: null, why: !e || e.indexOf(UNREACHABLE) === 0 ? NO_CONNECTION : e };
    }
    var plans = {};
    (Array.isArray(r.plans) ? r.plans : []).forEach(function (p) {
      if (p && (p.plan === "yearly" || p.plan === "four_weekly") && Number.isInteger(p.amount_cents) &&
          Array.isArray(p.lines) && p.lines.length > 0 && p.lines.every(function (l) { return typeof l === "string"; })) {
        plans[p.plan] = p;
      }
    });
    if (!plans.yearly || !plans.four_weekly || typeof r.terms_version !== "string") { return { offer: null, why: UPDATE_KNOWLU }; }
    // The answer as it came, its plans keyed by plan: the cents and the terms version go back unchanged.
    return { offer: Object.assign({}, r, { plans: plans }), why: "" };
  }
  function drawOffer(why) {
    if (!UP.root) { return; }
    var held = UP.offer;
    el("up-sub-year").disabled = !held || UP.checkoutBusy;
    el("up-sub-month").disabled = !held || UP.checkoutBusy;
    el("up-sub-year-lines").textContent = held ? held.plans.yearly.lines.join(" ") : "";
    el("up-sub-month-lines").textContent = held ? held.plans.four_weekly.lines.join(" ") : "";
    el("up-sub-note").textContent = held ? String(held.note || "") : "";
    el("up-sub-why").textContent = why || "";
    el("up-sub-wait").hidden = !why;
  }
  /* ask afresh each time (nothing is kept); only the newest answer is drawn, and none once the sheet has closed */
  function askOffer() {
    if (!UP.root) { return null; }
    var seq = (UP.offerSeq += 1);
    UP.offer = null;
    drawOffer("");
    return KnowluData.call("checkout_offer", {}).then(function (r) {
      if (!UP.root || seq !== UP.offerSeq) { return; }
      var got = offerFrom(r);
      UP.offer = got.offer;
      drawOffer(got.why);
    });
  }

  /* an account the service already calls entitled goes straight through; otherwise the payment page for the plan
     pressed, at the offer's cents and terms version, then a wait. One press at a time: two payment pages opened before
     either is paid could both be paid, so both plan buttons wait until open_checkout answers. */
  function subscribe(plan) {
    var offer = UP.offer;
    if (!offer || UP.checkoutBusy || !offer.plans[plan]) { return; }
    UP.checkoutBusy = true;
    drawOffer("");
    checkEntitled().then(function (yes) {
      if (yes) { UP.checkoutBusy = false; drawOffer(""); return finishUpgrade(); }
      return KnowluData.call("open_checkout", { plan: plan, amountCents: offer.plans[plan].amount_cents, termsVersion: offer.terms_version }).then(function (r) {
        UP.checkoutBusy = false;
        if (!UP.root) { return; }
        /* a refusal (the prices changed, an old build) is the server's sentence, and the offer is asked again */
        if (!r.ok) { el("up-error").textContent = r.error || UNREACHABLE; askOffer(); return; }
        say("");
        drawOffer("");
        var tries = 0;
        var tick = function () {
          if (!UP.root) { return; }
          tries += 1;
          checkEntitled().then(function (yes2) {
            if (!UP.root) { return; }
            if (yes2) { finishUpgrade(); return; }
            if (tries < 40) { setTimeout(tick, 3000); }
            else { say("Still not subscribed. When the payment page is done, choose a plan again."); }
          });
        };
        setTimeout(tick, 3000);
      });
    }).catch(function () { UP.checkoutBusy = false; say(UNREACHABLE); drawOffer(""); });
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
    /* only the two plan buttons start a checkout, with the plan pressed: a click on the lines or the wrapper does nothing */
    var plan = e.target.closest("#up-sub-year") ? "yearly" : e.target.closest("#up-sub-month") ? "four_weekly" : "";
    if (plan) { subscribe(plan); return; }
    if (e.target.closest("#up-sub-retry")) { askOffer(); }
  }

  window.KnowluAccount = {
    after: function (fn) {
      if (typeof fn !== "function") { return; }
      if (ACCOUNT.checked) { fn(); } else { ACCOUNT.waiting.push(fn); }
    }
  };

  KnowluBoot.onConsole(checkAccount);
})();
