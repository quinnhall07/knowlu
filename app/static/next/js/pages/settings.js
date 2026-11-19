/* Knowlu app: Settings (note §4 "Settings", §5; plan §5 T16). One page, #settings[/<section>], one column of sections:
   a display-face title over hairline rows, each row a label (with at most one quiet line under it) and its control.

   The frame draws Account, Membership, Grades, Appearance, About you, This computer and Data and privacy itself, and
   hosts the carried sections other files register with KnowluRouter.addSection(name, mod), in page order (SLOTS):
     sources   Sources: grades, school email, Google (carried/sources.js)
     week      Your week and Day (carried/week.js)
     status    Status: the sync line, runs, issues, warnings (carried/status.js)
     updates   This computer's Updates row (carried/update.js)
     report    Data and privacy's Report a problem row (carried/report.js)
   A section module: mount(host, api) draws into its host; update(state) runs on every newly painted revision;
   account(a), mail(s) and context(c) run when Settings' own reads answer; unmount() runs when Settings is left. api has
   account(), mail(), context() (the last answers, or null) and redraw(). With no module registered a host stays empty.

   Reads on mount (M6): get_settings, settings_context, account_status, grades_status, mail_status and profile. Settings
   never asks Google or the week itself (P4): their sections do, when the frame mounts them. #settings/<section> scrolls
   to the section named by data-section once the router has reset #center's scroll.

   window.KnowluSettings.redraw() asks every mounted section to draw again (the re-consent module calls it when the
   update-owed flag changes, so start controls lock or free at once). Nothing here keeps anything in storage: the three
   Appearance choices are prefs.js's (D4). No price, plan price or trial length appears (A2). */
