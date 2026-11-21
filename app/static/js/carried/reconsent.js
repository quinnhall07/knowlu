/* Knowlu app: privacy re-consent (note §5 "Privacy re-consent", I5, Q-A; plan §5 T20). As shipped: a panel, raised once
   a launch from privacy_status, with Accept, a link to the privacy policy and Settings, and no way to dismiss it.

   It lives in #layer (KnowluLayer), never a sheet: the page behind stays visible and usable, as the change text
   promises. The shell stands the layer down while a sheet is open or Settings is the route and draws it afresh when
   they close, so the two never stack. Settings routes to Data and privacy, where Delete my data and Report a problem
   are reachable without accepting.

   KnowluReconsent.ask(s) is account.js's: s is account_status's answer, and only a signed-in vault whose status answered
   is asked. Only an exact needs_accept === true raises the panel and sets the update-owed flag (KnowluKit.setOwed), in
   one place; Settings redraws each time the flag changes (KnowluSettings.redraw), so every start control locks or frees
   at once. A throw while raising it clears the flag and takes the panel down, so the flag never outlives the panel.

   PRIVACY_CHANGES has one key, the build's PRIVACY_VERSION (account.rs); line 1 is built from the reply's `current`,
   which every privacy_status reply sets to that version, and the lines carry no typed date.

   The panel takes no focus, so it says it is there politely: line 1 sits in a polite live region, written a frame after
   the panel is drawn (a region that arrives already holding its text is not announced). */
(function () {
  "use strict";

  var PRIVACY_CHANGES = {
    "2026-11-21": [
      "KNOWLU LLC operates Knowlu. The policy now names every company that handles part of your data, including Link, Stripe's checkout, which sells the subscription; Resend, which sends our email; and Google, which hosts our support mailbox.",
      "The policy also lists the record of each judgment and the rules learned from it, the sign-in security log, mail you send us, and the two answers you give at setup.",
      "You can choose This computer only, so your plan is not kept in your account; Knowlu still sends our servers what it needs to work, and the policy lists all of it.",
      "No one at Knowlu reads your tasks or mail without your explicit consent unless a valid legal demand requires it, and a new section says who can access your data.",
      "Email you forward to Knowlu, received by Amazon Web Services, is deleted as soon as Knowlu is done with it, and disconnecting Gmail or a mailbox deletes the records of what Knowlu decided about its email.",
      "Kept sign-ins are offered at setup, already on and marked Recommended, and you can turn each one off at setup or later in Settings.",
      "If you apply to the pilot on our website, your answers are kept in Knowlu's Google account, used only for the pilot and deleted when the pilot ends, or sooner if you ask.",
      "Until you accept, Knowlu keeps today's page and your calendar up to date, but does not sync, import or read anything else new.",
      "To leave instead, use Settings → Delete my data."
    ]
  };

  /* the shipped sentence after the policy link: the change text promises the page stays usable */
  var STILL_WORKS = "Today's page is underneath and still works.";
  /* line 1's place: a polite live region, written a frame after the panel is drawn (renderPanel) */
  var LINE1_HTML = '<p id="rc-line1" aria-live="polite"></p>';

  var RECONSENT_BUSY = false;   /* two presses on Accept are one command */
  var PRIVACY_ASKED = false;    /* once a launch */
  var LINES = [];               /* what the panel says, from the reply */
  var ROOT = null;
  var PANEL = { render: renderPanel };

  function el(id) { return document.getElementById(id); }
  function say(text) {
    var e = el("rc-error");
    if (e) { e.textContent = text || ""; }
  }
  function redrawSettings() {
    if (window.KnowluSettings) {
      try { KnowluSettings.redraw(); } catch (e) { window.console.error(e); }
    }
  }

  function fillReconsent(p) {
    var key = p.current;
    LINES = ["Knowlu's privacy policy changed on " + key + "."].concat(PRIVACY_CHANGES[key] || []);
  }

  function lineHtml(l, i) {
    if (i === 0) { return LINE1_HTML; }
    return "<p>" + h(l) + "</p>";
  }
  function panelHtml() {
    return '<div class="layer-in rc" role="group" aria-labelledby="rc-h"><h2 class="rc-h" id="rc-h">Our privacy policy changed</h2>' +
      '<div class="rc-changes" id="rc-changes">' + LINES.map(lineHtml).join("") + "</div>" +
      '<p class="rc-read"><button class="sh-link" type="button" data-policy="privacy">Read the privacy policy</button>. ' + h(STILL_WORKS) + "</p>" +
      '<p class="sh-err" id="rc-error" role="alert"></p>' +
      '<div class="sh-acts"><button class="obtn" type="button" id="rc-settings">Settings</button><button class="btn small" type="button" id="rc-accept">Accept</button></div></div>';
  }
  function renderPanel(host) {
    host.innerHTML = panelHtml();
    ROOT = host.querySelector(".rc");
    ROOT.addEventListener("click", onPanelClick);
    if (RECONSENT_BUSY) { el("rc-accept").disabled = true; }
    var drawn = ROOT;
    requestAnimationFrame(function () {
      var line = el("rc-line1");
      if (line && ROOT === drawn) { line.textContent = LINES[0] || ""; }
    });
  }

  function ask(s) {
    if (PRIVACY_ASKED || !s || !s.ok || s.needs_account) { return null; }
    PRIVACY_ASKED = true;
    return KnowluData.call("privacy_status", {}).then(function (p) {
      try {
        if (p && p.needs_accept === true) {
          fillReconsent(p);
          KnowluKit.setOwed(true);
          redrawSettings();
          KnowluLayer.show(PANEL);
        }
      } catch (e) {
        KnowluKit.setOwed(false);
        KnowluLayer.hide();
        redrawSettings();
        window.console.error(e);
      }
    });
  }

  function acceptPrivacy() {
    if (RECONSENT_BUSY) { return; }
    RECONSENT_BUSY = true;
    var btn = el("rc-accept");
    if (btn) { btn.disabled = true; }
    say("");
    KnowluData.call("accept_privacy", {}).then(function (r) {
      RECONSENT_BUSY = false;
      var again = el("rc-accept");
      if (again) { again.disabled = false; }
      if (!r || !r.ok) { say(KnowluKit.refusal(r)); return; }
      KnowluKit.setOwed(false);
      KnowluLayer.hide();
      ROOT = null;
      redrawSettings();
      KnowluData.refresh();
    });
  }

  function onPanelClick(e) {
    var policy = e.target.closest("[data-policy]");
    if (policy) {
      e.preventDefault();
      KnowluData.call("open_policy", { which: policy.getAttribute("data-policy") });
      return;
    }
    if (e.target.closest("#rc-accept")) { acceptPrivacy(); return; }
    if (e.target.closest("#rc-settings")) { KnowluRouter.go("settings/data"); }
  }

  window.KnowluReconsent = { ask: ask };
})();
