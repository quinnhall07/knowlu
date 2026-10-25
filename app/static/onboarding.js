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

  // The one Tauri call site. Without a backend (the headless shots) `?fixture=` names a file, the same
  // dev-only path the console's own page takes. It answers four commands, each from its own key:
  // {"onboarding_status": ..., "account_status": ..., "your_week": ..., "commitment_proposals": ...};
  // a file with no `onboarding_status` key is read as the status itself. Nothing else is answered.
  function invoke(cmd, args) {
    if (!(window.__TAURI__ && window.__TAURI__.core)) {
      var fx = new URLSearchParams(location.search).get("fixture");
      if (fx && (cmd === "onboarding_status" || cmd === "account_status" || cmd === "your_week" || cmd === "commitment_proposals")) {
        return fetch(fx).then(function (r) { return r.json(); }).then(function (d) {
          if (d && d[cmd] !== undefined) { return d[cmd]; }
          if (cmd === "onboarding_status" && d && d.account_status === undefined && d.your_week === undefined && d.commitment_proposals === undefined) { return d; }
          throw new Error("no fixture for " + cmd);
        });
      }
      return Promise.reject(new Error("no engine: __TAURI__ is absent"));
    }
    return window.__TAURI__.core.invoke(cmd, args || {});
  }

  // The day board: the page starts in Windows' light or dark setting and follows it. Nothing is stored.
  var LIGHT = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: light)") : null;
  function applyTheme() {
    ["setup", "onboarding", "picker"].forEach(function (id) { if (EL(id)) { EL(id).setAttribute("data-theme", LIGHT && LIGHT.matches ? "light" : "dark"); } });
  }
  applyTheme();
  if (LIGHT && LIGHT.addEventListener) { LIGHT.addEventListener("change", applyTheme); }

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
  // The rail's done lines for the account and payment rows, from `account_status` (this computer only).
  var MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  function dayWords(iso) {
    var t = Date.parse(iso);
    return isFinite(t) ? MONTHS[new Date(t).getMonth()] + " " + new Date(t).getDate() : "";
  }
  function accountLine(id, a) {
    if (!(a && a.ok === true)) { return ""; }
    if (id === "account") { return a.email ? "Signed in as " + a.email : ""; }
    var day = a.current_period_end ? dayWords(a.current_period_end) : "";
    if (a.status === "active" && a.plan === "tester") { return "Nothing to pay."; }
    if (a.status === "trialing") { return day ? "Trial on. First charge " + day + "." : ""; }
    if (a.status === "active") { return day ? "Paid. Renews " + day + "." : ""; }
    return "";
  }
  // The foot's estimate: the time each remaining step takes, in seconds.
  // Each is the middle of its range in the spec (1.3), summed and rounded up to a whole minute.
  // Step 7 takes the top of its range so that it still counts after rounding.
  // This is a display estimate, never a count of jobs. Step 9 is absent.
  var STEP_SECONDS = { 1: 30, 2: 45, 3: 120, 4: 90, 5: 30, 6: 30, 7: 60, 8: 60, 10: 120, 11: 30 };
  function minutesLeft(step) {
    var n = 0;
    Object.keys(STEP_SECONDS).forEach(function (k) { if (Number(k) >= Number(step)) { n += STEP_SECONDS[k]; } });
    return Math.max(1, Math.ceil(n / 60));
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
    if (past && (r.id === "account" || r.id === "payment")) { m.pf = accountLine(r.id, s && s.account); }
    return m;
  }
  function railHtml(s, account) {
    var view = {};
    Object.keys(s || {}).forEach(function (k) { view[k] = s[k]; });
    view.account = account || null;
    return ROWS.map(function (r) {
      var m = rowModel(r, view);
      var retry = m.retry ? ' <button class="su-link" type="button" data-retry="' + esc(m.retry) + '">Try again</button>' : "";
      return '<li class="su-row ' + m.cls + '" data-rail="' + m.id + '"><span class="su-shape" aria-hidden="true"></span><span class="su-nm">' + esc(m.nm) +
        '</span><span class="su-st">' + esc(m.st) + '</span><span class="su-pf">' + esc(m.pf) + retry + '</span></li>';
    }).join("");
  }

  // ---- the week strip (F11p2) ----------------------------------------------------------------------
  // Counts only, each one a `step3.due_*` field as given; the page sums nothing. Past days (before
  // `week.today`) show dimmed with no count. The label follows `week.of` with no date arithmetic.
  var DAYS = [["mon", "Mon"], ["tue", "Tue"], ["wed", "Wed"], ["thu", "Thu"], ["fri", "Fri"], ["sat", "Sat"], ["sun", "Sun"]];
  var MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  var STRIP_STEPS = [3, 5, 6, 7, 8];
  function weekStripModel(status, step) {
    var w = status && status.week, s3 = status && status.step3;
    if (STRIP_STEPS.indexOf(Number(step)) === -1 || !w || typeof w !== "object" || !s3 || typeof s3 !== "object") { return null; }
    var keys = DAYS.map(function (d) { return "due_" + d[0]; }).concat(["due_later"]);
    if (keys.every(function (k) { return s3[k] === undefined || s3[k] === null; })) { return null; }
    var parts = String(w.of || "").split("-"), month = MONTHS[Number(parts[1]) - 1], today = Number(w.today) || 0;
    return {
      label: month ? "Week of " + month + " " + Number(parts[2]) : "This week",
      days: DAYS.map(function (d, i) { return { name: d[1], past: i < today, count: i < today ? null : (s3["due_" + d[0]] === undefined ? null : s3["due_" + d[0]]) }; }),
      later: s3.due_later === undefined ? null : s3.due_later
    };
  }
  function weekStripHtml(status, step) {
    var m = weekStripModel(status, step);
    if (!m) { return ""; }
    var cell = function (name, count, past) {
      return '<li class="ob-day' + (past ? " ob-past" : "") + '"><span class="ob-dn">' + esc(name) + '</span>' + (count === null ? "" : '<span class="ob-dc">' + esc(String(count)) + '</span>') + '</li>';
    };
    return '<div class="ob-week-head"><strong>Your week</strong> <span class="su-dim">' + esc(m.label) + '</span></div><ul class="ob-days">' +
      m.days.map(function (d) { return cell(d.name, d.count, d.past); }).join("") + cell("Later", m.later, false) + '</ul>';
  }
  function renderWeek() {
    var el = EL("ob-week");
    if (!el) { return; }
    var html = weekStripHtml(S.status, S.step);
    el.innerHTML = html; el.hidden = !html;
  }
  OB.weekStripModel = weekStripModel; OB.weekStripHtml = weekStripHtml;

  // ---- the frame -----------------------------------------------------------------------------------
  var TITLES = { 2: "Your account", 3: "Your school", 4: "Payment", 5: "School email", 6: "Google: Gmail and Calendar", 7: "Another calendar", 8: "Homework sites", 10: "Your week", 11: "Here's your week" };
  var WHY = {
    2: "Sign in again so this plan is yours.", 3: "Knowlu reads your classes and assignments from your school.", 4: "Payment comes after you have seen what Knowlu found.",
    5: "Teachers announce date changes by email.", 6: "Gmail and your calendar hold what your school does not.", 7: "Add the calendar Knowlu should treat as busy time.",
    8: "Homework sites hold due dates your school does not.", 10: "Check your class times so the week fits around them.", 11: "Everything has run once. Here is your week."
  };
  var S = { account: null, status: null, step: 0, fixed: null, outstanding: false, lastLine: "", error: "", pollError: "", mounted: false };
  var panels = {}, leaveHooks = [];
  OB.registerPanel = function (n, def) { panels[n] = def; };
  OB.registerLeave = function (fn) { leaveHooks.push(fn); };
  OB.clearError = function () { S.error = ""; };
  OB.show = function (n) {
    var from = S.step;
    S.error = ""; S.step = n; renderPane(true); renderRail();
    // The account and payment lines are read again when the step moves past 2 or past 4, never on the poll.
    if (from <= 2 && n > 2) { loadAccount(); } else if (from <= 4 && n > 4) { loadAccount(); }
  };
  OB.refresh = function () { renderPane(true); renderRail(); };

  function frameHtml() {
    // The top bar holds Sign out only; Finish later is in the rail's foot.
    var bar = '<div class="ob-bar"><button class="su-link" id="ob-sign-out" type="button">Sign out</button></div>';
    return '<div class="su-rail" role="group" aria-label="Setup progress">' +
      '<div class="su-rail-head"><span class="su-logo" role="img" aria-label="Knowlu"><span aria-hidden="true">Know</span><span class="su-lu" aria-hidden="true">lu</span></span><span class="su-dim">Setup</span></div>' +
      '<div class="su-rail-title"><h2>Your sources</h2><span class="su-dim" id="ob-stepno"></span></div>' +
      '<ol class="su-rail-list" id="ob-rail" aria-live="polite"></ol>' +
      '<div class="su-rail-foot"><span class="su-dim" id="ob-left"></span><button class="su-link" id="ob-finish-later" type="button">Finish later</button></div></div>' +
      '<div class="su-main ob-main">' + bar +
      '<p class="ob-closing" id="ob-closing" role="status" hidden></p>' +
      '<section class="su-step ob-step" id="ob-pane" aria-labelledby="ob-title"><h1 id="ob-title" tabindex="-1"></h1>' +
      '<p class="su-why" id="ob-why"></p><div id="ob-ask"></div><div class="ob-proof" id="ob-proof"></div></section>' +
      '<div class="ob-week" id="ob-week" aria-label="Your week" hidden></div></div>';
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
    renderWeek();
  }
  function renderRail() {
    if (!S.mounted) { return; }
    EL("ob-rail").innerHTML = railHtml(S.status, S.account);
    var left = minutesLeft(Number(S.status && S.status.step) || S.step);
    EL("ob-left").textContent = "About " + left + (left === 1 ? " minute left" : " minutes left");
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

  // ==== F11c, step 8: homework sites (begins) ===================================================
  // Logins go to Credential Manager through `store_credentials` and the fields are emptied the moment
  // it answers; a confirmed row goes to `onboarding_answer` as the course's key, never a made-up slug.
  var discovering = false;
  var SITES = ["zybooks", "vhl"];
  var SKIP_ID = { zybooks: "ob-skip-8-zybooks", vhl: "ob-skip-8-vhl" };
  var w8 = { status: null, cur: "zybooks", proceeding: false, site: { zybooks: { stage: "login", rows: [], line: "" }, vhl: { stage: "login", rows: [], line: "" } } };

  // Sites with a `homework_sites` entry first, then the rest, zyBooks before VHL (even `unmatched` alone is evidence).
  function hasEntry(status, site) { var h = status && status.homework_sites; return !!(h && Array.isArray(h[site]) && h[site].length); }
  function siteOrder(status) {
    return SITES.filter(function (s) { return hasEntry(status, s); }).concat(SITES.filter(function (s) { return !hasEntry(status, s); }));
  }
  // The word `unmatched` is the engine's, never the student's: it is left out of the codes.
  function foundLine(status, site) {
    if (!hasEntry(status, site)) { return ""; }
    var codes = status.homework_sites[site].filter(function (c) { return String(c).toLowerCase() !== "unmatched"; });
    return "Found at your school: " + (codes.length ? "assignments in " + codes.join(", ") : "some assignments at your school") + " link to " + SITE_NAMES[site] + ".";
  }
  OB.siteOrder = siteOrder; OB.foundLine = foundLine;

  function clearCredentialFields(site) {
    var u = site === "vhl" ? EL("ob-vhl-username") : EL("ob-zybooks-username");
    var p = site === "vhl" ? EL("ob-vhl-password") : EL("ob-zybooks-password");
    if (u) { u.value = ""; }
    if (p) { p.value = ""; }
  }
  function loginHtml(site) {
    var z = site === "zybooks", name = SITE_NAMES[site];
    var user = z ? '<input id="ob-zybooks-username" type="text" autocomplete="off" spellcheck="false">' : '<input id="ob-vhl-username" type="text" autocomplete="off" spellcheck="false">';
    var pass = z ? '<input id="ob-zybooks-password" type="password" autocomplete="off">' : '<input id="ob-vhl-password" type="password" autocomplete="off">';
    return '<div class="su-field"><label>Email or username</label>' + user + '</div><div class="su-field"><label>Password</label>' + pass + '</div>' +
      '<p class="su-privacy">Stored only on this computer, in Windows Credential Manager, and used only to read ' + esc(name) + '.</p>' +
      '<p class="su-small">If you skip ' + esc(name) + ", its due dates are missing unless your school's calendar feed carries them.</p>" +
      '<div class="su-actions"><button class="b pri y su-btn" type="button" data-act8="signin" data-site="' + site + '"' + (discovering ? " disabled" : "") + '>Sign in to ' + esc(name) + '</button>' +
      '<button class="b su-btn" type="button" id="' + SKIP_ID[site] + '" data-act8="skip" data-site="' + site + '"' + (discovering ? " disabled" : "") + '>Skip ' + esc(name) + '</button></div>';
  }
  function rowHtml(site, x) {
    var head = '<div class="ob-row8" data-key="' + esc(x.key) + '"><span class="ob-key">' + esc(x.key) + (x.detail ? ' <span class="su-dim">' + esc(x.detail) + '</span>' : "") + '</span> ';
    if (x.mapped || x.ignored) { return head + '<span class="ob-row-line">' + (x.ignored ? "Knowlu leaves this one out." : "Already set.") + '</span></div>'; }
    var opts = ((w8.status && w8.status.courses) || []).map(function (c) {
      return '<option value="' + esc(c.key) + '"' + (x.suggested && c.code === x.suggested ? " selected" : "") + '>' + esc(c.label || c.code) + '</option>';
    }).join("");
    return head + '<select class="ob-course" aria-label="Class for ' + esc(x.key) + '"><option value="">Choose a class</option>' + opts + '</select> ' +
      '<button class="b su-btn" type="button" data-act8="confirm" data-site="' + site + '" data-key="' + esc(x.key) + '">Confirm</button>' +
      (site === "zybooks" ? ' <button class="su-link ob-not-a-class" type="button" data-act8="ignore" data-site="' + site + '" data-key="' + esc(x.key) + '">Not a class</button>' : "") +
      ' <span class="ob-row-line" role="status">' + esc(x.saved || "") + '</span></div>';
  }
  function siteHtml(site) {
    var f = w8.site[site], name = SITE_NAMES[site], note = '<p class="ob-state ob-failed" role="alert" id="ob-8-note-' + site + '"' + (f.line ? "" : " hidden") + '>' + esc(f.line) + '</p>';
    if (f.stage === "skipped") { return '<h2 class="ob-h2">' + esc(name) + '</h2><p class="ob-state" data-state="skipped">' + esc(costWords(HOMEWORK_COST + site)) + '</p>'; }
    if (f.stage === "done") { return '<h2 class="ob-h2">' + esc(name) + '</h2><p class="ob-state">' + esc(name) + ' is set up.</p>'; }
    if (f.stage === "storing" || f.stage === "reading") { return '<h2 class="ob-h2">' + esc(name) + '</h2><p class="ob-state" data-state="reading">Reading ' + esc(name) + '… This can take a minute.</p>'; }
    if (f.stage === "rows") {
      return '<h2 class="ob-h2">' + esc(name) + '</h2><p class="su-small">Choose the class each one belongs to. Leave one blank and Knowlu asks again later.</p>' + note +
        f.rows.map(function (x) { return rowHtml(site, x); }).join("") +
        '<div class="su-actions"><button class="b su-btn" type="button" data-act8="done" data-site="' + site + '">That\'s all for ' + esc(name) + '</button></div>';
    }
    var found = foundLine(w8.status, site);
    return '<div class="ob-form"><p class="ob-form-tag">Knowlu\'s form</p><h2 class="ob-h2">' + esc(name) + ' sign-in</h2>' +
      (found ? '<p class="ob-found">' + esc(found) + '</p>' : "") + note + loginHtml(site) + '</div>';
  }
  function say8(site, text) {
    var el = EL("ob-8-note-" + site);
    if (el) { el.textContent = text; el.hidden = !text; }
  }
  function paintSite(site) {
    var host = EL("ob-8-" + site);
    if (host) { host.innerHTML = siteHtml(site); }
  }
  function settled8(site) { return w8.site[site].stage === "done" || w8.site[site].stage === "skipped"; }
  // One site's form shows at a time; the other stays a "Show {site}" link away.
  function paintView8() {
    SITES.forEach(function (s) { var h = EL("ob-8-" + s); if (h) { h.hidden = s !== w8.cur; } });
    var host = EL("ob-8-show");
    if (host) {
      host.innerHTML = SITES.filter(function (s) { return s !== w8.cur; }).map(function (s) {
        return '<button class="su-link" type="button" data-act8="show" data-site="' + s + '">Show ' + esc(SITE_NAMES[s]) + '</button>';
      }).join(" ");
    }
  }
  function advance8(site) {
    if (w8.cur !== site) { return; }
    var next = siteOrder(w8.status).filter(function (s) { return !settled8(s); })[0];
    if (next) { w8.cur = next; }
    paintView8();
  }
  function paintNext() {
    var next = EL("ob-next-8");
    if (next) { next.disabled = discovering || !SITES.every(settled8) || w8.proceeding; }
    SITES.forEach(function (s) { var b = EL(SKIP_ID[s]); if (b) { b.disabled = discovering; } });
    document.querySelectorAll("#ob-step-8 [data-act8=\"signin\"]").forEach(function (b) { b.disabled = discovering; });
  }
  // ==== F11c, step 8 (continues) =================================================================
  function storeLogins(site) {
    var f = w8.site[site], z = site === "zybooks";
    if (discovering) { return; }
    var user = (z ? EL("ob-zybooks-username") : EL("ob-vhl-username")).value;
    var secret = (z ? EL("ob-zybooks-password") : EL("ob-vhl-password")).value;
    if (!user || !secret) { say8(site, "Type your " + SITE_NAMES[site] + " email or username and password first."); return; }
    OB.clearError();
    f.stage = "storing"; f.line = "";
    invoke("onboarding_store_credentials", { source: site, user: user, secret: secret }).then(function (a) {
      clearCredentialFields(site);
      if (a && a.ok) { discoverSite(site); return; }
      f.stage = "login"; f.line = (a && a.error) || "Knowlu couldn't save your sign-in. Try again."; paintSite(site);
    }).catch(function () {
      clearCredentialFields(site);
      f.stage = "login"; f.line = "Knowlu couldn't save your sign-in. Try again."; paintSite(site);
    });
    paintSite(site);
  }
  function discoverSite(site) {
    var f = w8.site[site];
    discovering = true; f.stage = "reading"; f.line = ""; paintSite(site); paintNext();
    invoke("onboarding_discover_coursework", { zybooks: site === "zybooks", vhl: site === "vhl" }).then(function (r) {
      f.rows = ((r && r.rows) || []).filter(function (x) { return x.source === site; });
      f.line = (r && r.note) || "";
      f.stage = f.rows.length ? "rows" : "login";
      if (!f.rows.length) { f.line = f.line || "Knowlu found nothing on " + SITE_NAMES[site] + ". Check your sign-in and try again."; }
    }).catch(function () {
      f.stage = "login"; f.line = "Knowlu couldn't reach " + SITE_NAMES[site] + ". Try again in a moment.";
    }).then(function () { discovering = false; paintSite(site); paintNext(); });
  }
  function answerRow(b) {
    var site = b.getAttribute("data-site"), key = b.getAttribute("data-key"), row = b.closest(".ob-row8");
    var line = row.querySelector(".ob-row-line"), sel = row.querySelector(".ob-course");
    var ignore = b.classList.contains("ob-not-a-class");
    if (!ignore && !(sel && sel.value)) { line.textContent = "Choose a class first, or leave this one blank."; return; }
    OB.clearError();
    var answer = ignore ? { zybooks_ignore: key } : { row: { source: site, key: key, course_slug: sel.value } };
    invoke("onboarding_answer", { step: 8, answer: answer }).then(function (a) {
      line.textContent = a && a.ok ? (ignore ? "Left out." : "Saved.") : ((a && a.error) || "Knowlu couldn't save that. Try again.");
    }).catch(function () { line.textContent = "Knowlu couldn't save that. Try again."; });
  }
  function skipSite(site) {
    if (discovering) { return; }
    OB.clearError();
    invoke("onboarding_skip", { step: 8, source: site }).then(function (a) {
      var f = w8.site[site];
      if (a && a.ok === false) { f.line = a.error || "Knowlu couldn't skip that. Try again."; paintSite(site); return; }
      f.stage = "skipped"; f.line = ""; paintSite(site); paintNext(); advance8(site);
      if (SITES.every(settled8)) { leaveStep8(); }
    }).catch(function () { w8.site[site].line = "Knowlu couldn't skip that. Try again."; paintSite(site); });
  }
  // Leaving step 8 is one answer, sent after any skip calls (plan 14.52.3): it starts the jobs after it.
  function leaveStep8() {
    if (w8.proceeding || discovering) { return; }
    w8.proceeding = true; paintNext(); OB.clearError();
    var say = function (t) { var el = EL("ob-8-line"); if (el) { el.textContent = t; el.hidden = !t; } };
    invoke("onboarding_answer", { step: 8, answer: { proceed: true } }).then(function (a) {
      w8.proceeding = false; paintNext();
      if (a && a.ok === true) { OB.show(10); return; }
      say((a && a.error) || "Knowlu couldn't move on. Try again.");
    }).catch(function () { w8.proceeding = false; paintNext(); say("Knowlu couldn't move on. Try again."); });
  }
  function onClick8(e) {
    var b = e.target.closest("[data-act8]");
    if (!b || !EL("ob-step-8")) { return; }
    var act = b.getAttribute("data-act8"), site = b.getAttribute("data-site");
    if (act === "signin") { storeLogins(site); }
    else if (act === "skip") { skipSite(site); }
    else if (act === "done") { w8.site[site].stage = "done"; paintSite(site); paintNext(); advance8(site); }
    else if (act === "show") { w8.cur = site; paintView8(); }
    else if (act === "next") { leaveStep8(); }
    else if (act === "confirm" || act === "ignore") { answerRow(b); }
  }
  function render8(status) {
    w8.status = status; w8.proceeding = false;
    SITES.forEach(function (s) {
      var skipped = ((status && status.skips) || []).some(function (k) { return k.step === 8 && k.source === s; });
      if (skipped) { w8.site[s].stage = "skipped"; }
    });
    var order = siteOrder(status);
    w8.cur = order.filter(function (s) { return !settled8(s); })[0] || order[0];
    return { ask: '<div id="ob-step-8">' + order.map(function (s) {
      return '<div class="ob-site" id="ob-8-' + s + '"' + (s === w8.cur ? "" : " hidden") + '>' + siteHtml(s) + '</div>';
    }).join("") + '<p class="ob-show8" id="ob-8-show">' + SITES.filter(function (s) { return s !== w8.cur; }).map(function (s) {
      return '<button class="su-link" type="button" data-act8="show" data-site="' + s + '">Show ' + esc(SITE_NAMES[s]) + '</button>';
    }).join(" ") + '</p>' +
      '<p class="ob-state ob-failed" id="ob-8-line" role="alert" hidden></p>' +
      '<div class="su-actions"><button class="b pri y su-btn" type="button" id="ob-next-8" data-act8="next"' + (discovering || !SITES.every(settled8) ? " disabled" : "") + '>Next</button></div></div>' };
  }
  // A poll updates nothing on this panel: a re-draw would wipe what the student is typing.
  function onStatus8(status) { w8.status = status; }
  OB.registerPanel(8, { render: render8, onStatus: onStatus8 });
  OB.registerLeave(function () { clearCredentialFields("zybooks"); clearCredentialFields("vhl"); });
  if (EL("onboarding")) { EL("onboarding").addEventListener("click", onClick8); }
  // ==== F11c, step 8 (ends) ======================================================================

  // ==== F11c, step 10: your week (begins) ========================================================
  // Class times show as set (read from `your_week`); the calendar's meetings are asked for only once
  // the status says `proposals_ready`. Confirming is today's `commitments_confirm`, which can refuse
  // by name while a sync runs: the sentence shows, with a Try again.
  var w10 = { asked: false, busy: false, reg: "" };
  var KINDS10 = [["class", "Class"], ["lab", "Lab"], ["work", "Work"], ["club", "Club"], ["meeting", "Meeting"], ["office-hours", "Office hours"]];
  var LEVELS10 = [["hard", "Must keep"], ["soft", "Usually"], ["optional", "Optional"]];
  function say10(text) {
    var el = EL("ob-10-line"), retry = EL("ob-step10-retry");
    if (el) { el.textContent = text; el.hidden = !text; }
    if (retry) { retry.hidden = !text; }
  }
  function setConfirm10() {
    var b = EL("ob-10-confirm");
    if (b) { b.disabled = w10.busy; }
  }
  function loadSet10() {
    invoke("your_week", {}).then(function (r) {
      var host = EL("ob-10-set");
      if (!host) { return; }
      var w = r && r.ok && r.week ? r.week : null;
      var rows = ((w && w.commitments) || []).map(function (c) {
        return editRowHtml(c);
      }).join("") + ((w && w.uncovered_courses) || []).map(function (u) {
        return '<li><span class="a">' + esc(u.title) + '</span> <span class="su-dim">no class times found. Knowlu will ask this week.</span></li>';
      }).join("");
      host.innerHTML = rows ? '<ul class="ob-list">' + rows + '</ul>' : '<p class="ob-state">Knowlu has no class times to show yet. It will ask in the coming days.</p>';
    }).catch(function () {
      var host = EL("ob-10-set");
      if (host) { host.innerHTML = '<p class="ob-state">Knowlu could not read your class times just now.</p>'; }
    });
  }
  function editRowHtml(c) {
    // A kind with no label here (event, exam, task-block) stays selected, under the engine's own word.
    var known = KINDS10.some(function (k) { return k[0] === c.kind; });
    var kinds = (known ? "" : '<option value="' + esc(c.kind || "") + '" selected>' + esc(c.kind || "") + '</option>') + KINDS10.map(function (k) { return '<option value="' + k[0] + '"' + (c.kind === k[0] ? " selected" : "") + '>' + k[1] + '</option>'; }).join("");
    var levels = LEVELS10.map(function (l) { return '<option value="' + l[0] + '"' + ((c.level || "soft") === l[0] ? " selected" : "") + '>' + l[1] + '</option>'; }).join("");
    return '<li class="ob-set" data-id="' + esc(c.id) + '" data-kind="' + esc(c.kind) + '" data-level="' + esc(c.level || "soft") + '"><span class="a">' + esc(c.title) + '</span> <span class="su-dim">' + esc(c.when || "") + '</span> ' +
      '<button class="su-link" type="button" data-act10="edit">Edit</button>' +
      '<span class="ob-edit" hidden> <select class="ob-kind" aria-label="What it is">' + kinds + '</select> <select class="ob-level" aria-label="How firm it is">' + levels + '</select> ' +
      '<button class="b su-btn" type="button" data-act10="save">Save</button> <span class="ob-row-line" role="status"></span></span></li>';
  }
  // Edit saves through the console's own set_fields (a kind and a level; no command edits a class's hours).
  function saveEdit10(row) {
    var line = row.querySelector(".ob-row-line"), kind = row.querySelector(".ob-kind").value, level = row.querySelector(".ob-level").value;
    var fields = {};
    if (kind !== row.getAttribute("data-kind")) { fields.kind = kind; }
    if (level !== row.getAttribute("data-level")) { fields.level = level; }
    if (!Object.keys(fields).length) { line.textContent = "Nothing changed."; return; }
    OB.clearError();
    invoke("set_fields", { view: "today", id: row.getAttribute("data-id"), fields: fields }).then(function (env) {
      if (env && env.ok === false) { line.textContent = env.error || "Knowlu couldn't save that. Try again."; return; }
      row.setAttribute("data-kind", kind); row.setAttribute("data-level", level);
      line.textContent = "Saved.";
    }).catch(function () { line.textContent = "Knowlu couldn't save that. Try again."; });
  }
  function propHtml(p) {
    var lv = ["hard", "soft", "optional"].map(function (l) { return '<option value="' + l + '"' + ((p.level || "soft") === l ? " selected" : "") + '>' + l.charAt(0).toUpperCase() + l.slice(1) + '</option>'; }).join("");
    return '<div class="ob-prop" data-key="' + esc(p.source_uid) + '" data-answer=""><span class="a">' + esc(p.title) + '</span> <span class="su-dim">' + esc(p.when || "") + (p.where ? " · " + esc(p.where) : "") + '</span> ' +
      '<button class="b su-btn" type="button" data-act10="mine" aria-pressed="false">Mine</button> <button class="b su-btn" type="button" data-act10="not" aria-pressed="false">Not mine</button> ' +
      '<select class="ob-level" aria-label="How firm is ' + esc(p.title) + '">' + lv + '</select></div>';
  }
  function askProposals(s) {
    if (!(s && s.proposals_ready) || w10.asked) { return; }
    w10.asked = true;
    invoke("commitment_proposals", {}).then(function (r) {
      var host = EL("ob-10-prop");
      if (!host) { return; }
      var ps = ((r && r.proposals) || []).filter(function (p) { return !p.window; });
      host.innerHTML = ps.length ? '<p class="su-small">Knowlu found these repeating on your calendar. Mark each one, or leave it blank and Knowlu asks again later.</p>' + ps.map(propHtml).join("")
        : '<p class="ob-state">' + esc((r && r.error) || "Knowlu found nothing repeating on your calendar.") + '</p>';
    }).catch(function () {
      var host = EL("ob-10-prop");
      if (host) { host.innerHTML = '<p class="ob-state">Knowlu could not read your calendars just now. Your class times can still be confirmed.</p>'; }
    }).then(setConfirm10);
  }
  // ==== F11c, step 10 (continues) ================================================================
  function confirm10() {
    if (w10.busy) { return; }
    var mine = [], notMine = [];
    document.querySelectorAll("#ob-10-prop .ob-prop").forEach(function (r) {
      var key = r.getAttribute("data-key"), a = r.getAttribute("data-answer"), lv = r.querySelector(".ob-level");
      if (a === "mine") { mine.push({ source_uid: key, level: lv ? lv.value : "soft" }); }
      else if (a === "not") { notMine.push(key); }
    });
    w10.busy = true; say10(""); setConfirm10(); OB.clearError();
    invoke("commitments_confirm", { view: "today", confirm: { mine: mine, not_mine: notMine } }).then(function (env) {
      w10.busy = false; setConfirm10();
      if (!env || env.ok !== true) { say10("Knowlu couldn't save your week: " + ((env && env.error) || "something went wrong") + "."); return; }
      w10.busy = true; setConfirm10();
      return invoke("onboarding_answer", { step: 10, answer: { confirmed: true } }).then(function (a) {
        w10.busy = false; setConfirm10();
        if (a && a.ok === true) { OB.show(11); return; }
        say10((a && a.error) || "Knowlu couldn't move on. Try again.");
      }, function () { w10.busy = false; setConfirm10(); say10("Your week is saved, but Knowlu couldn't move on. Try again."); });
    }).catch(function () { w10.busy = false; setConfirm10(); say10("Knowlu couldn't save your week. Try again."); });
  }
  function onClick10(e) {
    var b = e.target.closest("[data-act10]");
    if (!b || !EL("ob-step-10")) { return; }
    var act = b.getAttribute("data-act10"), row = b.closest(".ob-prop");
    if (act === "confirm") { confirm10(); return; }
    if (act === "edit" || act === "save") {
      var set = b.closest(".ob-set");
      if (act === "save") { saveEdit10(set); } else { set.querySelector(".ob-edit").hidden = !set.querySelector(".ob-edit").hidden; }
      return;
    }
    if (!row) { return; }
    // A second press on the pressed button puts the row back to unanswered.
    var next = row.getAttribute("data-answer") === act ? "" : act;
    row.setAttribute("data-answer", next);
    row.querySelectorAll("[data-act10]").forEach(function (x) { x.setAttribute("aria-pressed", String(x.getAttribute("data-act10") === next)); });
  }
  function render10(status) {
    w10.asked = false; w10.busy = false;
    w10.reg = (status && status.jobs && status.jobs["registrar.confirm"] && status.jobs["registrar.confirm"].state) || "";
    Promise.resolve().then(function () { loadSet10(); askProposals(status); });
    return { ask: '<div id="ob-step-10"><h2 class="ob-h2">Class times</h2><div id="ob-10-set"><p class="ob-state">Reading your class times…</p></div>' +
      '<h2 class="ob-h2">Meetings on your calendar</h2><div id="ob-10-prop"><p class="ob-state" id="ob-10-wait">Knowlu is reading your calendars…</p></div>' +
      '<p class="ob-state ob-failed" id="ob-10-line" role="alert" hidden></p>' +
      '<div class="su-actions"><button class="b pri y su-btn" type="button" id="ob-10-confirm" data-act10="confirm">Confirm my week</button>' +
      '<button class="b su-btn" type="button" id="ob-step10-retry" data-act10="confirm" hidden>Try again</button></div></div>' };
  }
  // A poll fills this panel's own nodes in place; it never redraws the pane.
  function onStatus10(status) {
    askProposals(status);
    var rec = status && status.jobs && status.jobs["config.ingest"], wait = EL("ob-10-wait");
    if (rec && rec.state === "failed" && wait && !w10.asked) {
      wait.innerHTML = esc(reasonWords(rec.reason)) + ' <button class="su-link" type="button" data-retry="config.ingest">Try again</button>';
    }
    // The class times appear when the registrar's step finishes, so they are read again then.
    var reg = status && status.jobs && status.jobs["registrar.confirm"];
    if (reg && reg.state === "done" && w10.reg !== "done") { w10.reg = "done"; loadSet10(); }
  }
  OB.registerPanel(10, { render: render10, onStatus: onStatus10 });
  if (EL("onboarding")) { EL("onboarding").addEventListener("click", onClick10); }
  // ==== F11c, step 10 (ends) =====================================================================

  // The one reader of `account_status`: on mount and at the step 2 and step 4 moves, never from the poll.
  function loadAccount() {
    invoke("account_status", {}).then(function (a) { S.account = a && a.ok === true ? a : null; renderRail(); })
      .catch(function () { S.account = null; renderRail(); });
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
      renderWeek(); renderRail();
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
    loadAccount();
    if (!S.tick) { S.tick = setInterval(renderClosing, 1000); }
    load().catch(function () {}).then(function () { if (!S.fixed) { setTimeout(poll, 2000); } });
  }

  OB.exitLine = exitLine; OB.elapsedText = elapsedText; OB.reasonWords = reasonWords; OB.costWords = costWords;
  OB.openKnowluEnabled = openKnowluEnabled; OB.isClosing = isClosing; OB.revealModel = revealModel; OB.railRow = railRow;
  OB.rowModel = rowModel; OB.rows = ROWS; OB.minutesLeft = minutesLeft;
  OB.finishLater = finishLater; OB.signOut = signOut; OB.renderRestart = renderRestart;

  // The headless shots open the frame at a step against a staged status (never a live one).
  window.KNOWLU_SHOTS = window.KNOWLU_SHOTS || {};
  window.KNOWLU_SHOTS.startOnboarding = startOnboarding;

  invoke("launch_state", {}).then(function (l) { if (l && l.mode === "onboarding") { startOnboarding(); } }).catch(function () {});
})();