(function () {
  "use strict";
  var SLOTS = ["sources", "week", "status", "updates", "report"];
  var EM_FORWARD_SENTENCE = "If Knowlu cannot turn forwarding off, your mail keeps going to Knowlu's address and is deleted unread; you can turn it off in your mail settings.";
  var PREFS_COPY = "Knowlu’s judge reads the first 600 characters of this when it sizes up a task. They are sent for that one call, and neither we nor the model’s host keep them. Otherwise this stays on this computer.";
  var INT_NOTE = "One item per line. Knowlu uses these to choose which campus events to show you. The first 600 characters also go to its judge with each event it judges, for that one call, and neither we nor the model’s host keep them. Otherwise these lists stay on this computer.";
  var INT_REASON = "Knowlu can only edit a list written on one line. Yours continues onto following lines, so it is shown here to read only.";
  /* account_status's status, as the cloud answers it (entitlement/handler.ts); a word not here reads neutral, never
     Active. trialing reads Active: Settings names no trial (A2). */
  var STATUS_WORDS = { active: "Active", trialing: "Active", none: "No subscription", past_due: "Payment due", unpaid: "Payment due", canceled: "Ended", incomplete: "Not finished", incomplete_expired: "Ended", paused: "Paused" };
  var PLAN_WORDS = { monthly: "Monthly", academic_year: "Academic year", yearly: "Yearly" };
  var INT_KEYS = ["strong", "mild", "never", "clubs"];

  var ROOT = null;       /* the page's element while Settings shows */
  var SEQ = 0;           /* bumped on unmount: a late answer draws nothing */
  var MOUNTED = [];      /* the carried sections mounted into their hosts */
  var ACCOUNT = null;    /* account_status's last answer */
  var MAIL = null;       /* mail_status's last answer */
  var CONTEXT = null;    /* settings_context's last answer */
  var TOAST = null;      /* the one Saved, Undo toast */
  var PREFS = { loaded: "", started: false };
  var INT = { loaded: null, started: false, editable: true };

  function byId(id) { return document.getElementById(id); }
  function say(el, text, bad) {
    if (!el) { return; }
    el.textContent = text || "";
    el.classList.toggle("st-bad", !!bad);
  }
  function live(seq) { return seq === SEQ && !!ROOT; }

  /* ---- the page ---- */
  function pageHtml() {
    return '<div class="page st-page"><h1 class="page-h">Settings</h1><div class="st">' +
      accountHtml() + slotHtml("sources") + slotHtml("week") + gradesHtml() + appearanceHtml() + aboutHtml() + slotHtml("status") + computerHtml() + dataHtml() +
      "</div></div>";
  }
  function slotHtml(name) {
    return '<div class="st-slot" data-slot="' + h(name) + '"></div>';
  }
  function accountHtml() {
    return '<section class="st-sec" data-section="account" aria-labelledby="st-h-account"><h2 class="st-h" id="st-h-account" tabindex="-1">Account</h2><ul class="st-rows">' +
      '<li class="st-row" id="set-account"><div class="st-l"><p class="st-n" id="set-account-name">Account</p><p class="st-hint" id="set-account-state">Reading this computer’s account</p></div></li>' +
      "</ul></section>" +
      '<section class="st-sec" data-section="membership" aria-labelledby="st-h-member"><h2 class="st-h" id="st-h-member" tabindex="-1">Membership</h2><ul class="st-rows">' +
      '<li class="st-row" id="set-member"><div class="st-l"><p class="st-n" id="set-member-state">Membership</p><p class="st-hint" id="set-member-hint"></p><p class="st-hint" id="set-member-note" role="status"></p></div>' +
      '<div class="st-c"><button class="obtn" type="button" hidden id="set-portal">Manage subscription</button></div></li>' +
      "</ul></section>";
  }
  function gradesHtml() {
    return '<section class="st-sec" data-section="grades" aria-labelledby="st-h-grades"><h2 class="st-h" id="st-h-grades" tabindex="-1">Grades</h2><ul class="st-rows">' +
      '<li class="st-row" id="set-grades-hide"><div class="st-l"><p class="st-n" id="set-grades-hide-l">Hide grades</p><p class="st-hint">Classes shows Hidden in place of each grade.</p></div>' +
      '<div class="st-c"><button class="switch" type="button" role="switch" id="set-grades-hide-in" aria-checked="false" aria-labelledby="set-grades-hide-l"></button></div></li>' +
      "</ul></section>";
  }
  function appearanceHtml() {
    function choiceHtml(name, value, label, on, face) {
      return '<label><input type="radio" name="' + h(name) + '" value="' + h(value) + '"' + (on ? " checked" : "") + '><i class="rd" aria-hidden="true"></i><span class="nm' + (face ? " " + h(face) : "") + '">' + h(label) + "</span></label>";
    }
    var theme = KnowluPrefs.theme(), font = KnowluPrefs.font(), text = KnowluPrefs.text();
    return '<section class="st-sec" data-section="appearance" aria-labelledby="st-h-look"><h2 class="st-h" id="st-h-look" tabindex="-1">Appearance</h2><ul class="st-rows">' +
      '<li class="st-row"><div class="st-l"><p class="st-n" id="st-theme-l">Theme</p></div><div class="st-c"><fieldset class="opts" aria-labelledby="st-theme-l">' +
      choiceHtml("st-theme", "system", "System", theme === "system", "") + choiceHtml("st-theme", "light", "Light", theme === "light", "") + choiceHtml("st-theme", "dark", "Dark", theme === "dark", "") +
      "</fieldset></div></li>" +
      '<li class="st-row"><div class="st-l"><p class="st-n" id="st-font-l">Reading font</p></div><div class="st-c"><fieldset class="opts" aria-labelledby="st-font-l">' +
      choiceHtml("st-font", "default", "Standard", font !== "dyslexic", "nm-std") + choiceHtml("st-font", "dyslexic", "OpenDyslexic", font === "dyslexic", "nm-dys") +
      "</fieldset></div></li>" +
      '<li class="st-row"><div class="st-l"><p class="st-n" id="st-text-l">Text size</p></div><div class="st-c"><fieldset class="opts" aria-labelledby="st-text-l">' +
      choiceHtml("st-text", "default", "Default", text !== "larger", "") + choiceHtml("st-text", "larger", "Larger", text === "larger", "") +
      "</fieldset></div></li>" +
      "</ul></section>";
  }
  function aboutHtml() {
    return '<section class="st-sec" data-section="about" aria-labelledby="st-h-about"><h2 class="st-h" id="st-h-about" tabindex="-1">About you</h2>' +
      '<div class="st-block" id="set-prefs-sec"><h3 class="st-sub">Your preferences</h3>' +
      '<div class="st-field" id="set-prefs"><label class="st-n" for="set-prefs-ta">How you like to work</label>' +
      '<textarea class="st-ta" id="set-prefs-ta" rows="5"></textarea>' +
      '<div class="st-acts"><span class="st-count" id="set-prefs-count" aria-live="polite"></span><button class="qbtn" type="button" id="set-prefs-cancel">Cancel</button><button class="tbtn" type="button" id="set-prefs-save">Save</button></div>' +
      '<p class="st-hint" id="set-prefs-note" role="status"></p>' +
      '<p class="st-hint">' + h(PREFS_COPY) + "</p></div></div>" +
      '<div class="st-block" id="set-ints"><h3 class="st-sub">Campus events</h3><div class="st-grid">' +
      '<label class="st-int"><span class="st-n">Always show me</span><textarea class="st-ta" id="set-int-strong" rows="3"></textarea></label>' +
      '<label class="st-int"><span class="st-n">Maybe</span><textarea class="st-ta" id="set-int-mild" rows="3"></textarea></label>' +
      '<label class="st-int"><span class="st-n">Never show me</span><textarea class="st-ta" id="set-int-never" rows="3"></textarea></label>' +
      '<label class="st-int"><span class="st-n">Clubs I’m in</span><textarea class="st-ta" id="set-int-clubs" rows="3"></textarea></label>' +
      "</div>" +
      '<p class="st-hint" id="set-int-reason" hidden>' + h(INT_REASON) + "</p>" +
      '<div class="st-acts"><button class="qbtn" type="button" id="set-int-cancel">Cancel</button><button class="tbtn" type="button" id="set-int-save">Save</button></div>' +
      '<p class="st-hint" id="set-int-say" role="status"></p>' +
      '<p class="st-hint" id="set-int-note">' + h(INT_NOTE) + "</p></div>" +
      "</section>";
  }
  function computerHtml() {
    return '<section class="st-sec" data-section="computer" aria-labelledby="st-h-computer"><h2 class="st-h" id="st-h-computer" tabindex="-1">This computer</h2><ul class="st-rows">' +
      '<li class="st-row" id="set-name"><div class="st-l"><label class="st-n" for="set-name-in">Profile name</label><p class="st-hint" id="set-name-note" role="status"></p></div>' +
      '<div class="st-c st-wide"><input class="field" type="text" id="set-name-in" autocomplete="off"><button class="tbtn" type="button" id="set-name-save">Save</button></div></li>' +
      '<li class="st-row" id="set-vault"><div class="st-l"><p class="st-n">Vault</p><p class="st-hint st-path" id="set-vault-path"></p></div>' +
      '<div class="st-c"><button class="obtn" type="button" id="set-vault-copy">Copy</button><button class="obtn" type="button" id="set-switch">Switch profile…</button></div></li>' +
      '<li class="st-row" id="set-backup"><div class="st-l"><p class="st-n">Backup folder</p><p class="st-hint st-path" id="set-bdir"></p></div>' +
      '<div class="st-c"><button class="obtn" type="button" id="set-bdir-pick">Choose…</button><button class="obtn" type="button" id="set-backup-now">Back up now</button></div></li>' +
      '<li class="st-row" id="set-autostart"><div class="st-l"><p class="st-n" id="set-autostart-l">Start with Windows</p></div>' +
      '<div class="st-c"><button class="switch" type="button" role="switch" id="set-autostart-in" aria-checked="false" aria-labelledby="set-autostart-l"></button></div></li>' +
      "</ul>" +
      '<ul class="st-rows st-slot" data-slot="updates"></ul>' +
      '<ul class="st-rows">' +
      '<li class="st-row" id="set-diag"><div class="st-l"><p class="st-n">Diagnostics</p><p class="st-hint" id="set-diag-note" role="status"></p></div>' +
      '<div class="st-c"><button class="obtn" type="button" id="set-diag-copy">Copy diagnostics</button></div></li>' +
      "</ul></section>";
  }
  function dataHtml() {
    return '<section class="st-sec" data-section="data" aria-labelledby="st-h-data"><h2 class="st-h" id="st-h-data" tabindex="-1">Data and privacy</h2><ul class="st-rows">' +
      '<li class="st-row" id="set-policy"><div class="st-l"><p class="st-n">What Knowlu keeps</p></div>' +
      '<div class="st-c"><button class="obtn" type="button" data-policy="privacy">Privacy policy</button><button class="obtn" type="button" data-policy="terms">Terms</button></div></li>' +
      "</ul>" +
      '<ul class="st-rows st-slot" data-slot="report"></ul>' +
      '<ul class="st-rows">' +
      '<li class="st-row" id="set-profile-pending" hidden><div class="st-l"><p class="st-hint st-warn">A saved email sign-in could not be deleted from this computer yet. Knowlu tries again each time it starts.</p></div></li>' +
      '<li class="st-row" id="set-delete"><div class="st-l"><p class="st-n">Delete my data</p><p class="st-hint" id="set-delete-note" role="status"></p></div>' +
      '<div class="st-c"><button class="tbtn st-danger" type="button" id="set-delete-1">Delete everything</button><button class="btn small" type="button" id="set-delete-2" hidden>Yes, delete it all</button></div></li>' +
      "</ul></section>";
  }

  /* ---- what the reads answer ---- */
  function notify(kind, value) {
    MOUNTED.forEach(function (mod) {
      var fn = kind === "account" ? mod.account : kind === "mail" ? mod.mail : kind === "context" ? mod.context : null;
      if (typeof fn === "function") {
        try { fn.call(mod, value); } catch (e) { window.console.error(e); }
      }
    });
  }
  function periodEnd(stamp) {
    var p = stamp ? KnowluFmt.parse(String(stamp).slice(0, 10)) : null;
    return p ? KnowluFmt.date(p.d) : "";
  }
  function drawAccount(a) {
    ACCOUNT = a;
    if (!ROOT) { return; }
    if (!a || !a.ok) {
      say(byId("set-account-name"), "Account");
      say(byId("set-account-state"), (a && a.error) || "Knowlu could not read this computer’s account just now.", true);
    } else if (a.needs_account) {
      say(byId("set-account-name"), "No account");
      say(byId("set-account-state"), "Not attached to an account yet");
    } else {
      say(byId("set-account-name"), a.email || "Signed in");
      say(byId("set-account-state"), "Signed in on this computer");
    }
    drawMember(a);
    notify("account", a);
  }
  /* The status carries nothing about cancellation (a subscription cancelled at the period's end stays active until it
     lapses), so the period's end reads Through <date> for every status, never a renewal. */
  function drawMember(a) {
    var portal = byId("set-portal");
    var tester = !!(a && a.ok && !a.needs_account && a.plan === "tester");
    portal.hidden = !a || !a.ok || !!a.needs_account || tester || a.status === "none";
    if (!a || !a.ok) { say(byId("set-member-state"), "Membership"); say(byId("set-member-hint"), "Not known just now"); return; }
    if (a.needs_account) { say(byId("set-member-state"), "No membership on this computer"); say(byId("set-member-hint"), ""); return; }
    var end = periodEnd(a.current_period_end);
    if (tester) { say(byId("set-member-state"), "Free tester access"); say(byId("set-member-hint"), (end ? "Through " + end + ". " : "") + "Nothing to pay."); return; }
    var word = !a.status ? "Not checked yet" : Object.prototype.hasOwnProperty.call(STATUS_WORDS, a.status) ? STATUS_WORDS[a.status] : "Membership";
    var plan = a.plan && Object.prototype.hasOwnProperty.call(PLAN_WORDS, a.plan) ? PLAN_WORDS[a.plan] : "";
    say(byId("set-member-state"), word + (plan ? ", " + plan : ""));
    say(byId("set-member-hint"), end ? "Through " + end : "");
  }
  function drawGrades(g) {
    var sw = byId("set-grades-hide-in");
    if (!sw || !g || g.ok === false) { return; }
    sw.setAttribute("aria-checked", g.hidden ? "true" : "false");
  }
  function drawSettings(s) {
    if (!ROOT || !s) { return; }
    say(byId("set-bdir"), s.backup_dir || "Not set");
    byId("set-autostart-in").setAttribute("aria-checked", s.autostart ? "true" : "false");
  }
  function drawContext(c) {
    CONTEXT = c;
    if (!ROOT || !c || !c.ok) { return; }
    var input = byId("set-name-in");
    if (input && document.activeElement !== input) { input.value = c.profile_name || ""; }
    say(byId("set-vault-path"), c.vault || "");
    /* never silently: an unreadable profile registry means the name above is the folder's, not this profile's */
    if (c.registry_error) { say(byId("set-diag-note"), c.registry_error, true); }
    notify("context", c);
  }
  function drawMail(s) {
    MAIL = s;
    if (!ROOT) { return; }
    byId("set-profile-pending").hidden = !(s && s.profile_delete_pending);
    notify("mail", s);
  }

  /* ---- a Saved, Undo toast held ten seconds (KnowluKit.toast's six are too short to change your mind) ---- */
  function undoToast(text, action, fn) {
    var host = byId("toasts");
    if (!host) { return null; }
    if (TOAST && TOAST.parentNode) { TOAST.parentNode.removeChild(TOAST); }
    var t = document.createElement("div");
    t.className = "toast";
    t.setAttribute("role", "status");
    var line = document.createElement("span");
    line.textContent = text;
    t.appendChild(line);
    var b = document.createElement("button");
    b.type = "button";
    b.className = "tbtn";
    b.textContent = action;
    var gone = function () { if (t.parentNode) { t.parentNode.removeChild(t); } };
    var timer = setTimeout(gone, 10000);
    b.addEventListener("click", function () { clearTimeout(timer); gone(); fn(); });
    t.appendChild(b);
    host.appendChild(t);
    TOAST = t;
    return t;
  }

  /* ---- About you: "How you like to work". PREFS.loaded is the text profile last returned, the expected of a save.
     Usage events carry the object kind only, never the text. ---- */
  function prefsDirty() { var ta = byId("set-prefs-ta"); return !!ta && ta.value !== PREFS.loaded; }
  function prefsCount() {
    var ta = byId("set-prefs-ta"), c = byId("set-prefs-count");
    if (!ta || !c) { return; }
    var n = Array.from(ta.value).length;
    c.textContent = n + " / 600";
    c.classList.toggle("st-over", n > 600);
  }
  function prefsLock(on) { ["set-prefs-ta", "set-prefs-save", "set-prefs-cancel"].forEach(function (id) { var el = byId(id); if (el) { el.disabled = on; } }); }
  function prefsFill(p) {
    var ta = byId("set-prefs-ta");
    if (!ta) { return; }
    PREFS.loaded = p.preferences || "";
    PREFS.started = false;
    ta.value = PREFS.loaded;
    prefsCount();
    var c = ROOT ? ROOT.querySelector("[data-prefs-conflict]") : null;
    if (c) { c.parentNode.removeChild(c); }
  }
  function loadPrefs() {
    if (prefsDirty()) { return Promise.resolve(); }
    var seq = SEQ;
    return KnowluData.call("profile", {}).then(function (env) {
      if (!live(seq)) { return; }
      if (env && env.ok && env.profile && !prefsDirty()) { prefsFill(env.profile); }
    });
  }
  function prefsConflict() {
    if (!ROOT || ROOT.querySelector("[data-prefs-conflict]")) { return; }
    var wrap = document.createElement("div");
    wrap.className = "st-conflict";
    wrap.setAttribute("data-prefs-conflict", "");
    wrap.innerHTML = '<p>This changed since you opened it. Your text is still here. Copy it, then reload to see the new version.</p>' +
      '<div class="st-acts"><button class="tbtn" type="button" data-prefs-copy>Copy</button><button class="qbtn" type="button" data-prefs-reload>Reload</button></div>';
    byId("set-prefs").appendChild(wrap);
    wrap.querySelector("[data-prefs-copy]").addEventListener("click", function () {
      KnowluData.call("copy_text", { text: byId("set-prefs-ta").value });
    });
    wrap.querySelector("[data-prefs-reload]").addEventListener("click", function () {
      if (prefsDirty() && !window.confirm("Reload and discard your text?")) { return; }
      ev("edit_cancelled", null, "preferences");
      KnowluData.call("profile", {}).then(function (env) { if (env && env.ok && env.profile) { prefsFill(env.profile); } });
    });
  }
  function savePrefs() {
    var before = PREFS.loaded;
    var ta = byId("set-prefs-ta");
    say(byId("set-prefs-note"), "");
    prefsLock(true);
    KnowluData.call("set_preferences", { expected: PREFS.loaded, text: ta.value }).then(function (env) {
      if (env && env.conflict) { prefsLock(false); prefsConflict(); return; }
      if (!env || !env.ok) { prefsLock(false); say(byId("set-prefs-note"), KnowluKit.refusal(env), true); return; }
      ev("edit_committed", null, "preferences");
      return KnowluData.call("profile", {}).then(function (p) {
        prefsLock(false);
        if (!p || !p.ok || !p.profile) { return; }
        prefsFill(p.profile);
        offerPrefsUndo(before, PREFS.loaded);
      });
    });
  }
  function offerPrefsUndo(before, after) {
    undoToast("Saved", "Undo", function () {
      KnowluData.call("set_preferences", { expected: after, text: before }).then(function (r) {
        if (r && r.ok) {
          ev("edit_committed", null, "preferences");
          KnowluData.call("profile", {}).then(function (p) { if (p && p.ok && p.profile) { prefsFill(p.profile); } });
          return;
        }
        KnowluKit.toast(KnowluKit.refusal(r), "Copy previous text", function () { KnowluData.call("copy_text", { text: before }); }, true);
      });
    });
  }
  function bindPrefs(root) {
    var ta = root.querySelector("#set-prefs-ta");
    ta.addEventListener("input", function () {
      var first = !PREFS.started;
      PREFS.started = true;
      if (first) {
        ev("edit_started", null, "preferences");
      }
      prefsCount();
    });
    ta.addEventListener("keydown", function (e) {
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); savePrefs(); }
    });
    byId("set-prefs-save").addEventListener("click", savePrefs);
    byId("set-prefs-cancel").addEventListener("click", function () {
      if (prefsDirty()) {
        ev("edit_cancelled", null, "preferences");
      }
      byId("set-prefs-ta").value = PREFS.loaded;
      PREFS.started = false;
      prefsCount();
    });
  }

  /* ---- About you: "Campus events". INT.loaded is the four lists profile last returned; Undo sends the ones held
     before the save. Usage events carry the object kind only, never an item. ---- */
  function intRead() {
    var out = {};
    INT_KEYS.forEach(function (k) {
      out[k] = byId("set-int-" + k).value.split("\n").map(function (s) { return s.trim(); }).filter(function (s) { return s; });
    });
    return out;
  }
  function intDirty() { return !!INT.loaded && !!byId("set-int-strong") && JSON.stringify(intRead()) !== JSON.stringify(INT.loaded); }
  function intLock(on) {
    INT_KEYS.forEach(function (k) { byId("set-int-" + k).disabled = on; });
    byId("set-int-save").disabled = on || !INT.editable;
    byId("set-int-cancel").disabled = on || !INT.editable;
  }
  function intFill(p) {
    if (!byId("set-int-strong")) { return; }
    INT.loaded = {};
    INT.started = false;
    INT.editable = p.interests_editable !== false;
    INT_KEYS.forEach(function (k) {
      INT.loaded[k] = (p[k] || []).slice();
      var ta = byId("set-int-" + k);
      ta.value = INT.loaded[k].join("\n");
      ta.readOnly = !INT.editable;
    });
    byId("set-int-save").disabled = !INT.editable;
    byId("set-int-cancel").disabled = !INT.editable;
    byId("set-int-reason").hidden = INT.editable;
  }
  function loadInterests() {
    if (intDirty()) { return Promise.resolve(); }
    var seq = SEQ;
    return KnowluData.call("profile", {}).then(function (env) {
      if (!live(seq)) { return; }
      if (env && env.ok && env.profile && !intDirty()) { intFill(env.profile); }
    });
  }
  function saveInterests() {
    var held = INT.loaded;
    var sent = intRead();
    say(byId("set-int-say"), "");
    intLock(true);
    KnowluData.call("set_interests", { strong: sent.strong, mild: sent.mild, never: sent.never, clubs: sent.clubs }).then(function (env) {
      if (!env || !env.ok) { intLock(false); say(byId("set-int-say"), KnowluKit.refusal(env), true); return; }
      ev("edit_committed", null, "interests");
      return KnowluData.call("profile", {}).then(function (p) {
        intLock(false);
        if (!p || !p.ok || !p.profile) { return; }
        intFill(p.profile);
        offerIntUndo(held);
      });
    });
  }
  function offerIntUndo(before) {
    undoToast("Saved", "Undo", function () {
      KnowluData.call("set_interests", { strong: before.strong, mild: before.mild, never: before.never, clubs: before.clubs }).then(function (r) {
        if (r && r.ok) {
          ev("edit_committed", null, "interests");
          KnowluData.call("profile", {}).then(function (p) { if (p && p.ok && p.profile) { intFill(p.profile); } });
          return;
        }
        KnowluKit.toast(KnowluKit.refusal(r), null, null, true);
      });
    });
  }
  function bindInterests(root) {
    INT_KEYS.forEach(function (k) {
      root.querySelector("#set-int-" + k).addEventListener("input", function () {
        var first = !INT.started;
        INT.started = true;
        if (first) {
          ev("edit_started", null, "interests");
        }
      });
    });
    byId("set-int-save").addEventListener("click", saveInterests);
    byId("set-int-cancel").addEventListener("click", function () {
      if (intDirty()) {
        ev("edit_cancelled", null, "interests");
      }
      if (INT.loaded) { intFill({ strong: INT.loaded.strong, mild: INT.loaded.mild, never: INT.loaded.never, clubs: INT.loaded.clubs, interests_editable: INT.editable }); }
    });
  }

  /* ---- Appearance: the three preferences, at once, kept by prefs.js ---- */
  function bindAppearance(root) {
    root.querySelector('[data-section="appearance"]').addEventListener("change", function (e) {
      var t = e.target;
      if (!t || t.type !== "radio" || !t.checked) { return; }
      if (t.name === "st-theme") { KnowluPrefs.setTheme(t.value); }
      if (t.name === "st-font") { KnowluPrefs.setFont(t.value); }
      if (t.name === "st-text") { KnowluPrefs.setText(t.value); }
    });
  }

  /* ---- Grades: Hide grades, saved through set_settings like the other rows; its state is grades_status's ---- */
  function bindGrades(root) {
    var sw = root.querySelector("#set-grades-hide-in");
    sw.addEventListener("click", function () {
      var was = sw.getAttribute("aria-checked") === "true";
      var on = !was;
      sw.setAttribute("aria-checked", on ? "true" : "false");
      sw.disabled = true;
      KnowluData.call("set_settings", { patch: { grades_hidden: on } }).then(function (r) {
        sw.disabled = false;
        if (!r || !r.ok) {
          sw.setAttribute("aria-checked", was ? "true" : "false");
          KnowluKit.toast(KnowluKit.refusal(r), null, null, true);
          return;
        }
        KnowluData.call("grades_status", {}).then(drawGrades);
      });
    });
  }

  /* ---- Delete my data: two presses, because the second deletes a folder of somebody's work and their account with
     it. Free while the privacy update is owed: stopping never waits for consent. ---- */
  function mailDeleteSentence() {
    var s = MAIL;
    return !s || !s.ok || (s.mailboxes || []).length ? " " + EM_FORWARD_SENTENCE : "";
  }
  function askDelete() {
    byId("set-delete-2").hidden = false;
    say(byId("set-delete-note"), "This deletes your vault, your backups, your account and everything we hold. It cannot be undone." + mailDeleteSentence());
  }
  function deleteAll() {
    var b = byId("set-delete-2");
    b.disabled = true;
    KnowluData.call("delete_my_data", {}).then(function (r) {
      var ok = !!(r && r.ok);
      say(byId("set-delete-note"), ok ? "Deleted. Knowlu will close." : KnowluKit.refusal(r), !ok);
      b.disabled = ok;
    });
  }

  /* ---- the rest of the page's presses ---- */
  function onClick(e) {
    var t = e.target;
    if (!t || !t.closest) { return; }
    if (t.closest("#set-portal")) {
      say(byId("set-member-note"), "");
      KnowluData.call("open_portal", {}).then(function (r) { if (!r || !r.ok) { say(byId("set-member-note"), KnowluKit.refusal(r), true); } });
      return;
    }
    if (t.closest("#set-name-save")) {
      KnowluData.call("set_profile_name", { name: byId("set-name-in").value }).then(function (r) {
        say(byId("set-name-note"), r && r.ok ? "Saved" : KnowluKit.refusal(r), !(r && r.ok));
      });
      return;
    }
    if (t.closest("#set-vault-copy")) {
      KnowluData.call("copy_text", { text: (CONTEXT && CONTEXT.vault) || "" }).then(function (r) {
        KnowluKit.toast(r && r.ok ? "Copied" : KnowluKit.refusal(r), null, null, !(r && r.ok));
      });
      return;
    }
    /* back to the picker: the window goes with it, this process is the one relaunching */
    if (t.closest("#set-switch")) { KnowluData.call("switch_profile", {}); return; }
    if (t.closest("#set-bdir-pick")) {
      KnowluData.call("pick_folder", { title: "Choose a backup folder" }).then(function (p) {
        if (!p || !p.path) { return; }
        return KnowluData.call("set_settings", { patch: { backup_dir: p.path } }).then(function (r) {
          if (r && r.ok) { drawSettings(r.settings); } else { KnowluKit.toast(KnowluKit.refusal(r), null, null, true); }
        });
      });
      return;
    }
    if (t.closest("#set-backup-now")) {
      var bk = byId("set-backup-now");
      bk.disabled = true;
      KnowluData.call("backup_now", {}).then(function (r) {
        bk.disabled = false;
        KnowluKit.toast(r && r.ok ? "Backed up" : KnowluKit.refusal(r), null, null, !(r && r.ok));
      });
      return;
    }
    if (t.closest("#set-autostart-in")) {
      var sw = byId("set-autostart-in");
      var on = sw.getAttribute("aria-checked") !== "true";
      KnowluData.call("set_settings", { patch: { autostart: on } }).then(function (r) {
        if (r && r.ok) { drawSettings(r.settings); } else { say(byId("set-diag-note"), KnowluKit.refusal(r), true); }
      });
      return;
    }
    if (t.closest("#set-diag-copy")) {
      KnowluData.call("copy_diagnostics", {}).then(function (r) { say(byId("set-diag-note"), r && r.ok ? "Copied" : KnowluKit.refusal(r), !(r && r.ok)); });
      return;
    }
    var pol = t.closest("[data-policy]");
    if (pol && ROOT.contains(pol) && pol.closest("#set-policy")) { KnowluData.call("open_policy", { which: pol.getAttribute("data-policy") }); return; }
    if (t.closest("#set-delete-1")) { askDelete(); return; }
    if (t.closest("#set-delete-2")) { deleteAll(); return; }
  }

  /* ---- the carried sections ---- */
  var API = {
    account: function () { return ACCOUNT; },
    mail: function () { return MAIL; },
    context: function () { return CONTEXT; },
    redraw: function () { redraw(); }
  };
  function mountSlots(root) {
    MOUNTED = [];
    SLOTS.forEach(function (name) {
      var mod = KnowluRouter.section(name);
      var host = root.querySelector('[data-slot="' + name + '"]');
      if (!mod || !host || typeof mod.mount !== "function") { return; }
      try { mod.mount(host, API); MOUNTED.push(mod); } catch (e) { window.console.error(e); }
    });
  }
  function redraw() {
    MOUNTED.forEach(function (mod) {
      if (typeof mod.redraw === "function") {
        try { mod.redraw(); } catch (e) { window.console.error(e); }
      }
    });
  }
  function goSection(name) {
    var list = ROOT ? ROOT.querySelectorAll("[data-section]") : [];
    for (var i = 0; i < list.length; i++) {
      if (list[i].getAttribute("data-section") !== name) { continue; }
      list[i].scrollIntoView({ block: "start" });
      var head = list[i].querySelector(".st-h");
      if (head) { head.focus({ preventScroll: true }); }
      return;
    }
  }

  function mount(center, ctx) {
    SEQ += 1;
    var seq = SEQ;
    PREFS = { loaded: "", started: false };
    INT = { loaded: null, started: false, editable: true };
    center.innerHTML = pageHtml();
    ROOT = center.querySelector(".st-page");
    ROOT.addEventListener("click", onClick);
    bindAppearance(ROOT);
    bindGrades(ROOT);
    bindPrefs(ROOT);
    bindInterests(ROOT);
    drawGrades(KnowluData.last("grades_status"));
    mountSlots(ROOT);
    KnowluData.call("account_status", {}).then(function (a) { if (live(seq)) { drawAccount(a); } });
    KnowluData.call("get_settings", {}).then(function (r) { if (live(seq) && r && r.ok) { drawSettings(r.settings); } });
    KnowluData.call("settings_context", {}).then(function (c) { if (live(seq)) { drawContext(c); } });
    KnowluData.call("grades_status", {}).then(function (g) { if (live(seq)) { drawGrades(g); } });
    KnowluData.call("mail_status", {}).then(function (s) { if (live(seq)) { drawMail(s); } });
    loadPrefs();
    loadInterests();
    if (ctx && ctx.arg) {
      var want = ctx.arg;
      requestAnimationFrame(function () { if (live(seq)) { goSection(want); } });
    }
  }
  function update(st) {
    MOUNTED.forEach(function (mod) {
      if (typeof mod.update === "function") {
        try { mod.update(st); } catch (e) { window.console.error(e); }
      }
    });
  }
  function unmount() {
    SEQ += 1;
    MOUNTED.forEach(function (mod) {
      if (typeof mod.unmount === "function") {
        try { mod.unmount(); } catch (e) { window.console.error(e); }
      }
    });
    MOUNTED = [];
    ROOT = null;
  }

  KnowluRouter.add("settings", { mount: mount, update: update, unmount: unmount });
  window.KnowluSettings = { redraw: redraw };
})();
