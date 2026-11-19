/* Knowlu app: Settings > Sources (note §4 "Settings", Sources row; §5 "Mail setup on first open"; plan §5 T17). One
   section the Settings frame hosts (KnowluRouter.addSection("sources", ...), its slot right after Account and
   Membership). Its rows, in order:
     Blackboard     grades_status's four states (not available, not connected, signed out, connected), Connect, Sign in
                    again, Refresh and Forget; when connected, when it was last read (grades_status.fetched_at)
     Google         google_status, google_connect, google_disconnect; for a signed-in student only, read when Settings
                    mounts (the frame's account_status answer reaches account()), never at launch (P4, M5)
     School email   mail_setup_progress's sentence, Show progress, Connect school email, From your email; then one row
                    per mailbox of mail_status, and the saved-sign-in switch
     a line per S.issues_panel.sources entry (a source that needs the student), from the read model
   Every control that starts a source or a setup carries data-start and the update-owed lock (KnowluKit.owed() inside
   KnowluKit.owedHtml); Disconnect, Forget, Turn off email and Cancel stay free. While the update is owed the section's
   first line says why, in words (KnowluKit.OWED_LINE, note §5). Nothing here sends a usage event, and
   every string a mailbox, the service or a note gave goes through h(). The mail sheet is mail.js's (window.KnowluMail). */
