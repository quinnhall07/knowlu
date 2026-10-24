/* Knowlu onboarding: the step frame and the rail (O3 F11a, the 5b "Departures" direction).
   A classic script, loaded after console.js. It shows what `onboarding_status` answers and
   computes nothing: no count, no gate, no job state is derived here. The steps' own content is
   F11b-F11e's; they hang on `KNOWLU_ONBOARDING.registerPanel(n, {render, onStatus})` and `registerLeave`;
   a panel that sends an answer calls `clearError()` to clear the action line.
   Pure helpers (the exit line, the words for a reason or a cost, the rail's rows) hang on
   `window.KNOWLU_ONBOARDING` so the static tests can run them under node. */
(function () {
  "use strict";
  var EL = function (id) { return document.getElementById(id); };
  // No regex literal here: the static pins read this file with a simple comment stripper.
  var esc = function (s) {
    return String(s == null ? "" : s).split("&").join("&amp;").split("<").join("&lt;").split(">").join("&gt;").split("\"").join("&quot;").split("'").join("&#39;");
  };
  var OB = {};
  window.KNOWLU_ONBOARDING = OB;

  // The one Tauri call site. Without a backend (the headless shots) `?fixture=` names a status
  // file, the same dev-only path the console's own page takes; nothing else is answered.
  function invoke(cmd, args) {
    if (!(window.__TAURI__ && window.__TAURI__.core)) {
      var fx = new URLSearchParams(location.search).get("fixture");
      if (fx && cmd === "onboarding_status") { return fetch(fx).then(function (r) { return r.json(); }); }
      return Promise.reject(new Error("no engine: __TAURI__ is absent"));
    }
    return window.__TAURI__.core.invoke(cmd, args || {});
  }

  // ---- the words: a key never reaches the student, only a sentence ------------------------------
  var STATES = ["intro", "asking", "reading", "proof", "failed", "skipped", "already-done"];
  var JOB_WORDS = {
    "school.capture": "reading your school", "school.preview": "counting what it found", "school.store": "saving your calendar link",
    "school.apply": "setting up your school", "school.courses": "adding your classes", "registrar.confirm": "confirming your class times",
    "mail.setup": "connecting your email", "mail.canary": "testing your email forwarding", "mail.first_pass": "reading your first emails",
    "google.gmail_first_read": "reading your Gmail", "calendar.store": "saving your calendar link", "calendar.read": "reading your calendar",
    "google.calendar_read": "reading your calendar", "coursework.discover": "finding your homework sites", "config.ingest": "setting up your sources",
    "slot.partial": "reading your first week", "slot.first_full": "building your week", "first.sync": "your first sync with your account",
    "first.coursework": "your homework sites", "first.ingest": "your calendar and feeds", "first.grades": "your grades",
    "backfill.older_mail": "your older email", "class_times": "your class times", "gmail": "Gmail", "mail": "school email"
  };
  var REASON_WORDS = {
    first_sync_failed: "Knowlu couldn't finish the first sync with your account. It will try again on its next run, or press Try again now.",
    job_crashed: "Knowlu stopped twice while this step was running, so it has paused it. Press Try again to run it once more.",
    grades_read_failed: "Knowlu couldn't read your grades this time. It will try again on its next run, or press Try again now. If it keeps failing, reconnect your school in Settings after setup.",
    courses_not_recorded: "Knowlu couldn't record your classes yet. Press Try again.",
    feeds_not_settled: "Knowlu is still saving your calendar links. Try again in a moment.",
    lms_link_missing: "Knowlu doesn't have your school's calendar link yet. Add it from the school step.",
    calendar_link_missing: "Knowlu doesn't have your calendar link yet. Add it from the calendar step.",
    mail_setup_cancelled: "Email setup was stopped before it finished. Press Try again when you're ready.",
    mail_setup_failed: "Knowlu couldn't finish connecting your school email. Press Try again.",
    mail_not_yet: "Knowlu is still waiting on your school email. It keeps checking.",
    feed_kept_on_device: "Knowlu couldn't save your calendar link to your account yet. It's kept on this computer until then. Press Try again."
  };
  var COST_WORDS = {
    mail_dates_missed: "Skipped: date changes and new work announced by email are missed.",
    gmail_and_calendar_missed: "Skipped: club, job and internship tasks from Gmail are missed; your own events are not counted as busy time.",
    calendar_busy_time_missed: "Skipped: your own events are not counted as busy time.",
    mail_setup_unfinished: "Finish connecting your email in Settings."
  };
  var SITE_NAMES = { zybooks: "zyBooks", vhl: "VHL" };
  var HOMEWORK_COST = "homework_due_dates_missing_";

  function jobWords(job) { return JOB_WORDS[job] || "your setup"; }
  function reasonWords(k) {
    k = String(k == null ? "" : k);
    if (REASON_WORDS[k]) { return REASON_WORDS[k]; }
    // A reason that is already a sentence passes through; an unmapped key never shows raw.
    return k.indexOf(" ") !== -1 ? k : "Knowlu couldn't finish this step. Press Try again.";
  }
  function costWords(k) {
    k = String(k == null ? "" : k);
    if (COST_WORDS[k]) { return COST_WORDS[k]; }
    if (k.indexOf(HOMEWORK_COST) === 0) {
      var site = k.slice(HOMEWORK_COST.length);
      return "Skipped: " + (SITE_NAMES[site] || "that site") + "'s due dates are missing unless your school's feed carries them.";
    }
    return "Skipped.";
  }
  function elapsedText(since, now) {
    var ms = now - Date.parse(since);
    var secs = isFinite(ms) && ms > 0 ? Math.floor(ms / 1000) : 0;
    return Math.floor(secs / 60) + " min " + (secs % 60) + " s";
  }
  // The three drain lines of plan section 4.10, picked from `exiting: {job, long, since}`.
  function exitLine(ex, now) {
    var name = jobWords(ex && ex.job), ms = now - Date.parse(ex && ex.since);
    if (!(ms >= 15000)) { return "Finishing " + name + "…"; }
    if (ex.long) {
      return "Still finishing " + name + ". Saving what Knowlu has read so far. This can take up to 20 minutes, and Knowlu closes by itself when it's done. " + elapsedText(ex.since, now) + " so far.";
    }
    return "Still finishing " + name + ". Knowlu closes by itself within a minute or two, once it's saved.";
  }
  // From the call to its answer the page is closing, whatever `exiting` reads (F8f2's ruling).
  function isClosing(outstanding, exiting) { return !!outstanding || !!exiting; }
  // The reveal's one rule is the server's: `finish_ready` is read, never re-derived.
  function openKnowluEnabled(s) {
    return !!(s && s.ok === true && s.finish_ready === true && s.gate && s.gate.open === true && !s.exiting);
  }
  function revealModel(s) {
    var g = (s && s.gate) || {};
    return {
      failed: (g.failed || []).map(function (f) { return { key: f.key, name: jobWords(f.key), text: reasonWords(f.reason), retry: true }; }),
      stillComing: (g.still_coming || []).map(function (k) { return { key: k, words: jobWords(k) }; }),
      notConnected: (g.not_connected || []).map(function (n) { return { key: n.key, name: jobWords(n.key), cost: costWords(n.cost_key) }; })
    };
  }

  // ---- the rail ----------------------------------------------------------------------------------
  var PROOF_LABELS = {
    classes: ["class", "classes"], class_times: ["class time", "class times"], assignments_upcoming: ["assignment coming up", "assignments coming up"],
    turned_in: ["already turned in", "already turned in"], grades_courses: ["class with grades", "classes with grades"],
    to_review: ["email to review", "emails to review"], gmail_items: ["thing from Gmail", "things from Gmail"],
    events_this_week: ["event this week", "events this week"], books_or_sections: ["book or section", "books or sections"], assignments: ["assignment", "assignments"]
  };
  function proofLine(proof) {
    var out = [];
    Object.keys(proof || {}).forEach(function (k) {
      var n = proof[k], w = PROOF_LABELS[k];
      if (w && typeof n === "number") { out.push(n + " " + (n === 1 ? w[0] : w[1])); }
    });
    return out.length ? out.join(", ") : "Done";
  }
  function railRow(job, rec) {
    var st = (rec && rec.state) || "pending";
    var line = st === "failed" ? reasonWords(rec.reason) : st === "done" ? proofLine(rec.proof) : st === "running" ? "Reading…"
      : st === "held" ? "Starts once your plan is active" : st === "skipped" ? "Skipped" : "Not yet";
    return { job: job, state: st, line: line, retry: st === "failed" };
  }
  // One row per source (the shell's eight rows), each over the jobs that feed it.
  var ROWS = [
    { id: "account", nm: "Your account", at: [2], pf: "", jobs: [], src: [] },
    { id: "school", nm: "Your school", at: [3], pf: "Classes, assignments and grades", jobs: ["school.capture", "school.preview", "school.apply", "school.courses", "registrar.confirm"], src: ["school"] },
    { id: "payment", nm: "Payment", at: [4], pf: "Nothing is charged before you see what Knowlu found", jobs: [], src: [] },
    { id: "email", nm: "School email", at: [5], pf: "Date changes your teachers send", jobs: ["mail.setup", "mail.canary", "mail.first_pass"], src: ["mail", "email"] },
    { id: "google", nm: "Google: Gmail and Calendar", at: [6, 7], pf: "Club and job deadlines, your calendar", jobs: ["google.gmail_first_read", "calendar.store", "calendar.read", "google.calendar_read"], src: ["google", "gmail", "calendar"] },
    { id: "homework", nm: "Homework sites", at: [8], pf: "Due dates from homework sites", jobs: ["coursework.discover"], src: ["zybooks", "vhl", "homework"] },
    { id: "week", nm: "Your week", at: [10], pf: "Check your class times", jobs: ["config.ingest", "slot.partial"], src: [] },
    { id: "reveal", nm: "Here's your week", at: [11], pf: "Opens when everything has run once", jobs: ["slot.first_full"], src: [] }
  ];
  // What one row shows: the first job that needs a look, else a skip's cost, else its proof.
  function rowModel(r, s) {
    var jobs = (s && s.jobs) || {}, step = Number(s && s.step) || 0;
    var here = r.at.indexOf(step) !== -1, past = r.at[0] < step;
    var m = { id: r.id, nm: r.nm, cls: here ? "now" : past ? "done" : "", st: here ? "Now" : past ? "Done" : "", pf: r.pf, retry: "" };
    var recs = r.jobs.filter(function (j) { return jobs[j]; }).map(function (j) { return railRow(j, jobs[j]); });
    var bad = recs.filter(function (x) { return x.state === "failed"; })[0];
    var skip = ((s && s.skips) || []).filter(function (k) { return r.src.indexOf(k.source) !== -1; })[0];
    var run = recs.filter(function (x) { return x.state === "running"; })[0];
    if (bad) { m.cls = "failed"; m.st = "Needs a look"; m.pf = bad.line; m.retry = bad.job; }
    else if (skip) { m.cls = "skipped"; m.st = "Skipped"; m.pf = costWords(skip.cost_key); }
    else if (run) { m.cls = "reading"; m.st = "Reading"; m.pf = run.line; }
    else if (recs.length && recs.every(function (x) { return x.state === "skipped"; })) { m.cls = "skipped"; m.st = "Skipped"; m.pf = "Skipped"; }
    else if (recs.length && recs.every(function (x) { return x.state === "done" || x.state === "skipped"; })) {
      m.cls = "done"; m.st = "Done"; m.pf = recs.filter(function (x) { return x.state === "done"; }).map(function (x) { return x.line; }).join(", ");
    }
    else if (recs.some(function (x) { return x.state === "held"; })) { m.cls = ""; m.st = "Waiting"; m.pf = "Starts once your plan is active"; }
    else if (past && r.jobs.length) { m.cls = ""; m.st = "Not yet"; m.pf = "Not yet"; }
    return m;
  }
  function railHtml(s) {
    return ROWS.map(function (r) {
      var m = rowModel(r, s);
      var retry = m.retry ? ' <button class="su-link" type="button" data-retry="' + esc(m.retry) + '">Try again</button>' : "";
      return '<li class="su-row ' + m.cls + '" data-rail="' + m.id + '"><span class="su-shape" aria-hidden="true"></span><span class="su-nm">' + esc(m.nm) +
        '</span><span class="su-st">' + esc(m.st) + '</span><span class="su-pf">' + esc(m.pf) + retry + '</span></li>';
    }).join("");
  }

  // ---- the frame -----------------------------------------------------------------------------------
  var TITLES = { 2: "Your account", 3: "Your school", 4: "Payment", 5: "School email", 6: "Google: Gmail and Calendar", 7: "Another calendar", 8: "Homework sites", 10: "Your week", 11: "Here's your week" };
  var WHY = {
    2: "Sign in again so this plan is yours.", 3: "Knowlu reads your classes and assignments from your school.", 4: "Payment comes after you have seen what Knowlu found.",
    5: "Teachers announce date changes by email.", 6: "Gmail and your calendar hold what your school does not.", 7: "Add the calendar Knowlu should treat as busy time.",
    8: "Homework sites hold due dates your school does not.", 10: "Check your class times so the week fits around them.", 11: "Everything has run once. Here is your week."
  };
  var S = { status: null, step: 0, fixed: null, outstanding: false, lastLine: "", error: "", pollError: "", mounted: false };
  var panels = {}, leaveHooks = [];
  OB.registerPanel = function (n, def) { panels[n] = def; };
  OB.registerLeave = function (fn) { leaveHooks.push(fn); };
  OB.clearError = function () { S.error = ""; };
  OB.show = function (n) { S.error = ""; S.step = n; renderPane(true); renderRail(); };
  OB.refresh = function () { renderPane(true); renderRail(); };

  function frameHtml() {
    return '<div class="su-rail" role="group" aria-label="Setup progress">' +
      '<div class="su-rail-head"><span class="su-mark" aria-hidden="true"></span><b>Knowlu</b><span class="su-dim">Setup</span></div>' +
      '<div class="su-rail-title"><h2>Your sources</h2><span class="su-dim" id="ob-stepno"></span></div>' +
      '<ol class="su-rail-list" id="ob-rail" aria-live="polite"></ol></div>' +
      '<div class="su-main ob-main"><div class="ob-bar"><button class="su-link" id="ob-finish-later" type="button">Finish later</button>' +
      '<button class="su-link" id="ob-sign-out" type="button">Sign out</button></div>' +
      '<p class="ob-closing" id="ob-closing" role="status" hidden></p>' +
      '<section class="su-step ob-step" id="ob-pane" aria-labelledby="ob-title"><h1 id="ob-title" tabindex="-1"></h1>' +
      '<p class="su-why" id="ob-why"></p><div id="ob-ask"></div><div class="ob-proof" id="ob-proof"></div></section></div>';
  }

  // The step's state block, by section 4.1's seven states. A step's own markup comes from its panel.
  function stateHtml(m) {
    var st = STATES.indexOf(m.state) === -1 ? "intro" : m.state;
    if (st === "reading") { return '<p class="ob-state" data-state="reading">Reading… You can carry on; the rail fills in as it finishes.</p>'; }
    if (st === "asking") { return '<p class="ob-state" data-state="asking">' + esc(m.where || "Look for the window Knowlu just opened.") + '</p>'; }
    if (st === "failed") {
      return '<p class="ob-state ob-failed" data-state="failed" role="alert">' + esc(reasonWords(m.reason)) + '</p>' +
        (m.job ? '<p><button class="b su-btn" type="button" data-retry="' + esc(m.job) + '">Try again</button></p>' : "");
    }
    if (st === "skipped") { return '<p class="ob-state" data-state="skipped">' + esc(costWords(m.cost)) + '</p>'; }
    if (st === "already-done") { return '<p class="ob-state" data-state="already-done">Already set up on this computer.</p>' + (m.proof || ""); }
    return st === "proof" ? (m.proof || "") : "";
  }

  // The pane is rewritten on a step change or an explicit refresh only, never on a poll, so what a
  // student has typed into a step survives the status coming in.
  function renderPane(force) {
    if (!S.mounted || (!force && S.paneStep === S.step)) { return; }
    S.paneStep = S.step;
    var n = S.step, p = panels[n], m = p ? (p.render(S.status) || {}) : {};
    EL("ob-title").textContent = m.title || TITLES[n] || "Setup";
    EL("ob-why").textContent = m.why || WHY[n] || "";
    EL("ob-ask").innerHTML = m.ask || "";
    EL("ob-proof").innerHTML = stateHtml(m);
    EL("ob-pane").setAttribute("data-step", String(n));
    EL("ob-stepno").textContent = "Step " + (n > 9 ? n - 1 : n) + " of 10";
  }
  function renderRail() {
    if (!S.mounted) { return; }
    EL("ob-rail").innerHTML = railHtml(S.status);
    renderClosing();
  }

  // ---- leaving: Finish later, Sign out, the restart screen ---------------------------------------
  function closingText() {
    var ex = S.status && S.status.exiting;
    if (ex) { S.lastLine = exitLine(ex, Date.now()); }
    return S.lastLine || "Finishing…";
  }
  function renderClosing() {
    if (!S.mounted) { return; }
    var closing = isClosing(S.outstanding, S.status && S.status.exiting);
    var el = EL("ob-closing");
    el.hidden = !closing && !(S.error || S.pollError);
    el.textContent = closing ? closingText() : (S.error || S.pollError);
    EL("onboarding").classList.toggle("ob-closing-now", closing);
    // A control re-enables nothing while the page is closing.
    // Only the buttons this frame disabled itself: a step's own disabled button stays as it left it.
    EL("onboarding").querySelectorAll("button").forEach(function (b) {
      if (closing) { if (!b.disabled) { b.disabled = true; b.setAttribute("data-ob-closed", "1"); } }
      else if (b.hasAttribute("data-ob-closed")) { b.disabled = false; b.removeAttribute("data-ob-closed"); }
    });
  }
  function leaving() {
    // A hook that throws, rejects or never answers must not keep the student from leaving.
    var hooks = leaveHooks.map(function (f) { return Promise.resolve().then(f).catch(function () { return null; }); });
    return Promise.race([Promise.all(hooks), new Promise(function (done) { setTimeout(done, 3000); })]);
  }
  function afterExit(a) {
    if (a && a.restart) { renderRestart(a.error); return; }
    if (a && a.ok === false) { S.outstanding = false; S.error = a.error || "Knowlu couldn't close setup."; renderClosing(); }
  }
  function exitFailed() { S.outstanding = false; S.error = "Knowlu couldn't close setup. Try again."; renderClosing(); }
  function finishLater() {
    if (S.outstanding) { return; }
    S.outstanding = true; S.error = ""; renderClosing();
    leaving().then(function () { return invoke("onboarding_quit", {}); }).then(afterExit).catch(exitFailed);
  }
  function signOut() {
    if (S.outstanding) { return; }
    S.outstanding = true; S.error = ""; renderClosing();
    leaving().then(function () { return invoke("onboarding_sign_out", {}); }).then(afterExit).catch(exitFailed);
  }
  // A restart answer: one screen, one button, and it quits.
  function renderRestart(why) {
    S.outstanding = false;
    EL("onboarding").innerHTML = '<div class="su-main ob-main"><section class="su-step ob-step" id="ob-restart" aria-labelledby="ob-restart-h">' +
      '<h1 id="ob-restart-h" tabindex="-1">Setup needs to start again</h1><p class="su-why">' + esc(why || "Knowlu stopped setup part-way, so it needs a fresh start.") +
      '</p><div class="su-actions"><button class="b pri y su-btn" type="button" data-act="finishLater">Close Knowlu</button></div></section></div>';
    S.mounted = false;
  }
  function retryJob(job) {
    invoke("onboarding_retry", { job: job }).then(function (a) {
      if (a && a.ok === false) { S.error = a.error || "Knowlu couldn't retry that. Try again."; renderClosing(); return; }
      return load();
    }).catch(function () {});
  }

  // ---- status, polling, the shots hook ---------------------------------------------------------------
  function load() {
    var ask = S.fixed ? Promise.resolve(S.fixed) : invoke("onboarding_status", {});
    return ask.then(function (s) {
      S.status = s || { ok: false };
      if (!S.mounted) { return; }
      if (S.status.ok === false) { S.pollError = S.status.error || "Knowlu couldn't read your setup."; renderClosing(); return; }
      S.pollError = "";
      if (!S.step) { S.step = Number(S.status.step) || 3; renderPane(); }
      var p = panels[S.step];
      if (p && p.onStatus) { p.onStatus(S.status); }
      renderRail();
    });
  }
  function poll() {
    if (!S.mounted || S.fixed) { return; }
    load().catch(function () {}).then(function () { setTimeout(poll, 2000); });
  }
  function onClick(e) {
    S.error = "";
    if (e.target.closest("#ob-finish-later") || e.target.closest("[data-act=finishLater]")) { finishLater(); return; }
    if (e.target.closest("#ob-sign-out")) { signOut(); return; }
    var r = e.target.closest("[data-retry]");
    if (r) { retryJob(r.getAttribute("data-retry")); }
  }
  function startOnboarding(step, status) {
    S.fixed = status || null; S.step = Number(step) || 0; S.status = status || null; S.mounted = true; S.error = "";
    var root = EL("onboarding");
    root.innerHTML = frameHtml();
    root.hidden = false;
    EL("setup").hidden = true; EL("picker").hidden = true;
    document.querySelector(".app").hidden = true;
    if (!root.getAttribute("data-wired")) { root.addEventListener("click", onClick); root.setAttribute("data-wired", "1"); }
    if (S.step) { renderPane(); }
    renderRail();
    if (!S.tick) { S.tick = setInterval(renderClosing, 1000); }
    load().catch(function () {}).then(function () { if (!S.fixed) { setTimeout(poll, 2000); } });
  }

  OB.exitLine = exitLine; OB.elapsedText = elapsedText; OB.reasonWords = reasonWords; OB.costWords = costWords;
  OB.openKnowluEnabled = openKnowluEnabled; OB.isClosing = isClosing; OB.revealModel = revealModel; OB.railRow = railRow;
  OB.rowModel = rowModel; OB.rows = ROWS;
  OB.finishLater = finishLater; OB.signOut = signOut; OB.renderRestart = renderRestart;

  // The headless shots open the frame at a step against a staged status (never a live one).
  window.KNOWLU_SHOTS = window.KNOWLU_SHOTS || {};
  window.KNOWLU_SHOTS.startOnboarding = startOnboarding;

  invoke("launch_state", {}).then(function (l) { if (l && l.mode === "onboarding") { startOnboarding(); } }).catch(function () {});
})();
