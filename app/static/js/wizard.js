/* Knowlu app: the wizard (v0.4.0 note §6; plan §5 T21). The first-run onboarding, ten panels in one document: the
   same panels, ids, function names, order and behaviour as before, restyled. boot.js calls
   window.KnowluWizard.start(launch, host) when launch_state answers mode "wizard"; the panels are WIZARD_HTML,
   rendered into #boot, and every piece of state lives on WIZ and is painted by renderWizard (the A-5 rule: a click
   handler sets WIZ and renders, never writes the DOM straight).

   The wizard runs under run_shell, which has no state, ui_event or mark_seen: it calls only shell commands, through
   its own invoke() over KnowluData.call, and reaches no shared helper that sends a console command (ev(), KnowluKit,
   the router). Nothing reaches this machine's disk until Finish, except the account (which is not this machine) and
   the coursework credentials, written the moment the student leaves the logins panel. No price, trial or trial
   length is shown (A2): the payment page in the browser shows the price. */
(function () {
  "use strict";

  var WIZARD_HTML =
    '<section class="wiz" id="wizard">' +
    '<div class="wiz-in">' +
    '<div class="wiz-hd"><h1 class="wm wiz-wm">Know<b>lu</b></h1><p class="wiz-n" id="wiz-step"></p></div>' +
    '<div class="wiz-panel" id="wiz-welcome" hidden><h2 tabindex="-1">Welcome</h2>' +
    '<p class="lede">Knowlu answers one question every morning: what should I work on today, and in what order?</p>' +
    '<p class="wiz-p" id="wiz-privacy"></p>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-account" hidden><h2 tabindex="-1">Your account</h2>' +
    '<label class="wiz-box"><input type="checkbox" id="wiz-18"><span>I am 18 or older</span></label>' +
    '<label class="wiz-box"><input type="checkbox" id="wiz-terms"><span>I accept the <a href="terms.html" class="policy" data-policy="terms" id="wiz-terms-link">terms</a> and the <a href="privacy.html" class="policy" data-policy="privacy" id="wiz-privacy-link">privacy policy</a></span></label>' +
    '<div class="wiz-row"><button class="ibtn wiz-google-b" type="button" id="wiz-google-signin">Continue with Google</button></div>' +
    '<p class="meta">Knowlu asks Google for your email address and your name, and nothing else. Your calendar and your mail are separate permissions, asked for later, and you can say no to both.</p>' +
    '<div class="wiz-row"><input class="field" type="text" id="wiz-email" placeholder="Or use your email address" aria-label="Email address" autocomplete="email"><button class="wbtn" type="button" id="wiz-magic">Email me a code</button></div>' +
    '<div class="wiz-row" id="wiz-code-row" hidden><input class="field" type="text" id="wiz-code" placeholder="The code from the email" aria-label="The code from the email" autocomplete="one-time-code"><button class="ibtn" type="button" id="wiz-code-go">Sign in with the code</button></div>' +
    '<p class="meta" id="wiz-account-note" aria-live="polite"></p>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-subscribe" hidden><h2 tabindex="-1">Subscribe</h2>' +
    '<p class="lede">Pick a plan. The payment page opens in your browser and shows the price. Cancel any time in Settings.</p>' +
    '<div class="wiz-row"><button class="ibtn" type="button" id="wiz-sub-month">Monthly</button><button class="wbtn" type="button" id="wiz-sub-year">Academic year</button></div>' +
    '<p class="meta" id="wiz-sub-note" aria-live="polite"></p>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-mail" hidden><h2 tabindex="-1">Next: connect your school email</h2>' +
    '<p class="lede">Knowlu sets up forwarding from your school mailbox, reads what arrives for things you have to do, and proposes each one for you to approve. It starts when Knowlu first opens.</p>' +
    '<div class="wiz-row"><button class="wbtn" type="button" id="wiz-mail-notnow">Not now</button><span class="meta" id="wiz-mail-note"></span></div>' +
    '<p class="meta">If your school&rsquo;s rules don&rsquo;t allow forwarding your school email, skip this.</p>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-vault" hidden><h2 tabindex="-1">Name this setup</h2>' +
    '<p class="lede">Knowlu keeps your work as plain files you can open, copy and take with you.</p>' +
    '<div class="wiz-row"><input class="field" type="text" id="wiz-name" placeholder="Name" aria-label="Name"></div>' +
    '<p class="meta" id="wiz-vault-path"></p>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-calendars" hidden><h2 tabindex="-1">Connect your calendars</h2>' +
    '<p class="lede">Two things, and Knowlu needs both before it can tell you what to do today: what your school says is due, and what your week already looks like.</p>' +
    '<h3>Your school</h3>' +
    '<p class="meta">Start typing its name. Every US college and university is in this list.</p>' +
    '<div class="wiz-row wiz-search"><input class="field" type="text" id="wiz-school" placeholder="Search for your school" aria-label="Your school" autocomplete="off"><span class="meta" id="wiz-school-picked"></span></div>' +
    '<div id="wiz-school-hits" class="wiz-hits"></div>' +
    '<p class="meta"><button class="qbtn" type="button" id="wiz-school-none">My school isn&rsquo;t listed</button></p>' +
    '<div class="wiz-row" id="wiz-school-free" hidden><input class="field" type="text" id="wiz-school-name" placeholder="Your school&rsquo;s name" aria-label="Your school&rsquo;s name"><input class="field wiz-state" type="text" id="wiz-school-state" placeholder="State, e.g. AL" aria-label="State"></div>' +
    '<p class="meta">Sign in the way you always do. Knowlu opens your school&rsquo;s own page in a window, you sign in there, and Knowlu keeps only the calendar link &mdash; never what you type into it.</p>' +
    '<div class="wiz-row" id="wiz-lms-kind" hidden><span class="meta">Which does your school use?</span>' +
    '<button class="wbtn wiz-logo-b" type="button" data-lms="blackboard"><img class="inv" src="logos/blackboard.svg" alt=""><span>Blackboard</span></button>' +
    '<button class="wbtn wiz-logo-b" type="button" data-lms="canvas"><img class="inv" src="logos/canvas.svg" alt=""><span>Canvas</span></button></div>' +
    '<div class="wiz-row"><button class="ibtn" type="button" id="wiz-lms-open">Sign in to my school</button><button class="wbtn" type="button" id="wiz-lms-capture" hidden>I have signed in &mdash; find my calendar</button><span class="meta" id="wiz-lms-state" aria-live="polite"></span></div>' +
    '<p class="meta">Or paste the link yourself: Blackboard &rarr; Calendar &rarr; Calendar Settings &rarr; Share Calendar. Canvas &rarr; Calendar &rarr; Calendar Feed.</p>' +
    '<input class="field" type="text" id="wiz-ics" placeholder="Paste the school feed link (it ends in .ics)" aria-label="School feed link">' +
    '<p class="meta" id="wiz-ics-note"></p>' +
    '<div id="wiz-courses" hidden>' +
    '<h3>Your classes</h3>' +
    '<p class="meta" id="wiz-courses-note"></p>' +
    '<div class="wiz-list" id="wiz-course-rows"></div>' +
    '<div class="wiz-row"><input class="field" type="text" id="wiz-course-add" placeholder="Add a course code, e.g. CS 100" aria-label="Add a course code"><button class="wbtn" type="button" id="wiz-course-add-go">Add</button></div>' +
    '</div>' +
    '<h3>Your own calendar</h3>' +
    '<p class="meta">This is what makes today&rsquo;s page know your day is already half full. In Google Calendar: <b>Settings</b> &rarr; click your calendar under <b>Settings for my calendars</b> &rarr; <b>Integrate calendar</b> &rarr; copy the <b>Secret address in iCal format</b>. Press <b>Reset</b> beside it first if that address has ever been shared with anyone &mdash; whoever holds it can read your calendar.</p>' +
    '<input class="field" type="text" id="wiz-cal-ics" placeholder="Paste the secret iCal address" aria-label="Secret iCal address">' +
    '<p class="meta" id="wiz-cal-note"></p>' +
    '<div class="wiz-row wiz-google-row" id="wiz-google-row"><button class="wbtn wiz-logo-b" type="button" id="wiz-google"><img src="logos/gcal.svg" alt=""><span>Sign in with Google instead</span></button><span class="meta">Reads your Google Calendar so Knowlu knows when you are busy. Read-only, and you can disconnect at any time. While Knowlu is in Google&rsquo;s testing programme this works for invited testers and the connection needs renewing about once a week.</span></div>' +
    '<p class="meta" id="wiz-google-note" aria-live="polite"></p>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-logins" hidden><h2 tabindex="-1">Coursework logins</h2>' +
    '<p class="lede">Optional. Stored in Windows Credential Manager on this machine &mdash; never in the vault, never in a backup, and never sent to us.</p>' +
    '<div class="wiz-login"><img class="inv" src="logos/zybooks.svg" alt="zyBooks"><input class="field" type="text" id="wiz-zy-user" placeholder="zyBooks email" aria-label="zyBooks email" autocomplete="off"><input class="field" type="password" id="wiz-zy-pass" placeholder="zyBooks password" aria-label="zyBooks password" autocomplete="off"></div>' +
    '<div class="wiz-login"><img src="logos/vhl.svg" alt="VHL"><input class="field" type="text" id="wiz-vhl-user" placeholder="VHL email" aria-label="VHL email" autocomplete="off"><input class="field" type="password" id="wiz-vhl-pass" placeholder="VHL password" aria-label="VHL password" autocomplete="off"></div>' +
    '<datalist id="wiz-course-codes"></datalist>' +
    '<div id="wiz-map" hidden>' +
    '<h3 id="wiz-map-heading">Which class is which?</h3>' +
    '<p class="meta" id="wiz-map-note" aria-live="polite">Knowlu found these on your accounts. Confirm the course each one belongs to &mdash; without this, Knowlu can see the work but not what it is for.</p>' +
    '<div class="wiz-list" id="wiz-map-rows"></div>' +
    '</div>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-gmail" hidden><h2 tabindex="-1">Gmail</h2>' +
    '<p class="lede">Knowlu can read your Gmail for things you have to do and propose each one for you to approve. You can skip this and connect later in Settings.</p>' +
    '<p class="meta">While Google reviews Knowlu, this works only for invited testers, and the connection needs renewing about once a week.</p>' +
    '<div class="wiz-row"><img class="wiz-mark" src="logos/gmail.svg" alt=""><button class="wbtn" type="button" id="wiz-gmail-connect">Connect Gmail directly (Google testers only)</button></div>' +
    '<p class="meta" id="wiz-gmail-note" aria-live="polite"></p>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-slots" hidden><h2 tabindex="-1">When should Knowlu look?</h2>' +
    '<div class="wiz-row wiz-slots"><input class="field wiz-tz" type="text" id="wiz-tz" placeholder="Timezone" aria-label="Timezone"><input class="field wiz-slot" type="text" id="wiz-slot1" placeholder="12:00" aria-label="First look"><input class="field wiz-slot" type="text" id="wiz-slot2" placeholder="18:00" aria-label="Second look"></div>' +
    '<label class="wiz-box"><input type="checkbox" id="wiz-autostart" checked><span>Start Knowlu with Windows</span></label>' +
    '</div>' +
    '<div class="wiz-panel" id="wiz-finish" hidden><h2 tabindex="-1">Ready</h2><p class="lede" id="wiz-summary"></p>' +
    '<p class="meta">Your first look runs as soon as you finish, and today&rsquo;s page will be waiting.</p>' +
    '<p class="meta" id="wiz-restore-note"></p>' +
    '</div>' +
    '<div class="wiz-nav"><p class="wiz-err" id="wiz-error" role="alert"></p><button class="obtn" type="button" id="wiz-back">Back</button><button class="btn" type="button" id="wiz-next">Next</button></div>' +
    '</div>' +
    '</section>';

  var PANELS = ["welcome", "account", "subscribe", "mail", "vault", "calendars", "logins", "gmail", "slots", "finish"];
  // One string, two languages: account::UNREACHABLE is the first clause of every transport failure the Rust side
  // emits, and this is the page's copy (ui_wizard.rs pins the two together).
  var UNREACHABLE = "the account service could not be reached";
  // The site's privacy sentence, byte for byte (ui_wizard.rs reads site/privacy.html).
  var PRIVACY = "Your tasks and notes live in a plain-text folder on this machine and in your Knowlu account, so every computer you sign in on opens on the same day; our servers keep them encrypted at rest, beside your account, the judgments made for you and what you correct, and none of it is ever sold or shared.";
  var GOOGLE_TIMEOUT = "Google did not finish connecting. If Google said Knowlu is not verified, this Google account is not on the tester list yet.";
  // Escaped on purpose: the shipped page carries no bare network literal.
  var ICS_OK = /^https:\/\/\S+(\.ics($|\?)|\/calendar\/)/i;
  // Windows' reserved device names, matched on the part before the first dot (onboarding.rs's RESERVED).
  var RESERVED_NAME = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;

  // `parent` comes from launch_state.default_parent and is never picked; it is in WIZ because dest() is what the
  // credential target is derived from, and a rename on the vault panel has to move the coursework logins with it.
  var WIZ = { mailOffer: true, step: 0, parent: "", name: "Knowlu", email: "", accountId: "", entitled: false,
              // the plan the entitlement answer carried, null until it has answered, so the tester form is never
              // drawn on a guess; entryCheck is true while the on-entry ask is out.
              plan: null, entryCheck: false,
              busy: false, accountNote: "",
              ics: "", icsNote: "", cal: "", calNote: "",
              // what the paste or capture said about the link it checked: { url, stored } when it validated, null
              // when it was refused; feedFlags turns it into the plan's per-feed flags at Finish.
              icsFeed: null, calFeed: null,
              // the Google calendar flow: connected, its status line, polling, and the cancellation token wizGo bumps.
              google: false, googleNote: "", googlePolling: false, googleSeq: 0,
              // the Gmail panel's own flow; nothing here reaches the plan, and Next never waits on it.
              gmail: false, gmailNote: "", gmailPolling: false, gmailSeq: 0,
              // the school picked: a unitid, a name, a state and (once known) an LMS kind. Declared once.
              campus: { unitid: "", name: "", state: "", lms: "" },
              // one row per discovered book or section, and the enrolment, captured or typed.
              map: [], courses: [],
              lmsOpen: false, discovering: false, discovered: false, mapWarned: false, checkoutOpened: false, schoolSeq: 0,
              tz: "", tzTouched: false, slots: ["12:00", "18:00"], autostart: true,
              zy: false, vhl: false, credVault: "", error: "",
              // what Finish's own restore found: blank until create_vault answers.
              restoreNote: "" };

  var HOST = null;
  var SHOWN_STEP = -1;
  var HITS = [];

  function EL(id) { return document.getElementById(id); }

  // The one way the wizard reaches the host: KnowluData.call, which never rejects. An unreachable host answers
  // { ok: false, error, state: null } without becoming KnowluData.last(cmd); that one shape is turned back into a
  // rejection here, so every .catch below still means "the command did not answer".
  function invoke(cmd, args) {
    return KnowluData.call(cmd, args || {}).then(function (v) {
      var answered = KnowluData.last(cmd) === v;
      if (!answered && v && v.ok === false && v.state === null && Object.keys(v).length === 3) { throw new Error(v.error); }
      return v;
    });
  }

  // Is this account entitled now, and on which plan: { entitled, plan }, and not entitled on any failure.
  function checkEntitlement() {
    return invoke("entitlement_now", {}).then(function (r) {
      var yes = !!(r && r.ok && (r.status === "active" || r.status === "trialing"));
      return { entitled: yes, plan: yes && r.plan ? String(r.plan) : "" };
    }).catch(function () { return { entitled: false, plan: "" }; });
  }

  // dest_for in onboarding.rs trims both halves and joins them without doubling a separator, and storeCredentials
  // derives the credential target from this string: the two must spell the path the same way.
  function dest() {
    var p = WIZ.parent.trim().replace(/[\\/]+$/, ""), n = WIZ.name.trim();
    return p && n ? p + "\\" + n : "";
  }

  // One sentence for a validated school feed; the courses clause only above zero.
  function feedSummary(link) {
    var s = "Found " + link.events + " events";
    if (link.courses > 0) { s += " across " + link.courses + " courses"; }
    return s + ".";
  }

  function startWizard(l, host) {
    HOST = host || EL("boot");
    HOST.innerHTML = WIZARD_HTML;
    WIZ.tz = l.tz || "";
    WIZ.parent = l.default_parent || "";
    EL("wiz-name").value = WIZ.name;
    EL("wiz-privacy").textContent = PRIVACY;
    EL("wiz-tz").value = WIZ.tz;
    EL("wiz-slot1").value = WIZ.slots[0];
    EL("wiz-slot2").value = WIZ.slots[1];
    bind();
    renderWizard();
  }

  function renderWizard() {
    PANELS.forEach(function (p, i) { EL("wiz-" + p).hidden = i !== WIZ.step; });
    EL("wiz-step").textContent = "step " + (WIZ.step + 1) + " of " + PANELS.length;
    // Hidden, not disabled: a Back that is not on screen at step 0 asks no question.
    EL("wiz-back").hidden = WIZ.step === 0;
    EL("wiz-next").textContent = WIZ.step === PANELS.length - 1 ? "Finish" : "Next";
    // The one writer of this property: wizGo's discovery latch and wizFinish's re-entry guard set WIZ.busy.
    EL("wiz-next").disabled = WIZ.busy;
    EL("wiz-error").textContent = WIZ.error;
    EL("wiz-mail-note").textContent = WIZ.mailOffer ? "" : "Skipped. You can connect it any time in Settings.";
    EL("wiz-mail-notnow").hidden = !WIZ.mailOffer;
    EL("wiz-account-note").textContent = WIZ.accountId ? "Signed in as " + WIZ.email : WIZ.accountNote;
    EL("wiz-google-signin").disabled = WIZ.busy;
    EL("wiz-magic").disabled = WIZ.busy;
    EL("wiz-code-go").disabled = WIZ.busy;
    // Silent until the payment page has actually been opened.
    EL("wiz-sub-note").textContent = WIZ.entitled ? (WIZ.plan === "tester" ? "Your account is already set up. Nothing to pay." : "Your subscription is active.")
      : (WIZ.checkoutOpened ? "Waiting for the payment page in your browser…" : "");
    // An entitled account sees no plan buttons, and none while the on-entry ask is out; for the plan tester the lede
    // goes and the heading is retitled.
    var tester = WIZ.entitled && WIZ.plan === "tester";
    EL("wiz-sub-month").hidden = WIZ.entitled || WIZ.entryCheck;
    EL("wiz-sub-year").hidden = WIZ.entitled || WIZ.entryCheck;
    document.querySelector("#wiz-subscribe .lede").hidden = tester || WIZ.entryCheck;
    document.querySelector("#wiz-subscribe h2").textContent = tester ? "You're all set" : "Subscribe";
    EL("wiz-vault-path").textContent = dest() ? "Your files will be at " + dest() : "";
    EL("wiz-lms-state").textContent = WIZ.icsNote;
    // The capture is a second press, and only exists once a window is open.
    EL("wiz-lms-capture").hidden = !WIZ.lmsOpen;
    // The sign-in page is looked up by school, so the button waits for one to be picked.
    EL("wiz-lms-open").disabled = !WIZ.campus.unitid;
    EL("wiz-ics-note").textContent = WIZ.ics && !ICS_OK.test(WIZ.ics) ? "That does not look like a calendar feed link." : "";
    EL("wiz-cal-note").textContent = WIZ.calNote;
    EL("wiz-google-note").textContent = WIZ.googleNote;
    EL("wiz-google").disabled = WIZ.google || WIZ.googlePolling;
    EL("wiz-gmail-note").textContent = WIZ.gmailNote;
    EL("wiz-gmail-connect").disabled = WIZ.gmail || WIZ.gmailPolling;
    EL("wiz-summary").textContent = dest() + ", looking at " + WIZ.slots.join(" and ") + " " + WIZ.tz + ".";
    EL("wiz-restore-note").textContent = WIZ.restoreNote;
    if (SHOWN_STEP !== WIZ.step) {
      // A new panel: its heading takes focus, so it is heard and focus never falls to <body> from a control the change
      // hid (sign-in moves on by itself). The first paint moves nothing.
      var head = document.querySelector("#wiz-" + PANELS[WIZ.step] + " h2");
      if (SHOWN_STEP !== -1 && head) { head.focus({ preventScroll: true }); }
      SHOWN_STEP = WIZ.step;
      HOST.scrollTop = 0;
    }
  }

  // Leaving the logins panel: write what was typed into Credential Manager, then clear the fields. On a failed write
  // the fields stay full, so the student can retype.
  function clearCredentialFields() {
    ["wiz-zy-pass", "wiz-vhl-pass", "wiz-zy-user", "wiz-vhl-user"].forEach(function (id) { EL(id).value = ""; });
  }
  function storeCredentials() {
    var jobs = [], stored = [], half = false;
    [["zybooks", "wiz-zy-user", "wiz-zy-pass"], ["vhl", "wiz-vhl-user", "wiz-vhl-pass"]].forEach(function (t) {
      var u = EL(t[1]).value.trim(), s = EL(t[2]).value;
      if (!u && !s) { return; }
      // Half a login is a typo, not a choice.
      if (!u || !s) { half = true; return; }
      stored.push(t[0]);
      // The vault does not exist yet: the target is derived from the path Finish will create.
      jobs.push(invoke("store_credentials", { vault: dest(), source: t[0], user: u, secret: s }));
    });
    if (half) { WIZ.error = "Enter both the email and the password, or leave both empty."; return Promise.resolve(false); }
    if (!jobs.length) { return Promise.resolve(true); }
    return Promise.all(jobs).then(function (rs) {
      var bad = rs.filter(function (r) { return r && !r.ok; });
      if (bad.length) { WIZ.error = bad[0].error; return false; }
      // Only now: these flags become enabled sources in the new vault.
      stored.forEach(function (src) { if (src === "zybooks") { WIZ.zy = true; } else { WIZ.vhl = true; } });
      // Re-typed logins are a new answer: a discovery for the old ones must not stand in for these.
      if (stored.length) { WIZ.discovered = false; WIZ.mapWarned = false; }
      WIZ.credVault = dest();
      clearCredentialFields();
      return true;
    }).catch(function () { WIZ.error = "the credential could not be stored"; return false; });
  }

  // The slots panel's answers live in WIZ, read on every keystroke and on the way out.
  function readSlotsPanel() {
    WIZ.tz = EL("wiz-tz").value.trim() || WIZ.tz;
    WIZ.slots = [EL("wiz-slot1").value.trim() || "12:00", EL("wiz-slot2").value.trim() || "18:00"];
    WIZ.autostart = EL("wiz-autostart").checked;
  }

  // A repeated refusal is announced again: one re-flow, and the sentence arrives again.
  function flashError() {
    var el = EL("wiz-error");
    el.classList.remove("flash");
    void el.offsetWidth;
    el.classList.add("flash");
  }

  // The discovery command states a reason; the page appends the way forward.
  function tidy(s) { return String(s || "").replace(/[.\s]+$/, ""); }

  function wizValid() {
    WIZ.error = "";
    if (WIZ.step === 1 && !WIZ.accountId) { WIZ.error = "Sign in first."; }
    if (WIZ.step === 2 && !WIZ.entitled) { WIZ.error = "Finish the payment page in your browser, then come back."; }
    if (WIZ.step === 4) {
      var n = WIZ.name.trim();
      if (!n) { WIZ.error = "Give this setup a name."; }
      else if (/[\\/:]/.test(n)) { WIZ.error = "A name has no slashes or colons."; }
      else if (/[*?"<>|]/.test(n)) { WIZ.error = "A name has no * ? \" < > or | either."; }
      else if (/[. ]$/.test(n)) { WIZ.error = "A name cannot end in a dot or a space."; }
      else if (RESERVED_NAME.test(n.split(".")[0])) { WIZ.error = "That name is reserved by Windows."; }
    }
    // Neither feed is compulsory, but a link that is there and malformed is caught here.
    if (WIZ.step === 5 && WIZ.ics && !ICS_OK.test(WIZ.ics)) { WIZ.error = "That does not look like a calendar feed link."; }
    if (WIZ.step === 5 && WIZ.cal && !ICS_OK.test(WIZ.cal)) { WIZ.error = "That does not look like a secret iCal address."; }
    return !WIZ.error;
  }

  function wizGo(n) {
    // One re-ask of the service before refusing the subscribe step, never from inside the refusal.
    if (n > WIZ.step && WIZ.step === 2 && !WIZ.entitled) {
      WIZ.busy = true; renderWizard();
      return checkEntitlement().then(function (r) {
        WIZ.busy = false;
        if (r.entitled) { WIZ.plan = r.plan; WIZ.entitled = true; }
        return wizStep(n);
      });
    }
    return wizStep(n);
  }

  function wizStep(n) {
    if (n > WIZ.step && !wizValid()) { renderWizard(); flashError(); return Promise.resolve(); }
    // A refusal belongs to the panel that raised it.
    if (n < WIZ.step) { WIZ.error = ""; }
    var leaving = WIZ.step;
    if (leaving === 8) { readSlotsPanel(); }
    // Leaving the calendars panel closes the sign-in window and deletes its session.
    if (leaving === 5 && WIZ.lmsOpen) {
      invoke("close_lms_window", {}).catch(function () {});
      WIZ.lmsOpen = false;
    }
    // Cancel a Google poll in flight; a consent that already finished stays true.
    if (leaving === 5 && WIZ.googlePolling) {
      WIZ.googleSeq += 1;
      WIZ.googlePolling = false;
    }
    if (leaving === 7 && WIZ.gmailPolling) {
      WIZ.gmailSeq += 1;
      WIZ.gmailPolling = false;
    }
    WIZ.step = Math.max(0, Math.min(PANELS.length - 1, n));
    if (leaving === 6 && n > leaving) {
      // One discovery at a time: a second Next while one is in flight is refused.
      if (WIZ.discovering) { WIZ.step = leaving; renderWizard(); return Promise.resolve(); }
      return storeCredentials().then(function (ok) {
        if (!ok) { WIZ.step = leaving; renderWizard(); return; }
        if (!WIZ.zy && !WIZ.vhl) { renderWizard(); return; }
        if (WIZ.map.length || WIZ.discovered) {
          // The first Next after a finished discovery with blank rows writes the sentence and stays; the next goes on.
          if (!WIZ.mapWarned && noteUnmapped()) { WIZ.mapWarned = true; WIZ.step = leaving; }
          renderWizard();
          return;
        }
        WIZ.step = leaving;
        WIZ.discovering = true;
        WIZ.busy = true;
        EL("wiz-map").hidden = false;
        EL("wiz-map-note").textContent = "Looking up your books and sections…";
        renderWizard();
        return invoke("discover_coursework", { vault: dest(), zybooks: WIZ.zy, vhl: WIZ.vhl }).then(function (d) {
          WIZ.map = ((d && d.rows) || []).map(function (r) {
            return { source: r.source, key: r.key, detail: r.detail, suggested: r.suggested, course: r.suggested || "", ignore: !!r.ignored };
          });
          // Set before renderMapping: a finished discovery that found nothing is still a finished discovery.
          WIZ.discovered = true;
          EL("wiz-map-note").textContent = WIZ.map.length
            ? (d && d.note ? tidy(d.note) + ". " : "Knowlu found these on your accounts. ") + "Confirm the course each one belongs to — without this, Knowlu can see the work but not what it is for."
            : tidy((d && d.note) || "We could not reach your coursework sites") + ". You can go on — Knowlu will try again on its first run.";
          renderMapping();
        }).catch(function () {
          WIZ.discovered = true;
          EL("wiz-map-note").textContent = "We could not look those up. You can go on — Knowlu will try again on its first run.";
          renderMapping();
        }).then(function () {
          WIZ.discovering = false;
          WIZ.busy = false;
          renderWizard();
        });
      });
    }
    // On entering the subscribe step, ask status and plan first, so a granted account sees the nothing-to-pay form.
    if (WIZ.step === 2 && leaving !== 2 && !WIZ.entitled) {
      WIZ.entryCheck = true;
      renderWizard();
      return checkEntitlement().then(function (r) {
        WIZ.entryCheck = false;
        if (r.entitled) { WIZ.plan = r.plan; WIZ.entitled = true; WIZ.error = ""; }
        renderWizard();
      });
    }
    renderWizard();
    return Promise.resolve();
  }

  function wizRegister(plan) {
    return invoke("create_vault", { name: WIZ.name, plan: plan });
  }

  // The one place Finish's three restore outcomes are worded, keyed on the explicit ok field.
  function restoreSentence(restored) {
    if (!restored) { return ""; }
    if (!restored.ok) { return "Your account could not be reached just now; your vault will fill in over the next syncs."; }
    if (!restored.empty) { return "Your account already had a vault here — it just came back."; }
    return "Your account had no vault yet — this is the first one.";
  }

  // The logins were keyed to the path as it stood then: say where they went and send the student back to them.
  function credentialsStranded() {
    WIZ.error = "Your logins were saved for " + WIZ.credVault + "; re-enter them.";
    WIZ.zy = false;
    WIZ.vhl = false;
    WIZ.credVault = "";
    WIZ.step = 6;
    WIZ.busy = false;
    renderWizard();
  }

  // Not now is the whole of the choice; the marker offer_mail writes at Finish is all the console reads.
  function wizNotNow() {
    WIZ.mailOffer = false;
    return wizGo(WIZ.step + 1);
  }

  function wizFinish() {
    // Disabled first, before any await, so a second press cannot start a second flow.
    WIZ.busy = true;
    renderWizard();
    readSlotsPanel();
    // The confirmed mapping and the course list, in the shapes WizardPlan takes: course is the confirmed code.
    var zyRows = WIZ.map.filter(function (r) { return r.source === "zybooks" && !r.ignore && r.course; });
    var vhlRows = WIZ.map.filter(function (r) { return r.source === "vhl" && !r.ignore && r.course; });
    var codes = {};
    zyRows.concat(vhlRows).forEach(function (r) { codes[r.course] = true; });
    // Only a typed course (no slug of its own) contributes its code to course_map.
    WIZ.courses.forEach(function (c) { if (c.code && !c.slug) { codes[c.code] = true; } });
    // Re-read the truth rather than trust the poll's last tick; WIZ.google is the fallback.
    return invoke("google_connected").then(function (status) {
      return (status && status.ok) ? status.calendar === true : WIZ.google;
    }).catch(function () { return WIZ.google; }).then(function (googleCalendar) {
      var icsFlags = feedFlags(WIZ.icsFeed, WIZ.ics), calFlags = feedFlags(WIZ.calFeed, WIZ.cal);
      var plan = { ics_url: WIZ.ics || null, personal_calendar: WIZ.cal || null,
                   ics_validated: icsFlags.validated, ics_stored: icsFlags.stored,
                   personal_calendar_validated: calFlags.validated, personal_calendar_stored: calFlags.stored,
                   google_calendar: googleCalendar, offer_mail: WIZ.mailOffer,
                   timezone: WIZ.tz, slots: WIZ.slots,
                   zybooks: WIZ.zy, vhl: WIZ.vhl, autostart: WIZ.autostart,
                   campus_choice: WIZ.campus,
                   zybooks_courses: zyRows.map(function (r) { return { code: r.key, course: r.course, label: r.course }; }),
                   vhl_sections: vhlRows.map(function (r) { return { section: r.key, course: r.course, label: r.course }; }),
                   // The page has no slug: an empty second element tells create_vault_in to derive one.
                   course_map: Object.keys(codes).map(function (c) { return [c, ""]; }),
                   // Only rows the student ticked as ignored; a blank row stays unmapped.
                   zybooks_ignore: WIZ.map.filter(function (r) { return r.source === "zybooks" && r.ignore; })
                                          .map(function (r) { return r.key; }),
                   courses: WIZ.courses };
      // Move the credentials first if the path changed since they were written. The secret never comes back.
      var moved = (WIZ.credVault && WIZ.credVault !== dest())
        ? invoke("retarget_credentials", { from_vault: WIZ.credVault, to_vault: dest() })
        : Promise.resolve({ ok: true, error: null });
      return moved.then(function (rt) {
        if (!rt || !rt.ok) { credentialsStranded(); return; }
        if (WIZ.credVault) { WIZ.credVault = dest(); }
        return wizRegister(plan).then(function (r) {
          if (!r.ok) { WIZ.error = r.error; WIZ.busy = false; renderWizard(); return; }
          WIZ.restoreNote = restoreSentence(r.restored);
          renderWizard();
          // Best effort in both directions: a refusal never stops a finished wizard from opening.
          return invoke("entitlement_now", {}).catch(function () { return null; }).then(function () {
            return invoke("finish_onboarding", { id: r.profile.id });
          });
        });
      });
    }).catch(function (e) { WIZ.error = String(e.message || e); WIZ.busy = false; renderWizard(); });
  }

  // The payment page is in the browser, so the wizard asks: every three seconds for two minutes, then it stops.
  function pollEntitlement() {
    var tries = 0;
    var tick = function () {
      if (WIZ.entitled || WIZ.step !== 2) { return; }
      tries += 1;
      checkEntitlement().then(function (r) {
        // A yes that lands after the student moved on must not act on a panel nobody is looking at.
        if (WIZ.entitled || WIZ.step !== 2) { return; }
        if (r.entitled) {
          WIZ.plan = r.plan; WIZ.entitled = true; WIZ.error = ""; renderWizard(); wizGo(3); return;
        }
        if (tries < 40) { setTimeout(tick, 3000); }
        else { WIZ.error = "Still not subscribed. When the payment page is done, press Next."; renderWizard(); }
      });
    };
    setTimeout(tick, 3000);
  }

  // The plan's two flags for one feed, true only for the value the field holds now.
  function feedFlags(feed, value) {
    var same = !!(feed && value && feed.url === value);
    return { validated: same, stored: same && feed.stored === true };
  }

  // One row per enrolled course, removable; the vault identifier is made in Rust at Finish. The human code first
  // and the school's own name beside it, or just the name when there was no code or the two are the same string.
  function courseHtml(c, i) {
    var plain = !c.label || c.label === c.name;
    var leadHtml = plain ? h(c.name || c.code) : h(c.label) + " &middot; " + h(c.name);
    return '<div class="wiz-item" data-course="' + h(i) + '"><span class="wiz-item-t">' + leadHtml +
           '</span><button class="qbtn" type="button" data-drop="' + h(i) + '">Remove</button></div>';
  }
  function renderCourses() {
    EL("wiz-course-rows").innerHTML = WIZ.courses.map(courseHtml).join("");
    renderCourseCodes();
  }

  // The codes a mapping row offers: the human code the capture read, and the course's own slug otherwise.
  function courseCodeHtml(c) {
    return '<option value="' + h(c.label || c.slug) + '">' + h(c.name || c.code) + "</option>";
  }
  function renderCourseCodes() {
    EL("wiz-course-codes").innerHTML = WIZ.courses.map(courseCodeHtml).join("");
  }

  // One row per discovered book or section: what it is, a field the student corrects, and Ignore. A row nobody can
  // guess for says so, on the next paint (never on every keystroke, which would take the focus out of the field).
  function mapRowHtml(r, i) {
    var hintHtml = (!r.suggested && !r.course) ? '<span class="wiz-hint">type the course this belongs to</span>' : "";
    return '<div class="wiz-item wiz-map-row" data-map="' + h(i) + '"><span class="wiz-item-t">' + h(r.key) +
           (r.detail ? " &middot; " + h(r.detail) : "") + hintHtml + '</span>' +
           '<input class="field" type="text" list="wiz-course-codes" data-course-for="' + h(i) + '" value="' + h(r.course || r.suggested || "") +
           '" placeholder="Course code, e.g. CS 100" aria-label="Course for ' + h(r.key) + '">' +
           '<label class="wiz-box"><input type="checkbox" data-ignore-for="' + h(i) + '"' + (r.ignore ? " checked" : "") + '><span>Ignore</span></label></div>';
  }
  function renderMapping() {
    // A finished discovery that found nothing still has a note to show.
    EL("wiz-map").hidden = WIZ.map.length === 0 && !WIZ.discovered;
    EL("wiz-map-heading").hidden = WIZ.map.length === 0;
    EL("wiz-map-rows").innerHTML = WIZ.map.map(mapRowHtml).join("");
  }

  // Returns the count of blank, un-ignored rows, and says what they cost.
  function noteUnmapped() {
    var n = WIZ.map.filter(function (r) { return !r.ignore && !r.course; }).length;
    if (n) { EL("wiz-map-note").textContent = n + " of these will be asked about in the app"; }
    return n;
  }

  function showWizardError(msg) { WIZ.error = msg; renderWizard(); }

  // The school typeahead: campus_search answers ten rows, and only the newest answer may paint.
  function schoolHits(q) {
    var seq = (WIZ.schoolSeq += 1);
    return invoke("campus_search", { query: q })
      .then(function (r) { return seq === WIZ.schoolSeq ? ((r && r.hits) || []) : null; })
      .catch(function () { return seq === WIZ.schoolSeq ? [] : null; });
  }

  // [unitid, name, city, state, lms]
  function hitHtml(r, i) {
    return '<div class="hit" data-school="' + h(i) + '" tabindex="0" role="button">' + h(r[1]) +
           '<span class="meta"> &middot; ' + h(r[2]) + ", " + h(r[3]) + "</span></div>";
  }
  function renderSchoolHits(hits) {
    EL("wiz-school-hits").innerHTML = hits.map(hitHtml).join("");
    HITS = hits;
  }

  function pickSchool(r) {
    // r[4] is the curated LMS kind, "" for an uncurated school: a curated school is never asked.
    WIZ.campus = { unitid: String(r[0]), name: r[1], state: r[3], lms: r[4] || "" };
    EL("wiz-lms-kind").hidden = true;
    EL("wiz-school").value = r[1];
    EL("wiz-school-hits").innerHTML = "";
    EL("wiz-school-free").hidden = true;
    EL("wiz-school-picked").textContent = r[2] + ", " + r[3];
    WIZ.icsNote = "";
    // The timezone is a suggestion from the state, and only into a field the student has not touched.
    invoke("timezone_for_state", { state: r[3] }).then(function (t) {
      if (t && t.ok && t.timezone && !WIZ.tzTouched) { WIZ.tz = t.timezone; EL("wiz-tz").value = t.timezone; }
    }).catch(function () {});
    renderWizard();
  }

  function bind() {
    EL("wizard").addEventListener("click", function (e) {
      if (e.target.closest("#wiz-mail-notnow")) { wizNotNow(); return; }
      if (e.target.closest("#wiz-back")) { wizGo(WIZ.step - 1); return; }
      if (e.target.closest("#wiz-next")) { if (WIZ.step === PANELS.length - 1) { wizFinish(); } else { wizGo(WIZ.step + 1); } return; }
      // One command, no arguments: the URL, the port, the verifier and the tokens are Rust's.
      if (e.target.closest("#wiz-google-signin")) {
        if (!(EL("wiz-18").checked && EL("wiz-terms").checked)) {
          WIZ.error = "Tick both boxes to create an account."; renderWizard(); return;
        }
        WIZ.busy = true; WIZ.error = ""; WIZ.accountNote = "Finish signing in, in your browser…";
        renderWizard();
        // ageAttested, not the Rust spelling: Tauri camel-cases every argument key.
        invoke("google_sign_in", { ageAttested: EL("wiz-18").checked }).then(function (r) {
          WIZ.busy = false; WIZ.accountNote = "";
          if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
          WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
          wizGo(2);
        }).catch(function () { WIZ.busy = false; WIZ.accountNote = ""; WIZ.error = UNREACHABLE; renderWizard(); });
        return;
      }
      if (e.target.closest("#wiz-magic")) {
        if (WIZ.busy) { return; }
        if (!(EL("wiz-18").checked && EL("wiz-terms").checked)) {
          WIZ.error = "Tick both boxes to create an account."; renderWizard(); return;
        }
        WIZ.busy = true; renderWizard();
        invoke("send_magic_link", { email: EL("wiz-email").value.trim(), ageAttested: EL("wiz-18").checked }).then(function (r) {
          WIZ.busy = false;
          WIZ.error = r.ok ? "" : r.error;
          EL("wiz-code-row").hidden = !r.ok;
          WIZ.accountNote = r.ok ? "We emailed you a code. Type it below." : "";
          renderWizard();
        }).catch(function () { WIZ.busy = false; WIZ.error = UNREACHABLE; renderWizard(); });
        return;
      }
      if (e.target.closest("#wiz-code-go")) {
        if (WIZ.busy) { return; }
        WIZ.busy = true; renderWizard();
        invoke("verify_email_code", { email: EL("wiz-email").value.trim(), code: EL("wiz-code").value }).then(function (r) {
          WIZ.busy = false;
          EL("wiz-code").value = "";
          if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
          WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
          EL("wiz-code-row").hidden = true;
          wizGo(2);
        }).catch(function () { WIZ.busy = false; WIZ.error = UNREACHABLE; renderWizard(); });
        return;
      }
      // The two policies open in the system browser: a plain navigation would lose the only window.
      var policy = e.target.closest("a.policy");
      if (policy) {
        e.preventDefault();
        invoke("open_policy", { which: policy.getAttribute("data-policy") }).catch(function () {});
        return;
      }
      if (e.target.closest("#wiz-sub-month") || e.target.closest("#wiz-sub-year")) {
        var which = e.target.closest("#wiz-sub-year") ? "academic_year" : "monthly";
        // Ask before opening a second payment page: an entitled account goes straight on.
        checkEntitlement().then(function (r) {
          if (r.entitled) { WIZ.plan = r.plan; WIZ.entitled = true; WIZ.error = ""; renderWizard(); wizGo(3); return; }
          return invoke("open_checkout", { plan: which }).then(function (r) {
            if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
            WIZ.checkoutOpened = true;
            renderWizard();
            pollEntitlement();
          });
        }).catch(function () {});
        return;
      }
      if (e.target.closest("#wiz-lms-open")) {
        // No school picked, no sign-in page to open. The button is disabled then; this is the backstop.
        if (!WIZ.campus.unitid) { return; }
        WIZ.icsNote = "Opening your school’s sign-in page…";
        renderWizard();
        invoke("open_lms_window", { unitid: WIZ.campus.unitid }).then(function (r) {
          if (!r.ok) {
            WIZ.icsNote = r.error;
            // Nobody has curated this school: ask which LMS it runs, once.
            EL("wiz-lms-kind").hidden = !!WIZ.campus.lms;
            renderWizard();
            return;
          }
          // The capture is not chained onto the open: the student has not signed in yet.
          WIZ.lmsOpen = true;
          WIZ.icsNote = "Sign in there, then press the button beside this one.";
          renderWizard();
        }).catch(function () { WIZ.icsNote = "That did not work — paste the link below instead."; renderWizard(); });
        return;
      }
      if (e.target.closest("#wiz-lms-capture")) {
        WIZ.icsNote = "Looking for your calendar link…";
        renderWizard();
        invoke("capture_calendar_link", { unitid: WIZ.campus.unitid }).then(function (c) {
          if (c.ok && c.link) {
            WIZ.ics = c.link.url;
            WIZ.icsFeed = { url: c.link.url, stored: c.stored === true };
            WIZ.icsNote = feedSummary(c.link);
            if (c.note) { WIZ.icsNote += " " + c.note; }
          } else {
            WIZ.icsNote = (c.error || "No link found") + " — paste it below instead.";
          }
          renderWizard();
          // The enrolled course list, in the same sitting: its own outcome.
          return invoke("capture_courses", { unitid: WIZ.campus.unitid }).then(function (cl) {
            WIZ.courses = (cl && cl.courses) || [];
            EL("wiz-courses").hidden = false;
            EL("wiz-courses-note").textContent = WIZ.courses.length
              ? "These are the classes Knowlu found. Remove any you are not taking."
              : "Knowlu could not read your class list — type the codes yourself, e.g. CS 100.";
            renderCourses();
          });
        }).catch(function () { WIZ.icsNote = "That did not work — paste the link below instead."; renderWizard(); });
        return;
      }
    });
    EL("wiz-name").addEventListener("input", function () { WIZ.name = EL("wiz-name").value; renderWizard(); });
    EL("wiz-ics").addEventListener("input", function () { WIZ.ics = EL("wiz-ics").value.trim(); renderWizard(); });
    EL("wiz-ics").addEventListener("change", function () {
      WIZ.ics = EL("wiz-ics").value.trim();
      if (!WIZ.ics) { WIZ.icsNote = ""; renderWizard(); return; }
      // The pasted link validates exactly as the captured one does.
      var sent = WIZ.ics;
      invoke("paste_calendar_link", { kind: "lms_ics", url: sent }).then(function (r) {
        WIZ.icsFeed = r.ok ? { url: sent, stored: r.stored === true } : null;
        WIZ.icsNote = r.ok ? feedSummary(r.link) : r.error;
        if (r.ok && r.note) { WIZ.icsNote += " " + r.note; }
        renderWizard();
      }).catch(function () {});
    });
    EL("wiz-courses").addEventListener("click", function (e) {
      var drop = e.target.closest("[data-drop]");
      if (drop) { WIZ.courses.splice(Number(drop.getAttribute("data-drop")), 1); renderCourses(); return; }
      if (e.target.closest("#wiz-course-add-go")) {
        var code = EL("wiz-course-add").value.trim();
        if (code) { WIZ.courses.push({ code: code, name: code, slug: "", label: code }); EL("wiz-course-add").value = ""; renderCourses(); }
      }
    });
    EL("wiz-map").addEventListener("input", function (e) {
      var f = e.target.getAttribute("data-course-for");
      if (f !== null) { WIZ.map[Number(f)].course = e.target.value.trim(); }
    });
    EL("wiz-map").addEventListener("change", function (e) {
      var g = e.target.getAttribute("data-ignore-for");
      if (g !== null) { WIZ.map[Number(g)].ignore = e.target.checked; renderMapping(); }
    });
    // The personal calendar: same command, a different kind and a different sentence.
    EL("wiz-cal-ics").addEventListener("change", function () {
      WIZ.cal = EL("wiz-cal-ics").value.trim();
      if (!WIZ.cal) { WIZ.calNote = ""; renderWizard(); return; }
      var sent = WIZ.cal;
      invoke("paste_calendar_link", { kind: "calendar_ics", url: sent }).then(function (r) {
        WIZ.calFeed = r.ok ? { url: sent, stored: r.stored === true } : null;
        // Zero is a connection, not a failure.
        WIZ.calNote = !r.ok ? r.error
          : (r.link.events === 0 ? "Connected — nothing on it yet." : "Found " + r.link.events + " things already on your calendar.");
        if (r.ok && r.note) { WIZ.calNote += " " + r.note; }
        renderWizard();
      }).catch(function () {});
    });

    // One Google connect for the calendar: nothing is written here; it sets a flag the plan carries. Every field the
    // poll touches lives on WIZ, and mySeq is the cancellation check wizGo bumps on leaving the panel.
    EL("wiz-google").addEventListener("click", async () => {
      var got = await invoke("google_connect_url", { scope: "calendar" });
      if (!got.ok) { showWizardError(got.error); return; }
      var opened = await invoke("open_external", { url: got.url });
      if (!opened.ok) { showWizardError(opened.error); return; }
      var mySeq = ++WIZ.googleSeq;
      WIZ.googlePolling = true;
      WIZ.googleNote = "Finish signing in to Google in your browser — this may take a moment.";
      renderWizard();
      for (var i = 0; i < 20; i++) {
        await new Promise(function (r) { setTimeout(r, 3000); });
        if (WIZ.googleSeq !== mySeq) { return; }
        var status = await invoke("google_connected");
        if (WIZ.googleSeq !== mySeq) { return; }
        if (status.ok && status.calendar) {
          WIZ.google = true;
          WIZ.googlePolling = false;
          WIZ.googleNote = "Google Calendar connected — already on your calendar.";
          renderWizard();
          return;
        }
      }
      WIZ.googlePolling = false;
      WIZ.googleNote = "Google did not finish connecting. You can try again, or use the secret address above.";
      renderWizard();
    });

    // The Gmail panel's Connect Gmail: the same commands with scope gmail, its own token. Never gates Next.
    EL("wiz-gmail-connect").addEventListener("click", async () => {
      var got = await invoke("google_connect_url", { scope: "gmail" });
      if (!got.ok) { showWizardError(got.error); return; }
      var opened = await invoke("open_external", { url: got.url });
      if (!opened.ok) { showWizardError(opened.error); return; }
      var mySeq = ++WIZ.gmailSeq;
      WIZ.gmailPolling = true;
      WIZ.gmailNote = "Finish signing in to Google in your browser — this may take a moment.";
      renderWizard();
      for (var i = 0; i < 20; i++) {
        await new Promise(function (r) { setTimeout(r, 3000); });
        if (WIZ.gmailSeq !== mySeq) { return; }
        var status = await invoke("google_connected");
        if (WIZ.gmailSeq !== mySeq) { return; }
        if (status.ok && status.gmail) {
          WIZ.gmail = true;
          WIZ.gmailPolling = false;
          WIZ.gmailNote = "Gmail is connected.";
          renderWizard();
          return;
        }
      }
      WIZ.gmailPolling = false;
      WIZ.gmailNote = GOOGLE_TIMEOUT;
      renderWizard();
    });

    EL("wiz-school").addEventListener("input", function () {
      schoolHits(EL("wiz-school").value).then(function (hits) { if (hits) { renderSchoolHits(hits); } });
    });
    EL("wiz-school-hits").addEventListener("click", function (e) {
      var hit = e.target.closest("[data-school]");
      if (hit) { pickSchool(HITS[Number(hit.getAttribute("data-school"))]); }
    });
    // Keyboard-selectable: Enter or Space picks, ArrowDown from the field reaches the first hit.
    EL("wiz-school-hits").addEventListener("keydown", function (e) {
      var hit = e.target.closest("[data-school]");
      if (hit && (e.key === "Enter" || e.key === " ")) {
        e.preventDefault();
        pickSchool(HITS[Number(hit.getAttribute("data-school"))]);
      }
    });
    EL("wiz-school").addEventListener("keydown", function (e) {
      if (e.key === "ArrowDown") {
        var first = EL("wiz-school-hits").querySelector("[data-school]");
        if (first) { e.preventDefault(); first.focus(); }
      }
    });
    // A name and a state is enough to make a vault; that school simply has no curated feeds.
    EL("wiz-school-none").addEventListener("click", function () {
      EL("wiz-school-free").hidden = false;
      EL("wiz-school-hits").innerHTML = "";
    });
    EL("wiz-school-free").addEventListener("input", function () {
      WIZ.campus = { unitid: "", name: EL("wiz-school-name").value.trim(), state: EL("wiz-school-state").value.trim().toUpperCase(), lms: WIZ.campus.unitid ? "" : (WIZ.campus.lms || "") };
      // A hand-typed school is asked its LMS directly, once name and state are both in.
      var typed = !!(WIZ.campus.name && WIZ.campus.state);
      EL("wiz-lms-kind").hidden = !(typed && !WIZ.campus.lms);
      WIZ.icsNote = typed ? "Sign-in isn’t available for schools that aren’t listed yet - paste your calendar link below." : "";
      EL("wiz-school-picked").textContent = "";
      renderWizard();
    });
    EL("wiz-lms-kind").addEventListener("click", function (e) {
      var b = e.target.closest("[data-lms]");
      if (b) { WIZ.campus.lms = b.getAttribute("data-lms"); EL("wiz-lms-kind").hidden = true; renderWizard(); }
    });
    // The slots panel, live: the finish summary is rendered from WIZ.
    ["wiz-tz", "wiz-slot1", "wiz-slot2"].forEach(function (id) {
      EL(id).addEventListener("input", function () { readSlotsPanel(); renderWizard(); });
    });
    // A suggestion never overwrites an answer.
    EL("wiz-tz").addEventListener("input", function () { WIZ.tzTouched = true; });
    EL("wiz-autostart").addEventListener("change", function () { readSlotsPanel(); renderWizard(); });
  }

  window.KnowluWizard = {
    start: function (launch, host) { startWizard(launch || {}, host); }
  };
})();
