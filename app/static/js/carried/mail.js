/* Knowlu app: the mail sheet (note §5 "Mail setup on first open", §11 item 4; plan §5 T17; email spec §4.5, §4.14).
   One sheet, KnowluSheets "mail" (N5: it waits behind sign-in, upgrade and week setup), with two views:
     progress           what the email setup is doing: one row per step with its state, the backfill's count and time
                        left, a guided step's address with Copy and the provider's path, the one question it may ask,
                        and what an ended setup did. Polled through mail_setup_progress every 1.5 s while it runs.
                        Leave closes the sheet and never stops the setup; Stop does (mail_setup_stop).
     From your email    what forwarded mail added, by course (mail_applied, the engine's read-only list): items
                        created, fields changed, past items archived, notices opened, each with Undo through the
                        console's own writers, or "changed since" when the student has written to it since
   Opening either view while the sheet is open switches it in place, so two sheets never stack.

   window.KnowluMail.openProgress()   opens the progress view (Settings > Sources > School email, Rotate)
   window.KnowluMail.openApplied()    opens From your email (Home's What changed, Settings, the progress view)

   First open (D6, M10): when the wizard's announcement was taken and no setup has begun (mail_status offer_pending and
   not setup_running), the school setup starts at launch, on the console path only (KnowluBoot.onConsole), once the
   launch account check has settled (KnowluAccount.after, account.js, however long the privacy read takes): account.js
   asks account_status and, for a signed-in vault, privacy_status, through KnowluData.call; this file reads their answers
   with KnowluData.last and asks neither. While the privacy update is owed it starts nothing (the offer stays for the next
   launch), and if the check did not answer it starts nothing the same way.

   The sheet's surface names its heading (aria-labelledby); the shell copies that onto #sheet, the one dialog, and a view
   switch points #sheet at the view's own heading.

   Every string here is a fixed sentence of this page's or the engine's, or mail text through h(); nothing here sends a
   usage event. */