(function () {
  "use strict";
  var HOST = null;   /* the section's host while Settings shows */
  var API = null;    /* the frame's api: account(), mail(), context(), redraw() */

  function byId(id) { return document.getElementById(id); }
  function say(id, text, bad) {
    var el = byId(id);
    if (!el) { return; }
    el.textContent = text || "";
    el.classList.toggle("st-bad", !!bad);
  }

  function sectionHtml() {
    return '<section class="st-sec" data-section="sources" aria-labelledby="st-h-sources"><h2 class="st-h" id="st-h-sources" tabindex="-1">Sources</h2>' +
      '<p class="st-hint st-warn" id="set-src-owed" hidden></p>' +
      '<ul class="st-rows">' +
      '<li class="st-row st-src" id="set-grades"><span class="st-logo"><img src="logos/blackboard.svg" alt=""></span><div class="st-l"><p class="st-n">Blackboard</p><div id="set-grades-say"></div><p class="st-hint" id="set-grades-note" role="status"></p></div><div class="st-c" id="set-grades-c"></div></li>' +
      '<li class="st-row st-src" id="set-google" hidden><span class="st-logo"><img src="logos/gmail.svg" alt=""></span><div class="st-l"><p class="st-n">Google</p><p class="st-hint" id="set-google-state" role="status"></p><p class="st-hint" id="set-google-note"></p></div><div class="st-c" id="set-google-c"></div></li>' +
      '<li class="st-row st-src" id="set-email"><span class="st-logo"><svg class="st-mail" viewBox="0 0 20 20" aria-hidden="true"><path d="M2.5 4.5h15v11h-15zM2.5 5l7.5 6 7.5-6"/></svg></span><div class="st-l"><p class="st-n">School email</p><p class="st-hint" id="set-email-state" role="status"></p></div><div class="st-c" id="set-email-c"></div></li>' +
      "</ul>" +
      '<ul class="st-rows em-boxes" id="set-email-boxes"></ul>' +
      '<ul class="st-rows"><li class="st-row" id="set-email-keeprow" hidden>' + keepHtml() + "</li></ul>" +
      '<ul class="st-rows" id="set-src-issues"></ul>' +
      "</section>";
  }

  /* ---- grades row (Blackboard). Which state shows is grades_status's word alone: the page has no address entry and
     never decides whether a school offers grades. A signed-out session is a named state in the page's quiet voice. ---- */
  var GR = { note: "" };
  function gradesUnavailableHtml() {
    return '<p class="st-hint">Grades from Blackboard are not available at your school yet</p>';
  }
  function drawGrades() {
    if (!byId("set-grades")) { return; }
    var s = KnowluData.last("grades_status"), st = KnowluData.state();
    var have = !!(st && Array.isArray(st.grades) && st.grades.length);
    var sayHtml = "", ctlHtml = "", forgetHtml = "";
    if (!s) {
      sayHtml = '<p class="st-hint">Checking Blackboard</p>';
    } else if (s.ok === false) {
      sayHtml = '<p class="st-hint">Knowlu could not read Blackboard just now.</p>';
    } else if (s.available === false) {
      sayHtml = gradesUnavailableHtml();
    } else if (!s.connected) {
      sayHtml = '<p class="st-hint">Connect Blackboard to see your grades</p>';
      ctlHtml = KnowluKit.owedHtml('<button class="tbtn" type="button" data-grades-connect data-start' + KnowluKit.owed() + ">Connect</button>");
    } else if (s.signed_out) {
      sayHtml = '<p class="st-hint">Sign in to Blackboard again to update grades</p>';
      ctlHtml = KnowluKit.owedHtml('<button class="tbtn" type="button" data-grades-connect data-start' + KnowluKit.owed() + ">Sign in again</button>");
    } else {
      sayHtml = (have ? '<p class="st-hint">' + h(KnowluFmt.read(s.fetched_at)) + "</p>" : '<p class="st-hint">Blackboard is connected. Your grades appear after the next refresh.</p>');
      ctlHtml = KnowluKit.owedHtml('<button class="obtn" type="button" data-grades-refresh data-start' + KnowluKit.owed() + ">Refresh</button>");
    }
    forgetHtml = s && s.ok !== false && s.connected ? '<button class="qbtn" type="button" id="set-grades-forget">Forget Blackboard sign-in</button>' : "";
    byId("set-grades-say").innerHTML = sayHtml;
    byId("set-grades-c").innerHTML = ctlHtml + forgetHtml;
    say("set-grades-note", GR.note);
  }
  function loadGrades() {
    return KnowluData.call("grades_status", {}).then(function () { if (HOST) { drawGrades(); } });
  }
  function gradesSay(text) { GR.note = text || ""; drawGrades(); }
  /* A capture's answer is a named outcome, never an error line; the row re-reads its own state after every press, and
     the page polls so the new grade notes paint. */
  function gradesAct(cmd, button) {
    if (button) { button.disabled = true; }
    gradesSay("");
    return KnowluData.call(cmd, {}).then(function (r) {
      GR.note = r && r.ok === false ? String(r.error || "") : "";
      return loadGrades().then(function () { KnowluData.refresh(); });
    });
  }
  function gradesClick(t) {
    var conn = t.closest("[data-grades-connect]");
    if (conn) { gradesAct("grades_connect", conn); return; }
    var ref = t.closest("[data-grades-refresh]");
    if (ref) { gradesAct("grades_refresh", ref); return; }
    if (t.closest("#set-grades-forget")) {
      KnowluData.call("grades_forget", {}).then(function (r) {
        GR.note = r && r.ok === false ? String(r.error || "") : "Forgotten.";
        return loadGrades();
      });
    }
  }
  /* ---- end grades row ---- */

  /* ---- Google row (Gmail connect T9a to T9c, D6, D14). The row keeps no cache: it asks google_status each time
     Settings mounts, for a signed-in student only. SET.googleSeq is bumped on every new load, click and on leaving
     Settings, so a stale answer never repaints the row. Connect and Reconnect open the browser from Rust; the page then
     polls google_status until the consent link's own lifetime is up (ten minutes), starting at 3 s and backing off to
     10 s, then says Google was not heard from and offers Check again, which re-reads the status and mints nothing. ---- */
  var SET = { googleSeq: 0, google: null, googleNote: "", googlePolling: false, googleWaitedOut: false, googleScope: "", confirming: false, leaving: false };
  var GOOGLE_TESTING = "While Google reviews Knowlu, this works only for invited testers, and the connection needs renewing about once a week.";
  var GOOGLE_DISCONNECT_CONFIRM = "This disconnects Google Calendar too — Google keeps them as one permission. Knowlu stops reading both and deletes the proposals from your mail it had not delivered yet. What it already added stays in your vault, and your calendar stops updating.";
  var GOOGLE_DISCLOSURE = "Google also tells Knowlu which Google account you connected, so this row can show it.";
  var GOOGLE_POLLING = "Waiting for Google… finish in your browser.";
  var GOOGLE_LINK_LIFETIME_MS = 600000;
  var GOOGLE_NO_ANSWER = "Didn’t hear back from Google.";
  var GOOGLE_TIMEOUT = "Google did not finish connecting. If Google said Knowlu is not verified, this Google account is not on the tester list yet.";
  var GOOGLE_UNREACHABLE = "Knowlu could not reach Google just now.";

  function googleSentence(g) {
    if (!g) { return "Checking Google…"; }
    if (!g.ok) { return g.error || GOOGLE_UNREACHABLE; }
    var who = g.email ? " (as " + g.email + ")" : "";
    if (g.state === "revoked") { return "Google stopped answering for Knowlu. While Google reviews Knowlu, connections expire after seven days."; }
    if (g.state === "none") { return "Gmail is not connected. Knowlu can read your inbox for things you have to do and propose each one for you to approve."; }
    if (g.state === "quiet" && g.gmail) { return "Gmail is connected" + who + ", but Knowlu has found no mail in it for a week. If this is the wrong account, disconnect it and connect the right one."; }
    if (g.gmail) { return "Gmail is connected" + who + ", read-only. Knowlu proposes what it finds; nothing is added without you."; }
    return "Google Calendar is connected" + who + ". Gmail is not.";
  }
  /* Connect and Reconnect stay live during the ten-minute poll: pressing one again mints a new link. */
  function googleControlsHtml() {
    var g = SET.google, busy = SET.googlePolling;
    var known = !!(g && g.ok), none = known && g.state === "none", revoked = known && g.state === "revoked";
    var live = known && !none && !revoked;
    var lock = SET.leaving || busy;
    var connectHtml = none || (live && !g.gmail) ? KnowluKit.owedHtml('<button class="tbtn" type="button" id="set-google-connect" data-start' + KnowluKit.owed() + (SET.leaving ? " disabled" : "") + ">Connect Gmail</button>") : "";
    var reconnectHtml = revoked ? KnowluKit.owedHtml('<button class="tbtn" type="button" id="set-google-reconnect" data-start' + KnowluKit.owed() + (SET.leaving ? " disabled" : "") + ">Reconnect</button>") : "";
    var retryHtml = (g && !g.ok ? '<button class="obtn" type="button" id="set-google-retry"' + (lock ? " disabled" : "") + ">Try again</button>" : "");
    var checkHtml = (SET.googleWaitedOut ? '<button class="obtn" type="button" id="set-google-check"' + (lock ? " disabled" : "") + ">Check again</button>" : "");
    var offHtml = ((revoked || live) && !SET.confirming ? '<button class="obtn" type="button" id="set-google-disconnect-1"' + (lock ? " disabled" : "") + ">Disconnect Google</button>" : "");
    var confirmHtml = (SET.confirming && (revoked || live) ? '<button class="tbtn st-danger" type="button" id="set-google-disconnect-2"' + (lock ? " disabled" : "") + ">Yes, disconnect</button>" : "");
    return retryHtml + checkHtml + connectHtml + reconnectHtml + offHtml + confirmHtml;
  }
  function drawGoogle() {
    if (!byId("set-google")) { return; }
    var g = SET.google, busy = SET.googlePolling;
    byId("set-google-state").textContent = busy ? GOOGLE_POLLING : (SET.googleWaitedOut ? GOOGLE_NO_ANSWER + " " : "") + googleSentence(g);
    byId("set-google-state").classList.toggle("st-bad", !busy && !!(g && !g.ok));
    say("set-google-note", (SET.confirming ? GOOGLE_DISCONNECT_CONFIRM + " " : "") + (SET.googleNote ? SET.googleNote + " " : "") + GOOGLE_TESTING + " " + GOOGLE_DISCLOSURE);
    byId("set-google-c").innerHTML = googleControlsHtml();
  }
  function loadGoogleRow() {
    var seq = ++SET.googleSeq;
    SET.google = null; SET.googleNote = ""; SET.googlePolling = false; SET.googleWaitedOut = false; SET.confirming = false;
    byId("set-google").hidden = false;
    drawGoogle();
    return KnowluData.call("google_status", {}).then(function (g) {
      if (seq !== SET.googleSeq || !HOST) { return; }
      SET.google = g || { ok: false, error: GOOGLE_UNREACHABLE };
      drawGoogle();
    });
  }
  /* Signed-in students only: an install with no account, or an account_status that failed, hides the row and drops any
     answer still on its way. */
  function gateGoogleRow(a) {
    if (!HOST) { return; }
    if (a && a.ok && !a.needs_account) { loadGoogleRow(); return; }
    SET.googleSeq += 1;
    byId("set-google").hidden = true;
  }
  function googleDone(g, scope) { return !!(g && g.ok && (scope === "reconnect" ? g.state === "active" : g.gmail === true)); }
  async function connectGoogle(scope) {
    var mySeq = ++SET.googleSeq;
    SET.googlePolling = true; SET.googleWaitedOut = false; SET.googleScope = scope; SET.googleNote = ""; SET.confirming = false;
    drawGoogle();
    var started = await KnowluData.call("google_connect", { scope: scope });
    if (SET.googleSeq !== mySeq) { return; }
    if (!started || !started.ok) {
      SET.googlePolling = false; SET.googleNote = KnowluKit.refusal(started);
      drawGoogle();
      return;
    }
    var deadline = Date.now() + GOOGLE_LINK_LIFETIME_MS, delay = 3000;
    while (Date.now() < deadline) {
      await new Promise(function (r) { setTimeout(r, delay); });
      if (SET.googleSeq !== mySeq) { return; }
      delay = Math.min(Math.round(delay * 1.25), 10000);
      var g = await KnowluData.call("google_status", {});
      if (SET.googleSeq !== mySeq) { return; }
      if (googleDone(g, scope)) {
        SET.google = g; SET.googlePolling = false; SET.googleNote = "";
        drawGoogle();
        return;
      }
    }
    var fresh = await KnowluData.call("google_status", {});
    if (SET.googleSeq !== mySeq) { return; }
    SET.google = fresh || { ok: false, error: GOOGLE_UNREACHABLE };
    SET.googlePolling = false; SET.googleNote = GOOGLE_TIMEOUT;
    SET.googleWaitedOut = !googleDone(SET.google, scope);
    drawGoogle();
  }
  /* Check again: one status read, no google_connect, so no new link is minted. */
  async function checkGoogleAgain() {
    var mySeq = ++SET.googleSeq;
    SET.googlePolling = true;
    drawGoogle();
    var g = await KnowluData.call("google_status", {});
    if (SET.googleSeq !== mySeq) { return; }
    SET.googlePolling = false;
    if (g) { SET.google = g; }
    if (googleDone(g, SET.googleScope)) { SET.googleWaitedOut = false; SET.googleNote = ""; }
    drawGoogle();
  }
  /* Step 2 of Disconnect. A refusal keeps the row as it was and says why under it; on ok the row is asked afresh. */
  async function disconnectGoogle() {
    var mySeq = ++SET.googleSeq;
    SET.leaving = true;
    drawGoogle();
    var r = await KnowluData.call("google_disconnect", {});
    if (SET.googleSeq !== mySeq) { return; }
    SET.leaving = false;
    if (r && r.ok) { loadGoogleRow(); return; }
    SET.confirming = false; SET.googleNote = KnowluKit.refusal(r);
    drawGoogle();
  }
  function googleClick(t) {
    if (t.closest("#set-google-connect")) { connectGoogle("gmail"); return; }
    if (t.closest("#set-google-reconnect")) { connectGoogle("reconnect"); return; }
    if (t.closest("#set-google-retry")) { loadGoogleRow(); return; }
    if (t.closest("#set-google-check")) { checkGoogleAgain(); return; }
    if (t.closest("#set-google-disconnect-1")) { SET.confirming = true; drawGoogle(); return; }
    if (t.closest("#set-google-disconnect-2")) { disconnectGoogle(); return; }
  }
  /* ---- end Google row ---- */

  /* ---- mail settings (email spec §4.12, §4.13, D15). The School email row carries the setup's sentence; a mailbox row
     is the service's status joined to what this device keeps. Everything a row shows reaches the DOM through h(). ---- */
  var SETM = { s: null, confirming: null, notes: [], busy: false, progress: null };
  var EM_STATE = { pending: "Setting up.", confirmed: "Confirmed; waiting for the first message.", active: "Forwarding is working.",
    quiet: "No mail has come through for a while.", blocked: "The test message did not arrive. Your school may be blocking forwarding.",
    off: "Forwarding is off or goes to another address.", paused_send: "Forwarding paused — Microsoft refused a send.",
    lapsed: "Paused because your subscription lapsed.", revoked: "Disconnected." };
  var EM_PROVIDER = { m365: "School email", google: "Google mail", other: "School email" };
  var EM_TURN_OFF_CONFIRM = "Knowlu turns off forwarding in this mailbox, stops reading it and deletes what it holds to serve it. What it already added stays in your plan.";
  var EM_STILL = "Knowlu’s forwarding address is still in your Gmail forwarding list. Remove it by hand: in Gmail, open Settings, then See all settings, then Forwarding and POP/IMAP, and remove it there.";

  function keepHtml() {
    return '<div class="st-l"><p class="st-n" id="set-email-keep-l">Keep my email sign-in</p><p class="st-hint">Saved on this computer so Knowlu can check forwarding.</p><p class="st-hint" id="set-email-keep-note" role="status"></p></div>' +
      '<div class="st-c"><button class="switch" type="button" role="switch" id="set-email-keep" aria-checked="true" aria-labelledby="set-email-keep-l"></button></div>';
  }
  function emailControlsHtml(p, active) {
    var openHtml = p ? '<button class="obtn" type="button" id="set-email-open">Show progress</button>' : "";
    var startHtml = active ? "" : KnowluKit.owedHtml('<button class="tbtn" type="button" id="set-email-start" data-start' + KnowluKit.owed() + ">Connect school email</button>");
    var appliedHtml = p || boxCount() ? '<button class="obtn" type="button" id="set-email-applied">From your email</button>' : "";
    return openHtml + startHtml + appliedHtml;
  }
  function boxCount() { var s = SETM.s; return s && s.ok && Array.isArray(s.mailboxes) ? s.mailboxes.length : 0; }
  function drawEmail() {
    if (!byId("set-email")) { return; }
    var r = SETM.progress, p = r && r.progress;
    say("set-email-state", !p ? "Not connected here." : p.status === "ended" ? (p.line || "Setup ended.") : (p.sentence || "Setting up."));
    byId("set-email-c").innerHTML = emailControlsHtml(p, !!(r && r.active));
  }
  function loadEmail() {
    if (!HOST) { return Promise.resolve(); }
    return KnowluData.call("mail_setup_progress", {}).then(function (r) {
      if (!HOST) { return; }
      SETM.progress = r || null;
      drawEmail();
    });
  }
  function emBtnHtml(label, act, id, extra) {
    return '<button class="obtn' + h(extra || "") + '" type="button" data-em-act="' + h(act) + '" data-em-id="' + h(id) + '">' + h(label) + "</button>";
  }
  function emStartHtml(label, act, id) {
    return KnowluKit.owedHtml('<button class="obtn" type="button" data-em-act="' + h(act) + '" data-em-id="' + h(id) + '" data-start' + KnowluKit.owed() + ">" + h(label) + "</button>");
  }
  function emCopyHtml(key, address) {
    return '<p class="st-hint em-addr"><code>' + h(address) + '</code><button class="tbtn" type="button" data-em-copy="' + h(key) + '">Copy</button></p>';
  }
  function mailBoxHtml(m) {
    var id = m.address_id, ask = SETM.confirming === id;
    var m365Backfill = m.open_backfill && m.provider === "m365";
    var resumable = !m.turned_off && ((m.open_backfill && !m365Backfill) || ["off", "quiet", "blocked", "paused_send"].indexOf(m.state) >= 0);
    var line = m.turned_off ? "Turned off in Knowlu." : (EM_STATE[m.state] || "");
    if (m365Backfill && !m.turned_off) { line += " New mail is arriving. Knowlu will bring in older Outlook mail on a later run."; }
    else if (m.open_backfill && !m.turned_off) { line += " Still bringing in recent mail; Resume carries on."; }
    var amberHtml = (m.unbound_reason ? '<p class="st-hint em-amber">' + h(m.unbound_note || "") + " (" + h(m.unbound_since_live) + ")" + (m.suggest_rotate === true && !m.turned_off ? " " + emStartHtml("Rotate", "rotate", id) : "") + "</p>" : "");
    var rotateHtml = m.suggest_rotate !== true && !m.turned_off ? emStartHtml("Rotate", "rotate", id) : "";
    var resumeHtml = resumable ? emStartHtml("Resume", "resume", id) : "";
    var addrHtml = m.address ? emCopyHtml("m:" + id, m.address) : "";
    var confirmHtml = (ask ? '<p class="st-hint em-confirm">' + h(EM_TURN_OFF_CONFIRM) + "</p>" : "");
    var offHtml = (ask ? emBtnHtml("Yes, turn off email", "off2", id, " st-danger") + emBtnHtml("Cancel", "cancel", id) : emBtnHtml("Turn off email", "off1", id));
    return '<li class="st-row em-box"><div class="st-l"><p class="st-n">' + h(EM_PROVIDER[m.provider] || "Email") + '</p><p class="st-hint">' + h(line) + "</p>" + addrHtml + amberHtml + confirmHtml + "</div>" +
      '<div class="st-c">' + resumeHtml + rotateHtml + offHtml + "</div></li>";
  }
  function emNoteHtml(n, i) {
    return '<li class="st-row em-box"><div class="st-l"><p class="st-hint">' + h(n.lines.join(" ")) + "</p>" + (n.address ? emCopyHtml("n:" + i, n.address) : "") + "</div></li>";
  }
  function drawBoxes() {
    var box = byId("set-email-boxes");
    if (!box) { return; }
    var s = SETM.s;
    var rowsHtml = s && s.ok && Array.isArray(s.mailboxes) ? s.mailboxes.map(mailBoxHtml).join("") : "";
    var failHtml = s && !s.ok ? '<li class="st-row em-box"><div class="st-l"><p class="st-hint st-bad">Knowlu could not read your mailboxes just now.</p></div></li>' : "";
    var notesHtml = SETM.notes.map(emNoteHtml).join("");
    box.innerHTML = rowsHtml + failHtml + notesHtml;
    byId("set-email-keeprow").hidden = !boxCount();
    byId("set-email-keep").setAttribute("aria-checked", !s || s.keep_session !== false ? "true" : "false");
  }
  function loadBoxes() {
    return KnowluData.call("mail_status", {}).then(function (s) {
      if (!HOST) { return; }
      SETM.s = s || null;
      drawBoxes();
      drawEmail();
    });
  }
  function emNote(r) {
    var lines = [], still = false;
    (r.steps || []).forEach(function (st) {
      if (st.step === "remove_address" && st.state === "needs_you") { still = true; lines.push(EM_STILL); }
      else if (st.message) { lines.push(st.message); }
    });
    if (!r.ok) { lines.unshift("Email could not be turned off yet" + (r.error ? ": " + r.error + "." : ".") + " Turn off email again to retry."); }
    else if (!r.needs_you) { lines.unshift("Email is turned off for this mailbox. What Knowlu already added stays in your plan."); }
    return { lines: lines, address: r.needs_you || still ? r.address || null : null };
  }
  function copied(r) {
    if (r && r.ok) { KnowluKit.toast("Copied"); return; }
    KnowluKit.toast(KnowluKit.refusal(r), null, null, true);
  }
  function emailClick(t) {
    if (t.closest("#set-email-open")) { window.KnowluMail.openProgress(); return; }
    if (t.closest("#set-email-applied")) { window.KnowluMail.openApplied(); return; }
    if (t.closest("#set-email-start")) {
      KnowluData.call("mail_setup_start", { provider: "school" }).then(function (r) {
        if (r && r.ok) { window.KnowluMail.openProgress(); loadEmail(); return; }
        KnowluKit.toast(KnowluKit.refusal(r), null, null, true);
      });
      return;
    }
    if (t.closest("#set-email-keep")) {
      var on = byId("set-email-keep").getAttribute("aria-checked") !== "true";
      byId("set-email-keep").setAttribute("aria-checked", on ? "true" : "false");
      KnowluData.call("mail_keep_session", { on: on }).then(function (r) {
        say("set-email-keep-note", !r || !r.ok ? KnowluKit.refusal(r) : r.profiles_left ? "Some saved email sign-in could not be deleted yet; Knowlu tries again when it starts." : "", !r || !r.ok);
        return loadBoxes();
      });
      return;
    }
    var cp = t.closest("[data-em-copy]");
    if (cp) {
      var key = cp.getAttribute("data-em-copy"), addr = null;
      if (key.indexOf("n:") === 0) { addr = (SETM.notes[Number(key.slice(2))] || {}).address; }
      else { addr = ((SETM.s && SETM.s.mailboxes) || []).filter(function (m) { return "m:" + m["address_id"] === key; }).map(function (m) { return m.address; })[0]; }
      if (addr) { KnowluData.call("copy_text", { text: addr }).then(copied); }
      return;
    }
    var b = t.closest("[data-em-act]");
    if (!b || SETM.busy) { return; }
    var act = b.getAttribute("data-em-act"), id = b.getAttribute("data-em-id");
    if (act === "off1") { SETM.confirming = id; drawBoxes(); return; }
    if (act === "cancel") { SETM.confirming = null; drawBoxes(); return; }
    SETM.busy = true;
    if (act === "off2") {
      SETM.confirming = null;
      KnowluData.call("mail_disconnect", { addressId: id }).then(function (r) { SETM.busy = false; SETM.notes = [emNote(r || {})]; return loadBoxes(); });
    } else if (act === "resume") {
      KnowluData.call("mail_sign_in", { addressId: id }).then(function (r) {
        SETM.busy = false;
        SETM.notes = r && r.ok ? [] : [{ lines: [(r && r.error) || "Could not sign in to your mail."], address: null }];
        return loadBoxes();
      });
    } else if (act === "rotate") {
      KnowluData.call("mail_rotate", { addressId: id }).then(function (r) {
        SETM.busy = false;
        if (r && r.ok) { SETM.notes = []; window.KnowluMail.openProgress(); loadEmail(); return; }
        SETM.notes = [{ lines: [(r && r.error) || "Could not start a new address."], address: null }];
        drawBoxes();
      });
    } else {
      SETM.busy = false;
    }
  }
  /* ---- end mail settings ---- */

  /* ---- source issues: an open heads-up for a source in trouble (S.issues_panel.sources), its title and the note's
     first line, read from the read model ---- */
  function issueRowHtml(r) {
    return '<li class="st-row st-src" data-id="' + h(r.id) + '" data-kind="info"><span class="st-logo" aria-hidden="true"></span><div class="st-l"><p class="st-n">' + h(r.title) + '</p><p class="st-hint st-bad">' + h(r.text) + "</p></div></li>";
  }
  function drawIssues() {
    var el = byId("set-src-issues");
    if (!el) { return; }
    var st = KnowluData.state(), ip = st && st.issues_panel;
    var list = ip && Array.isArray(ip.sources) ? ip.sources : [];
    el.innerHTML = list.map(issueRowHtml).join("");
  }

  /* note §5, review M5: the lock says why in words, not only in each control's tip */
  function drawOwed() {
    var line = byId("set-src-owed");
    if (!line) { return; }
    line.hidden = !KnowluKit.isOwed();
    line.textContent = KnowluKit.isOwed() ? KnowluKit.OWED_LINE + "." : "";
  }
  function drawAll() {
    if (!HOST) { return; }
    drawOwed();
    drawGrades();
    if (!byId("set-google").hidden) { drawGoogle(); }
    drawEmail();
    drawBoxes();
    drawIssues();
  }
  function onClick(e) {
    var t = e.target;
    if (!t || !t.closest) { return; }
    gradesClick(t);
    googleClick(t);
    emailClick(t);
  }
  function mount(host, api) {
    HOST = host;
    API = api;
    GR.note = "";
    SETM.confirming = null; SETM.notes = []; SETM.busy = false; SETM.progress = null;
    SETM.s = api && typeof api.mail === "function" ? api.mail() : null;
    host.innerHTML = sectionHtml();
    host.addEventListener("click", onClick);
    drawAll();
    loadGrades();
    loadEmail();
  }
  function unmount() {
    SET.googleSeq += 1;
    SET.googlePolling = false; SET.googleWaitedOut = false; SET.confirming = false; SET.leaving = false;
    SETM.confirming = null;
    if (HOST) { HOST.removeEventListener("click", onClick); }
    HOST = null;
    API = null;
  }

  KnowluRouter.addSection("sources", {
    mount: mount,
    update: function () { if (HOST) { drawGrades(); drawIssues(); } },
    account: function (a) { gateGoogleRow(a); },
    mail: function (s) { if (HOST) { SETM.s = s || null; drawBoxes(); drawEmail(); } },
    redraw: function () { drawAll(); loadEmail(); },
    unmount: unmount
  });
})();
