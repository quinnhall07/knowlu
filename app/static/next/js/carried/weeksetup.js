/* Knowlu app: "When do your classes meet?", the first-day sheet (note §5; plan §5 T18; phase 2 spec §2, D1, D3).

   Once the console boots (KnowluBoot.onConsole), and once a launch, it reads `your_week`; while that says `setup`
   (the vault's first day, no planning-day note) it asks KnowluSheets for the `weeksetup` sheet. The sheet reads the
   calendar's proposals (`commitment_proposals`): Your classes (classes and labs, preset to Mine, with the registrar's
   button where the school has one), the rest of the week, office hours, and Your day (the window editor KnowluWeek
   lends, filled from the window proposal unless the student already edited it). Each row is Mine or Not mine, with
   how much it binds on a Mine row; nothing is typed. Finish sends the answers and the window through
   commitments_confirm and says any rows the engine skipped; Not now hides the sheet for the session. Neither runs
   while the calendar read, a registrar fetch or a Finish is in flight (finishMayRun), and Escape does not close it.
   Closing it closes the school's sign-in window. */
(function () {
  "use strict";
  var DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
  var FOUND = "Knowlu found these on your calendar. Mark each one.";
  var NOTHING = "Knowlu found nothing repeating on your calendar. Set your day below.";
  var weekSetup = { asked: false, dismissed: false, open: false, dirty: false, reading: false, fetching: false, finishing: false };
  var registrarLabel = null;
  var WEEK_NOW = null;   /* the your_week block the check read */
  var ROOT = null;       /* the sheet's own element while it shows */
  var API = null;        /* the shell's close for it */
  var SHEET = { render: renderSheet, close: onSheetClosed, escape: false };

  function el(id) { return document.getElementById(id); }

  /* R5 re-review: the one answer to "may Finish be pressed?": nothing else is in flight. */
  function finishMayRun() {
    return !weekSetup.reading && !weekSetup.fetching && !weekSetup.finishing;
  }
  /* a Finish in flight holds the registrar controls too, so no capture starts under it */
  function setupFinishing(on) {
    weekSetup.finishing = on;
    if (!ROOT) { return; }
    el("ws-reg").querySelectorAll("[data-reg-open], [data-reg-done]").forEach(function (b) { b.disabled = on; });
    el("ws-finish").disabled = !finishMayRun();
    el("ws-later").disabled = !finishMayRun();
  }
  /* R5 I1: a registrar fetch holds Finish and Not now until it and the reload settle */
  function setupBusy(on) {
    weekSetup.fetching = on;
    if (!ROOT) { return; }
    el("ws-finish").disabled = !finishMayRun();
    el("ws-later").disabled = !finishMayRun();
  }

  function checkWeekSetup() {
    if (weekSetup.asked || weekSetup.dismissed || weekSetup.open) { return; }
    weekSetup.asked = true;
    KnowluData.call("your_week", {}).then(function (r) {
      if (r && r.ok) { registrarLabel = r.registrar_label || null; KnowluWeek.setLabel(registrarLabel); }
      if (r && r.ok && r.week && r.week.setup && !weekSetup.dismissed) {
        WEEK_NOW = r.week;
        KnowluSheets.request("weeksetup", SHEET);
      }
    });
  }

  function sheetHtml() {
    return '<div class="sheet-in wide ws" aria-labelledby="ws-h">' +
      '<h2 class="ws-h" id="ws-h" tabindex="-1">When do your classes meet?</h2><p class="st-hint" id="ws-status" role="status">Reading your calendar…</p>' +
      '<section class="ws-part" id="ws-classes"><h3 class="st-sub">Your classes</h3><div class="wk-reg" id="ws-reg" hidden>' + KnowluWeek.registrarHtml() + "</div>" +
      '<div class="wk-changes" id="ws-reg-added" hidden></div><ul class="ws-rows" id="ws-class-rows"></ul></section>' +
      '<section class="ws-part" id="ws-week" hidden><h3 class="st-sub">The rest of your week</h3><ul class="ws-rows" id="ws-week-rows"></ul></section>' +
      '<section class="ws-part" id="ws-oh" hidden><h3 class="st-sub">Office hours</h3><ul class="ws-rows" id="ws-oh-rows"></ul></section>' +
      '<section class="ws-part"><h3 class="st-sub">Your day</h3><ul class="st-rows wk-window" id="ws-window"></ul></section>' +
      '<p class="st-hint ws-foot">Rows you leave blank will come back as questions over the next few days.</p>' +
      '<div class="ws-acts"><button class="qbtn" type="button" id="ws-later">Not now</button><button class="btn" type="button" id="ws-finish">Finish</button></div></div>';
  }

  /* One row: the title, the engine's when, where; Mine / Not mine; the level, shown on a Mine row. Nothing typed. */
  function setupRowHtml(p) {
    var preset = { class: 1, lab: 1, work: 1 }[p.kind] ? "mine" : "";
    var from = KnowluWeek.fromRegistrar(p) && registrarLabel ? " · from " + registrarLabel : "";
    var meta = (p.when || "") + (p.where ? " · " + p.where : "") + from;
    return '<li class="ws-row" data-key="' + h(p.source_uid) + '" data-answer="' + h(preset) + '" data-level="' + h(p.level) + '"><div class="ws-l"><p class="st-n">' + h(p.title) + '</p><p class="st-hint">' + h(meta) + "</p></div>" +
      '<div class="ws-c"><div class="seg ws-ans" role="group" aria-label="Is this yours? ' + h(p.title) + '"><button class="obtn" type="button" data-answer-set="mine" aria-pressed="' + (preset === "mine" ? "true" : "false") + '">Mine</button>' +
      '<button class="obtn" type="button" data-answer-set="not" aria-pressed="false">Not mine</button></div>' + KnowluWeek.levelHtml(p.level, h(p.title)) + "</div></li>";
  }
  function paintSetupRows(ps, uncovered) {
    var classes = ps.filter(function (p) { return !p.window && (p.kind === "class" || p.kind === "lab"); });
    var office = ps.filter(function (p) { return !p.window && p.kind === "office-hours"; });
    var rest = ps.filter(function (p) { return !p.window && classes.indexOf(p) === -1 && office.indexOf(p) === -1; });
    var uncoveredHtml = uncovered.map(function (u) { return '<li class="ws-row ws-uncovered"><div class="ws-l"><p class="st-n">' + h(u.title) + '</p><p class="st-hint">no class times found; Knowlu will ask this week</p></div></li>'; }).join("");
    el("ws-class-rows").innerHTML = classes.map(setupRowHtml).join("") + uncoveredHtml;
    el("ws-week-rows").innerHTML = rest.map(setupRowHtml).join("");
    el("ws-oh-rows").innerHTML = office.map(setupRowHtml).join("");
    el("ws-classes").hidden = !classes.length && !uncovered.length && !registrarLabel;
    el("ws-reg").hidden = !registrarLabel;
    if (registrarLabel) { el("ws-reg").querySelector("[data-reg-open]").textContent = "Get my class times from " + registrarLabel; }
    el("ws-week").hidden = !rest.length;
    el("ws-oh").hidden = !office.length;
  }

  function openWeekSetup(week) {
    weekSetup.open = true;
    el("ws-status").textContent = "Reading your calendar…";
    var byDay = {};
    DAYS.forEach(function (d) { byDay[d] = { start: "08:00", end: "22:00" }; });
    el("ws-window").innerHTML = KnowluWeek.editorHtml(byDay);
    paintSetupRows([], week.uncovered_courses || []);
    /* Finish waits for the calendar read to settle: a Finish before it would write the planning-day note from the
       placeholder hours, and the window proposal would never be offered again */
    weekSetup.dirty = false;
    el("ws-finish").disabled = true; weekSetup.reading = true;
    el("ws-later").disabled = true;
    var ready = function () {
      weekSetup.reading = false;
      if (!ROOT) { return; }
      el("ws-later").disabled = !finishMayRun();
      if (finishMayRun()) { el("ws-finish").disabled = false; }
    };
    KnowluData.call("commitment_proposals", {}).then(function (r) {
      if (!ROOT) { return; }
      var ps = (r && r.proposals) || [];
      var win = ps.filter(function (p) { return p.window; })[0];
      /* an edit the student made while waiting is theirs: the proposal does not overwrite it */
      if (win && !weekSetup.dirty) {
        byDay = {};
        win.meets.forEach(function (m) { m.days.forEach(function (d) { byDay[d] = { start: m.start, end: m.end }; }); });
        el("ws-window").innerHTML = KnowluWeek.editorHtml(byDay);
      }
      paintSetupRows(ps, (r && r.uncovered_courses) || week.uncovered_courses || []);
      el("ws-status").textContent = ps.some(function (p) { return !p.window; }) ? FOUND : NOTHING;
    }).then(ready);
  }

  /* R5-b: after a fetch the sheet reads its rows again, and the answers and levels already given are put back by key */
  function reloadSetupRows() {
    var kept = {};
    ROOT.querySelectorAll(".ws-row[data-key]").forEach(function (r) {
      kept[r.getAttribute("data-key")] = { answer: r.getAttribute("data-answer"), level: r.getAttribute("data-level") };
    });
    return KnowluData.call("commitment_proposals", {}).then(function (r) {
      if (!ROOT) { return; }
      paintSetupRows((r && r.proposals) || [], (r && r.uncovered_courses) || []);
      ROOT.querySelectorAll(".ws-row[data-key]").forEach(function (row) {
        var key = row.getAttribute("data-key");
        if (!Object.prototype.hasOwnProperty.call(kept, key)) { return; }
        var k = kept[key];
        row.setAttribute("data-answer", k.answer || "");
        row.setAttribute("data-level", k.level || "");
        row.querySelectorAll("[data-answer-set]").forEach(function (b) { b.setAttribute("aria-pressed", b.getAttribute("data-answer-set") === k.answer ? "true" : "false"); });
        row.querySelectorAll("[data-level-set]").forEach(function (b) { b.setAttribute("aria-pressed", b.getAttribute("data-level-set") === k.level ? "true" : "false"); });
      });
      return KnowluData.call("your_week", {}).then(function (w) {
        if (!ROOT) { return; }
        paintRegistrarAdded(w && w.ok && w.week ? w.week.commitments : [], w && w.ok && w.week ? w.week.registrar_changes : null);
      });
    });
  }
  function addedHtml(c) {
    return '<li><span class="ws-added">' + h(c.title) + '</span> <span class="wk-card">' + h(c.when || "") + "</span></li>";
  }
  /* phase 3 I3: the sheet lists what the registrar confirmed (your_week's rows keyed registrar:) and its change list */
  function paintRegistrarAdded(commitments, changes) {
    var added = (commitments || []).filter(function (c) { return c.source_uid && c.source_uid.indexOf("registrar:") === 0; });
    var box = el("ws-reg-added");
    var listHtml = KnowluWeek.changesHtml(changes, commitments, registrarLabel);
    box.hidden = (!added.length && !listHtml) || !registrarLabel;
    var headHtml = "";
    if (added.length) {
      headHtml = '<p class="st-sub">From ' + h(registrarLabel) + " — added</p><ul>" + added.map(addedHtml).join("") + "</ul>";
    }
    box.innerHTML = box.hidden ? "" : headHtml + listHtml;
  }

  function closeWeekSetup() {
    weekSetup.open = false;
    /* phase 3 m2: the school's window goes with the sheet, wiped as on Cancel */
    KnowluData.call("close_registrar_window", {});
    var api = API;
    API = null;
    if (api) { api.close(); }
  }
  function finishWeekSetup() {
    var mine = [], notMine = [];
    ROOT.querySelectorAll(".ws-row[data-key]").forEach(function (r) {
      var key = r.getAttribute("data-key"), a = r.getAttribute("data-answer");
      if (a === "mine") { mine.push({ source_uid: key, level: r.getAttribute("data-level") }); }
      else if (a === "not") { notMine.push(key); }
    });
    /* D3: Finish always writes the planning-day note, so the sheet never returns; a blank Your day would write none */
    var seq = KnowluWeek.sequence(el("ws-window"));
    if (!seq) { KnowluWeek.showError(el("ws-window"), "Set the hours for at least one day"); return; }
    var payload = { mine: mine, not_mine: notMine, window: seq };
    setupFinishing(true);
    KnowluData.call("commitments_confirm", { confirm: payload }).then(function (env) {
      setupFinishing(false);
      if (!env || !env.ok) {
        var why = KnowluKit.refusal(env);
        KnowluWeek.showError(el("ws-window"), why);
        el("ws-status").textContent = why;
        return;
      }
      /* M4: a row --confirm skipped comes back as a question; say so rather than close in silence */
      var skipped = (env.result && env.result.warnings) || [];
      closeWeekSetup();
      if (skipped.length) { KnowluKit.toast(skipped.length + (skipped.length === 1 ? " row" : " rows") + " will come back as questions: " + skipped.join("; ")); }
    });
  }

  function onSheetClick(e) {
    var row = e.target.closest(".ws-row[data-key]");
    var a = e.target.closest("[data-answer-set]");
    if (a && row) {
      /* Q11-c: a second press on the pressed answer returns the row to unanswered */
      var v = a.getAttribute("data-answer-set"), next = row.getAttribute("data-answer") === v ? "" : v;
      row.setAttribute("data-answer", next);
      row.querySelectorAll("[data-answer-set]").forEach(function (b) { b.setAttribute("aria-pressed", b.getAttribute("data-answer-set") === next ? "true" : "false"); });
      return;
    }
    var l = e.target.closest("[data-level-set]");
    if (l && row) {
      row.setAttribute("data-level", l.getAttribute("data-level-set"));
      row.querySelectorAll("[data-level-set]").forEach(function (b) { b.setAttribute("aria-pressed", b === l ? "true" : "false"); });
      return;
    }
    if (e.target.closest("#ws-later")) { if (finishMayRun()) { weekSetup.dismissed = true; closeWeekSetup(); } return; }
    if (e.target.closest("#ws-finish")) { finishWeekSetup(); }
  }

  function renderSheet(host, api) {
    API = api;
    host.innerHTML = sheetHtml();
    ROOT = host.querySelector(".ws");
    ROOT.addEventListener("click", onSheetClick);
    KnowluWeek.bindEditor(el("ws-window"), function () { weekSetup.dirty = true; KnowluWeek.showError(el("ws-window"), null); });
    KnowluWeek.bindRegistrar(el("ws-reg"), reloadSetupRows, setupBusy);
    openWeekSetup(WEEK_NOW || {});
    el("ws-h").focus({ preventScroll: true });
  }
  function onSheetClosed() {
    ROOT = null;
    API = null;
    weekSetup.open = false;
  }

  KnowluBoot.onConsole(checkWeekSetup);
})();