(function () {
  "use strict";
  var MP = { host: null, view: "progress", back: false, timer: null, last: null };

  function byId(id) { return document.getElementById(id); }

  var SHEET_HTML = '<div class="sheet-in wide mail-sheet" aria-labelledby="mp-h">' +
    '<section class="mail-view" id="mail-progress">' +
    '<div class="mail-hd"><h2 class="mail-h" id="mp-h" tabindex="-1">Connecting your email</h2><button class="obtn" type="button" id="mp-leave">Leave</button></div>' +
    '<p class="mail-lede" id="mp-sentence" role="status"></p>' +
    '<ol class="mp-rows" id="mp-rows"></ol>' +
    '<p class="mail-meta" id="mp-backfill"></p>' +
    '<div class="mp-ask" id="mp-ask"></div>' +
    '<p class="mail-meta" id="mp-end"></p>' +
    '<div class="mail-acts"><button class="tbtn" type="button" id="mp-applied">See what came from your email</button><button class="qbtn" type="button" id="mp-stop">Stop setup</button></div>' +
    "</section>" +
    '<section class="mail-view" id="mail-applied" hidden>' +
    '<div class="mail-hd"><h2 class="mail-h" id="ma-h" tabindex="-1">From your email</h2><button class="obtn" type="button" id="ma-close">Close</button></div>' +
    '<div class="ma-body" id="ma-body"></div>' +
    "</section></div>";

  /* ---- mail progress view ---- */
  var MP_STEP = { sign_in: "Sign in", step_aside: "Step aside", forwarding: "Open forwarding", add_address: "Add Knowlu’s address",
    confirm_address: "Confirm the address", turn_on: "Turn forwarding on", canary: "Test message", backfill: "Bring in recent mail", finish: "Finish" };
  var MP_STATE = { waiting: "waiting", working: "working", done: "done", needs_you: "needs you", done_by_hand: "done by hand", skipped: "skipped", failed: "failed" };
  var MP_REASON = { guided: "Do this one yourself, then say you have.", forwarding_elsewhere: "This mailbox already forwards somewhere else.",
    not_found: "Knowlu could not find the setting on the page.", script_failed: "That step did not work.",
    blocked: "The test message did not arrive. Your school may be blocking forwarding." };
  /* A step's own words for a reason, read before MP_REASON: sign-in's not_found is the mailbox address the page never
     showed, not a setting. */
  var MP_STEP_REASON = { sign_in: { not_found: "Knowlu could not read your mailbox’s address from the page. Start setup again.",
    script_failed: "Signing in did not finish. Start setup again.",
    already_connected: "This mailbox is already set up in Knowlu. Open Settings, then Email, to resume it or turn it off there.",
    refused: "Knowlu’s service did not accept this mailbox, for the reason given below." } };
  var MP_MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  /* A guided step's directions (spec §4.4): the provider's own path to the setting, four lines at most, keyed on the
     provider and the step, never on a school. The address comes from mail_status by the setup's opaque id. */
  var MP_PATH = {
    m365: ["Open Outlook on the web and choose Settings.", "Go to Mail, then Forwarding.", "Turn forwarding on and paste the address above.", "Save, then come back here."],
    google: ["Open Gmail and choose Settings, then See all settings.", "Open Forwarding and POP/IMAP.", "Choose Add a forwarding address and paste the address above.", "Confirm it, turn forwarding on, then come back here."],
    other: ["Open your mail provider’s settings.", "Find Forwarding (sometimes under Mail or Rules).", "Forward a copy of every message to the address above.", "Save, then come back here."]
  };
  var MP_PATH_CONFIRM = ["Open the confirmation message your provider sent.", "Follow its link or enter its code.", "Then come back here."];
  var MPA = { id: null, address: null, busy: false };

  function mpDate(iso) {
    var m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(iso || ""));
    return m ? Number(m[3]) + " " + MP_MONTHS[Number(m[2]) - 1] : String(iso || "");
  }
  function mpLeft(secs) {
    if (secs == null) { return ""; }
    var m = Math.max(1, Math.round(secs / 60));
    return m >= 60 ? " · about " + Math.round(m / 60) + " h left" : " · about " + m + " min left";
  }
  function stepName(s) { return MP_STEP[s] || s; }
  /* Every answer but "Keep it as it is" (stop) moves the setup on, so it waits while the privacy update is owed. */
  function mpBtnHtml(label, choice, cls) {
    var free = choice === "stop";
    var btnHtml = '<button class="' + h(cls) + '" type="button" data-mp-choice="' + h(choice) + '"';
    return (free ? btnHtml + ">" + h(label) + "</button>" : KnowluKit.owedHtml(btnHtml + ' data-start' + KnowluKit.owed() + ">" + h(label) + "</button>"));
  }
  function needsYou(x) { return x.state === "needs_you" || x.state === "failed"; }
  function mpGuided(p) {
    return p.rows.some(function (x) { return x.state === "needs_you" || x.state === "failed" || (x.guided && x.state === "working"); });
  }
  function mpAddressFor(p) {
    var want = p.address_id;
    if (!want || MPA.busy || MPA.id === want) { return; }
    MPA.busy = true;
    KnowluData.call("mail_status", {}).then(function (s) {
      var m = ((s && s.mailboxes) || []).filter(function (x) { return x["address_id"] === want; })[0];
      MPA.id = want; MPA.address = m ? m.address || null : null; MPA.busy = false;
      if (MP.last) { drawProgress({ progress: MP.last }); }
    });
  }
  function mpGuideHtml(p) {
    if (p.status === "ended" || !mpGuided(p)) { return ""; }
    var row = p.rows.filter(function (x) { return x.state === "needs_you" || x.state === "failed" || x.guided; })[0] || {};
    /* The held Microsoft 365 backfill: forwarding is already set and verified, so no address and no path. */
    if (row.step === "backfill" && p.provider === "m365") {
      return '<div class="mp-guide"><p>' + h("Knowlu can’t bring in Outlook’s older mail yet. New mail is already arriving — press I’ve done this to finish.") + "</p></div>";
    }
    mpAddressFor(p);
    var lines = row.step === "confirm_address" ? MP_PATH_CONFIRM : (MP_PATH[p.provider] || MP_PATH.other);
    var addrHtml = (MPA.id === p.address_id && MPA.address ? '<p class="mp-addr"><code>' + h(MPA.address) + '</code><button class="tbtn" type="button" data-mp-copy>Copy</button></p>' : "");
    return '<div class="mp-guide">' + addrHtml + '<ol class="mp-path">' + lines.map(function (l) { return "<li>" + h(l) + "</li>"; }).join("") + "</ol></div>";
  }
  function drawProgress(r) {
    if (!MP.host || !byId("mp-rows")) { return; }
    var p = r && r.progress;
    MP.last = p || null;
    if (!p) {
      byId("mp-sentence").textContent = "No email setup is running.";
      byId("mp-rows").innerHTML = "";
      byId("mp-backfill").textContent = "";
      byId("mp-ask").innerHTML = "";
      byId("mp-end").textContent = "";
      byId("mp-stop").hidden = true;
      return;
    }
    byId("mp-sentence").textContent = p.sentence || "";
    var rowsHtml = "";
    p.rows.forEach(function (row) {
      var said = row.reason && ((MP_STEP_REASON[row.step] || {})[row.reason] || MP_REASON[row.reason]);
      var whyHtml = (said ? '<span class="mp-why">' + h(said) + "</span>" : "");
      rowsHtml += '<li class="mp-row mp-' + h(row.state) + '"><span class="mp-step">' + h(stepName(row.step)) + '</span><span class="mp-state">' + h(MP_STATE[row.state] || row.state) + "</span>" + whyHtml + "</li>";
    });
    byId("mp-rows").innerHTML = rowsHtml;
    var b = p.backfill;
    byId("mp-backfill").textContent = b ? "Bringing in mail from the last 120 days, since " + mpDate(b.since) + ": " + b.handled + " of " + b.expected + mpLeft(b.eta_secs) + (b.paused ? " (paused)" : "") : "";
    var askHtml = mpGuideHtml(p);
    if (p.ask === "school_address") {
      askHtml = '<div class="mp-q"><label class="mail-meta" for="mp-addr">Your school email address</label><div class="mp-line"><input class="field" type="email" id="mp-addr" autocomplete="email" spellcheck="false">' + mpBtnHtml("Continue", "school_address", "btn small") + "</div></div>";
    } else if (p.ask === "forwarding_elsewhere") {
      askHtml = '<div class="mail-acts">' + mpBtnHtml("Forward to Knowlu instead", "replace", "tbtn") + mpBtnHtml("Keep it as it is", "stop", "qbtn") + "</div>";
    } else if (p.status !== "ended" && p.rows.some(needsYou)) {
      askHtml += '<div class="mail-acts">' + mpBtnHtml("I’ve done this", "done", "btn small") + (p.blocked ? mpBtnHtml("Try again", "retry", "qbtn") : "") + "</div>";
    }
    if (p.blocked && (p.offers || []).indexOf("personal_gmail") >= 0) {
      askHtml += '<div class="mail-acts">' + KnowluKit.owedHtml('<button class="tbtn" type="button" id="mp-gmail" data-start' + KnowluKit.owed() + ">Connect personal Gmail instead</button>") + "</div>";
    }
    byId("mp-ask").innerHTML = askHtml;
    var end = "";
    if (p.status === "ended") {
      end = p.line && p.line !== p.sentence ? p.line : "";
      /* The row already says it; but a setup that kept its address says so only here (runner.rs KEPT_*_LINE). */
      var kept = p.kept === true;
      if (!kept && p.rows.some(function (x) { return x.reason === "already_connected"; })) { end = ""; }
      if (p.done && p.done.length) { end += " Done: " + p.done.map(stepName).join(", ") + "."; }
      if (p.left && p.left.length) { end += " Not done: " + p.left.map(stepName).join(", ") + "."; }
      if (p.not_brought_in_before) { end += " Mail before " + mpDate(p.not_brought_in_before) + " was not brought in."; }
    }
    byId("mp-end").textContent = end.trim();
    byId("mp-stop").hidden = p.status === "ended";
  }
  function stopPoll() {
    if (MP.timer) { clearInterval(MP.timer); }
    MP.timer = null;
  }
  function pollMailProgress() {
    return KnowluData.call("mail_setup_progress", {}).then(function (r) {
      drawProgress(r);
      if (!r || !r.active) { stopPoll(); }
      return r;
    });
  }
  function startPoll() {
    if (!MP.timer) { MP.timer = setInterval(pollMailProgress, 1500); }
    return pollMailProgress();
  }
  /* Leave closes the sheet; the setup carries on, and Settings > Sources > School email shows it. */
  function mailLeave() {
    stopPoll();
    KnowluSheets.close("mail");
  }
  function copiedAddress(r) {
    if (r && r.ok) { KnowluKit.toast("Copied"); return; }
    KnowluKit.toast(KnowluKit.refusal(r), null, null, true);
  }
  function progressClick(t) {
    if (t.closest("#mp-leave")) { mailLeave(); return; }
    if (t.closest("#mp-stop")) { KnowluData.call("mail_setup_stop", {}).then(pollMailProgress); return; }
    if (t.closest("#mp-gmail")) {
      KnowluData.call("mail_setup_start", { provider: "gmail" }).then(function (r) { if (r && r.ok) { openMailProgress(); } });
      return;
    }
    if (t.closest("[data-mp-copy]")) {
      if (MPA.address) { KnowluData.call("copy_text", { text: MPA.address }).then(copiedAddress); }
      return;
    }
    var c = t.closest("[data-mp-choice]");
    if (!c || c.disabled) { return; }
    var k = c.getAttribute("data-mp-choice"), a;
    if (k === "school_address") { a = KnowluData.call("mail_setup_answer", { choice: { school_address: byId("mp-addr").value.trim() } }); }
    else if (k === "done") { a = KnowluData.call("mail_setup_answer", { choice: "done" }); }
    else if (k === "retry") { a = KnowluData.call("mail_setup_answer", { choice: "retry" }); }
    else if (k === "replace") { a = KnowluData.call("mail_setup_answer", { choice: "replace" }); }
    else { a = KnowluData.call("mail_setup_answer", { choice: "stop" }); }
    a.then(function (r) {
      if (r && r.ok === false) { KnowluKit.toast(KnowluKit.refusal(r), null, null, true); }
      return startPoll();
    });
  }
  /* ---- end mail progress view ---- */

  /* ---- mail applied (email spec §4.14, D10). What forwarded mail added, by course, each row with Undo through the
     console's own writers. Every string from a message goes through h(); a row that changed since shows no Undo. ---- */
  var MA = { rows: [], note: "" };
  var FIELD_WORDS = { due: "Due", title: "Title", status: "Status", effort_hours: "Effort", importance: "Importance", start_by: "Start by" };

  function courseName(slug) {
    if (slug == null) { return "Notices"; }
    var course = KnowluCourses.match(slug);
    return course && course.title ? course.title : String(slug);
  }
  function valueText(field, v) {
    if (v == null || v === "") { return "none"; }
    return field === "due" && KnowluFmt.parse(v) ? KnowluFmt.due(v) : String(v);
  }
  function maRowHtml(x, kind) {
    var i = MA.rows.push({ kind: kind, row: x }) - 1, mainHtml, fromHtml = "";
    if (kind === "changed") {
      mainHtml = h(x.title) + '<span class="ma-what">' + h(FIELD_WORDS[x.field] || x.field) + ": " + h(valueText(x.field, x.old)) + " → " + h(valueText(x.field, x.new)) + "</span>";
      fromHtml = (x.sender ? h(x.sender) : "") + (x.date ? " · " + h(KnowluFmt.day(x.date)) : "");
    } else if (kind === "notice") {
      mainHtml = h(x.title);
    } else {
      mainHtml = h(x.title) + (x.due ? '<span class="ma-what">Due ' + h(KnowluFmt.due(x.due)) + "</span>" : "");
    }
    var endHtml = (x.changed_since ? '<span class="ma-since">changed since</span>' : '<button class="tbtn" type="button" data-ma-undo="' + h(i) + '">Undo</button>');
    return '<li class="ma-row"><div class="ma-main"><p class="ma-t">' + mainHtml + "</p>" + (fromHtml ? '<p class="ma-from">' + fromHtml + "</p>" : "") + "</div>" + endHtml + "</li>";
  }
  function maSectionHtml(title, rows, kind) {
    return (rows && rows.length ? '<h4 class="ma-sh">' + h(title) + '</h4><ul class="ma-rows">' + rows.map(function (x) { return maRowHtml(x, kind); }).join("") + "</ul>" : "");
  }
  function drawApplied(r) {
    var body = byId("ma-body");
    if (!MP.host || !body) { return; }
    MA.rows = [];
    var noteHtml = (MA.note ? '<p class="mail-meta ma-note" role="alert">' + h(MA.note) + "</p>" : "");
    if (!r || r.ok === false || !Array.isArray(r.courses)) {
      body.innerHTML = noteHtml + '<p class="mail-meta">Knowlu could not read what came from your email just now.</p>';
      return;
    }
    if (!r.courses.length) {
      body.innerHTML = noteHtml + '<p class="mail-meta">Nothing has come in from your email yet.</p>';
      return;
    }
    var html = "";
    r.courses.forEach(function (g) {
      var items = g.archived && Array.isArray(g.archived.items) ? g.archived.items : [];
      var archHtml = (items.length ? '<details class="ma-arch"><summary>' + h("Past items archived (" + g.archived.count + ")") + '</summary><ul class="ma-rows">' + items.map(function (x) { return maRowHtml(x, "archived"); }).join("") + "</ul></details>" : "");
      html += '<section class="ma-course"><h3 class="ma-ch">' + h(courseName(g.course)) + "</h3>" + maSectionHtml("Items created", g.created, "created") + maSectionHtml("Fields changed", g.changed, "changed") + archHtml + maSectionHtml("Notices opened", g.notices, "notice") + "</section>";
    });
    body.innerHTML = noteHtml + html;
  }
  function loadMailApplied() {
    return KnowluData.call("mail_applied", {}).then(drawApplied);
  }
  function appliedClick(t) {
    if (t.closest("#ma-close")) {
      if (MP.back) { MP.back = false; MP.view = "progress"; showView("progress"); startPoll(); return; }
      KnowluSheets.close("mail");
      return;
    }
    var b = t.closest("[data-ma-undo]");
    if (!b) { return; }
    var at = MA.rows[Number(b.getAttribute("data-ma-undo"))];
    if (!at) { return; }
    var row = at.row, fields = {}, p;
    b.disabled = true;
    if (at.kind === "changed") { fields[row.field] = row.old; p = KnowluData.call("set_fields", { id: row.id, fields: fields }); }
    else if (at.kind === "notice") { p = KnowluData.call("close_info", { id: row.id }); }
    else { p = KnowluData.call("delete_note", { id: row.id }); }
    p.then(function (env) {
      MA.note = env && env.ok !== false ? "" : "Could not undo that: " + KnowluKit.refusal(env);
      return loadMailApplied();
    });
  }
  /* ---- end mail applied ---- */

  /* ---- the sheet ---- */
  function showView(v) {
    var prog = byId("mail-progress"), app = byId("mail-applied");
    if (!prog || !app) { return; }
    prog.hidden = v !== "progress";
    app.hidden = v !== "applied";
    if (v !== "progress") { stopPoll(); }
    byId("ma-close").textContent = MP.back ? "Back" : "Close";
    var dlg = MP.host.querySelector(".mail-sheet");
    var name = v === "applied" ? "ma-h" : "mp-h";
    if (dlg) { dlg.setAttribute("aria-labelledby", name); }
    MP.host.setAttribute("aria-labelledby", name);
    var head = byId(v === "applied" ? "ma-h" : "mp-h");
    if (head) { head.focus({ preventScroll: true }); }
  }
  function onSheetClick(e) {
    var t = e.target;
    if (!t || !t.closest) { return; }
    if (t.closest("#mp-applied")) { openMailApplied(); return; }
    progressClick(t);
    appliedClick(t);
  }
  KnowluSheets.define("mail", {
    render: function (host) {
      MP.host = host;
      host.innerHTML = SHEET_HTML;
      host.querySelector(".mail-sheet").addEventListener("click", onSheetClick);
      showView(MP.view);
      if (MP.view === "applied") { loadMailApplied(); } else { startPoll(); }
    },
    close: function () {
      stopPoll();
      MP.host = null;
      MP.back = false;
    }
  });
  function openMailProgress() {
    MP.view = "progress";
    MP.back = false;
    if (MP.host) { showView("progress"); return startPoll(); }
    KnowluSheets.request("mail");
    return Promise.resolve();
  }
  function openMailApplied() {
    MA.note = "";
    MP.back = !!MP.host && MP.view === "progress";
    MP.view = "applied";
    if (MP.host) { showView("applied"); return loadMailApplied(); }
    KnowluSheets.request("mail");
    return Promise.resolve();
  }
  window.KnowluMail = { openProgress: openMailProgress, openApplied: openMailApplied };

  /* ---- first open (D6, M10) ---- */
  var FIRST_DONE = false;
  /* Once the launch account check has settled: did it answer? An install with no account, or a signed-in one whose
     privacy answer is in. A check that failed starts nothing (the offer stays for the next launch). */
  function accountAnswered() {
    var a = KnowluData.last("account_status"), p = KnowluData.last("privacy_status");
    return !!(a && a.ok && (a.needs_account || p));
  }
  function mailFirstOpen() {
    if (FIRST_DONE || KnowluKit.isOwed()) { return Promise.resolve(); }
    FIRST_DONE = true;
    return KnowluData.call("mail_status", {}).then(function (s) {
      if (s && s.offer_pending && !s.setup_running) {
        return KnowluData.call("mail_setup_start", { provider: "school" }).then(function (r) { if (r && r.ok) { return openMailProgress(); } });
      }
    });
  }
  KnowluBoot.onConsole(function () {
    if (window.KnowluAccount) { KnowluAccount.after(function () { if (accountAnswered()) { mailFirstOpen(); } }); }
  });
})();
