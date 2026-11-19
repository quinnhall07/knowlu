/* Knowlu app, Home's main pane (T7: note §4 Home, plan T7; DESIGN.md "The desktop app").

   What it draws, all of it the read model's (the page computes nothing):
     the domain row     every active class (KnowluCourses.active, M1) with its standing in three words (A3): behind is
                        Behind, tight and steady are On track, ahead is Ahead, none is no word. A press filters Do today
                        and Recommended to that class; a second press clears it. The aside is never filtered.
     Do today           S.must_do.groups[].rows, in the engine's order. The lead is the first unchecked row (else the first
                        Recommended row): the coloured field, with Lock in (#lockin/<id>). A row is its check (the kit's),
                        its name (opens Details), its class, its minutes and its due. A pending amendment is the row's own
                        marker (row.amend_badge, N1). Start by, slack and Won't fit are Details' (A4).
     the totals         how many unchecked rows are left and their minutes; "Over by" the meter's spill when the verdict
                        has an overflow sentence (it opens a panel: the sentence, the give text, the spill items); with
                        nothing left, "Clear through" the day before the first busy day ahead (I6, N10).
     can't be read      S.unreadable, in Do today, never hidden (I1).
     Recommended        S.recommended.rows[] ({row, take_hours, of_hours}), else the engine's empty text.
     the first run      while the envelope carries first_run (the state is null): "Setting up your day" and the steps;
                        data.js asks every three seconds until the day arrives.
   The aside (Waiting, today's schedule, free time) is home_aside.js's: Home mounts it in its two slots and hands it
   the whole state. Sends no command of its own: the kit writes a check, the router reads. */
(function () {
  "use strict";

  var STANDING = { behind: "Behind", tight: "On track", steady: "On track", ahead: "Ahead" };
  var FIRST_RUN_SAYS = { sync: "Syncing with your account", entitlement: "Checking your account", coursework: "Fetching your coursework", ingest: "Reading your school calendar", judge: "Working out what each task needs", rank: "Putting your day in order" };
  var FIRST_NOTE = "couldn't check — will retry";
  var STATE_WORDS = { done: "Done", failed: "Didn't work", skipped: "Skipped", now: "Now" };

  var LAYOUT_HTML = '<div class="home" id="hm">' +
    '<div class="hm-first" id="hm-first" hidden>' +
    '<h1 class="page-h">' + "Setting up your day" + "</h1>" +
    '<ol class="hm-steps" id="hm-steps" aria-live="polite"></ol>' +
    "<p class='hm-first-end' id='hm-first-end' hidden>The first run didn't finish. Knowlu will try again on its own.</p>" +
    "</div>" +
    '<div class="hm-day" id="hm-day" hidden>' +
    '<h1 class="page-h hm-h">Home</h1>' +
    '<div class="hm-doms" id="hm-doms" role="group" aria-label="Filter by class"></div>' +
    '<div class="hm-col">' +
    '<div class="hm-pane hm-pane-today">' +
    '<section class="hm-sec" id="hm-sec-today" aria-labelledby="hm-lbl-today" data-rows>' +
    '<h2 class="hm-lbl" id="hm-lbl-today">Do today</h2><div id="hm-today"></div></section>' +
    '<div class="hm-slot" id="hm-aside-wait"></div>' +
    "</div>" +
    '<div class="hm-pane hm-pane-rec">' +
    '<section class="hm-sec" id="hm-sec-rec" aria-labelledby="hm-lbl-rec" data-rows>' +
    '<h2 class="hm-lbl" id="hm-lbl-rec">Recommended</h2><div id="hm-rec"></div></section>' +
    '<div class="hm-slot" id="hm-aside-day"></div>' +
    "</div>" +
    "</div>" +
    "</div>" +
    "</div>";

  var ROOT = null, ST = null, FILTER = null, FIRST_HTML = "";

  function $(id) { return document.getElementById(id); }

  /* ---- the first run ---- */
  function firstRunSays(name) {
    var k = String(name == null ? "" : name).split(/[ (]/)[0];
    return Object.prototype.hasOwnProperty.call(FIRST_RUN_SAYS, k) ? FIRST_RUN_SAYS[k] : null;
  }
  function stepRowHtml(state, says, note) {
    return '<li class="hm-step" data-state="' + h(state) + '"><i class="hm-step-mk" aria-hidden="true"></i><span class="vh">' + h(STATE_WORDS[state] || "") + "</span>" +
      '<span class="hm-step-say">' + h(says) + "</span>" + (note ? '<span class="hm-step-note">' + h(note) + "</span>" : "") + "</li>";
  }
  /* an account check that could not reach the service lands at code 0 (a network is not a failed slot), but a check
     mark would say it worked: it shows as failed, with a note. A step with no sentence (engine: ...) has no row. */
  function stepHtml(s) {
    var says = firstRunSays(s[0]);
    if (!says) { return ""; }
    var unchecked = String(s[0]).indexOf("entitlement (refresh failed") === 0;
    var state = String(s[0]).indexOf("(skipped:") !== -1 ? "skipped" : (s[1] === 0 && !unchecked ? "done" : "failed");
    return stepRowHtml(state, says, unchecked ? FIRST_NOTE : "");
  }
  function drawFirstRun(fr) {
    fr = fr || {};
    $("hm-day").hidden = true;
    $("hm-first").hidden = false;
    var now = fr.running ? firstRunSays(fr.current) : null;
    var html = (fr.steps || []).map(stepHtml).join("") + (now ? stepRowHtml("now", now, "") : "");
    /* repainted only when it changed: a fresh list every three seconds would re-announce it */
    if (html !== FIRST_HTML) {
      $("hm-steps").innerHTML = html;
      FIRST_HTML = html;
    }
    /* any failed step counts, listed or not: the engine missing is `engine: ...` at -1, which has no row */
    var failed = (fr.steps || []).some(function (s) { return s[1] !== 0; });
    $("hm-first-end").hidden = !(!fr.running && failed);
  }

  /* ---- the domain row ---- */
  function domHtml(k) {
    var word = Object.prototype.hasOwnProperty.call(STANDING, k.standing) ? STANDING[k.standing] : "";
    return '<button class="hm-dom ' + h(KnowluCourses.slot(k)) + (k.standing === "behind" ? " is-behind" : "") + '" type="button" data-dom="' + h(k.slug) +
      '" aria-pressed="' + (FILTER === k.slug ? "true" : "false") + '"><i class="mk" aria-hidden="true"></i><b>' + h(k.title) + "</b>" +
      (word ? "<span>" + h(word) + "</span>" : "") + "</button>";
  }
  function domsHtml() {
    return KnowluCourses.all().filter(function (k) { return KnowluCourses.active(k); }).map(domHtml).join("");
  }
  function setFilter(slug) {
    FILTER = FILTER === slug ? null : slug;
    Array.prototype.forEach.call($("hm-doms").querySelectorAll("[data-dom]"), function (b) {
      b.setAttribute("aria-pressed", b.getAttribute("data-dom") === FILTER ? "true" : "false");
    });
    drawLists();
  }
  /* the filter: Do today and Recommended only */
  function shown(list, courseOf) {
    if (!FILTER) { return list; }
    return list.filter(function (it) {
      var k = KnowluCourses.match(courseOf(it));
      return !!k && k.slug === FILTER;
    });
  }
  function filterCode() {
    var k = KnowluCourses.of(FILTER);
    return k ? k.title : "";
  }

  /* ---- rows ---- */
  function minutesText(hours, of) {
    var m = KnowluFmt.minutes((Number(hours) || 0) * 60);
    return of != null && Number(of) > (Number(hours) || 0) + 0.01 ? m + " of " + KnowluFmt.minutes(Number(of) * 60) : m;
  }
  function amendHtml(row) {
    var a = row.amend_badge;
    return (a && a.sentence ? '<span class="hm-amend">' + h(a.sentence) + "</span>" : "");
  }
  function codeHtml(row) {
    var k = KnowluCourses.match(row.course);
    return (k ? KnowluKit.markHtml(row.course) + h(k.title) : "");
  }
  function rowHtml(row, take, of) {
    var slot = KnowluCourses.slot(KnowluCourses.match(row.course));
    var hours = take == null ? row.hours : take;
    return '<li class="hm-row hov' + (slot ? " " + h(slot) : "") + (row.status === "done" ? " is-done" : "") + '" data-row data-id="' + h(row.id) + '" data-kind="task">' +
      KnowluKit.checkHtml(row) +
      '<button class="hm-nm" type="button" data-det data-open="' + h(row.id) + '">' + h(row.title) + "</button>" +
      '<span class="hm-code">' + codeHtml(row) + "</span>" +
      '<span class="hm-min">' + h(minutesText(hours, of)) + "</span>" +
      '<span class="hm-due">' + h(KnowluFmt.due(row.due)) + "</span>" +
      amendHtml(row) + "</li>";
  }
  function leadHtml(lead) {
    var row = lead.row;
    var slot = KnowluCourses.slot(KnowluCourses.match(row.course));
    var hours = lead.take ? lead.take.take_hours : row.hours;
    var of = lead.take ? lead.take.of_hours : null;
    return '<div class="hm-field' + (slot ? " " + h(slot) : "") + '" data-row="lead" data-id="' + h(row.id) + '" data-kind="task" role="group" aria-labelledby="hm-lead-name">' +
      '<div class="hm-fin">' +
      '<span class="hm-fck">' + KnowluKit.checkHtml(row) + "</span>" +
      '<h3 class="hm-name" id="hm-lead-name"><button class="hm-name-btn" type="button" data-det data-open="' + h(row.id) + '">' + h(row.title) + "</button></h3>" +
      '<span class="hm-lockc"><a class="hm-lock" href="' + h("#lockin/" + row.id) + '" aria-describedby="hm-lead-name">Lock in</a></span>' +
      '<span class="hm-code">' + codeHtml(row) + "</span>" +
      '<span class="hm-min">' + h(minutesText(hours, of)) + "</span>" +
      '<span class="hm-due">' + h(KnowluFmt.lead(row.due)) + "</span>" +
      amendHtml(row) +
      "</div></div>";
  }
  function emptyHtml(text) {
    return (text ? '<p class="hm-empty">' + h(text) + "</p>" : "");
  }
  function filteredText() {
    return "Nothing for " + filterCode();
  }

  /* the lead: the first unchecked Do today row, else the first Recommended row */
  function leadOf(open, rec) {
    if (open.length) { return { row: open[0], where: "today", take: null }; }
    if (rec.length) { return { row: rec[0].row, where: "rec", take: rec[0] }; }
    return null;
  }

  /* ---- the totals ---- */
  function dayWord(x) {
    var d = KnowluFmt.day(x);
    return d === "Today" || d === "Tomorrow" ? d.toLowerCase() : d;
  }
  /* only with nothing left in Do today: the first day ahead that holds work. None in two weeks says so; today itself
     says nothing; else clear through the day before it */
  function clearLine(st, open) {
    if (open.length) { return ""; }
    var b = (st.ahead && st.ahead.buckets) || [];
    for (var i = 0; i < b.length && i < 14; i += 1) {
      if (b[i] && b[i].items > 0) { return i === 0 ? "" : "Clear through " + dayWord(b[i - 1].date); }
    }
    return "Clear for two weeks";
  }
  function sumHtml(st, open, allOpen) {
    var clear = clearLine(st, allOpen);
    if (!open.length) {
      return (clear ? '<div class="hm-sum"><span class="hm-sum-l"><span class="hm-sum-a">' + h(clear) + "</span></span></div>" : "");
    }
    var mins = open.reduce(function (n, r) { return n + (Number(r.hours) || 0) * 60; }, 0);
    var overHtml = (st.verdict && st.verdict.overflow_sentence && st.meter ? '<button class="hm-over tbtn" type="button" data-det data-over>' + h("Over by " + KnowluFmt.minutes((Number(st.meter.spill_hours) || 0) * 60)) + "</button>" : "");
    return '<div class="hm-sum"><span class="hm-sum-l"><span class="hm-sum-a">' + h(open.length + " left") + "</span>" + overHtml + "</span>" +
      '<span class="hm-sum-b">' + h(KnowluFmt.minutes(mins)) + "</span></div>";
  }
  function pathHtml(p) {
    return "<li>" + h(p) + "</li>";
  }
  function unreadableHtml(st) {
    var bad = Array.isArray(st.unreadable) ? st.unreadable : [];
    if (!bad.length) { return ""; }
    var n = bad.length === 1 ? "1 task note can't be read" : bad.length + " task notes can't be read";
    return '<div class="hm-unread" role="status"><p class="hm-unread-n">' + h(n) + '</p><ul class="hm-unread-paths">' + bad.map(pathHtml).join("") + "</ul></div>";
  }

  /* ---- the over-by panel ---- */
  function overloadHtml(st) {
    if (!st) { return ""; }
    var m = st.meter || {};
    var items = Array.isArray(m.spill_items) ? m.spill_items : [];
    return '<p class="hm-ov-s">' + h(st.verdict && st.verdict.overflow_sentence) + "</p>" +
      (m.give_text ? '<p class="hm-ov-g">' + h(m.give_text) + "</p>" : "") +
      '<ul class="hm-ov-list">' + items.map(function (it) { return '<li class="hm-ov-r"><span>' + h(it.title) + "</span><span>" + h(KnowluFmt.minutes((Number(it.hours) || 0) * 60)) + "</span></li>"; }).join("") + "</ul>";
  }
  KnowluPanel.add("overload", {
    render: function (body, id, api) {
      var st = ST || KnowluData.state();
      api.title(st && st.meter ? "Over by " + KnowluFmt.minutes((Number(st.meter.spill_hours) || 0) * 60) : "");
      body.innerHTML = overloadHtml(st);
    }
  });

  /* ---- the page ---- */
  function todayHtml(st, rows, open, allOpen, lead) {
    var md = st.must_do || {};
    var list = lead && lead.where === "today" ? rows.filter(function (r) { return r !== lead.row; }) : rows;
    var noneHtml = rows.length ? "" : emptyHtml(FILTER ? filteredText() : md.empty_text);
    return (lead && lead.where === "today" ? leadHtml(lead) : "") +
      '<ul class="hm-rows" aria-label="Do today">' + list.map(function (r) { return rowHtml(r); }).join("") + "</ul>" +
      noneHtml + sumHtml(st, open, allOpen) + unreadableHtml(st);
  }
  function recHtml(st, rec, lead) {
    var r = st.recommended || {};
    var list = lead && lead.where === "rec" ? rec.slice(1) : rec;
    var noneHtml = rec.length ? "" : emptyHtml(FILTER ? filteredText() : r.empty_text);
    return (lead && lead.where === "rec" ? leadHtml(lead) : "") +
      '<ol class="hm-rows" aria-label="Recommended, best first">' + list.map(function (x) { return rowHtml(x.row, x.take_hours, x.of_hours); }).join("") + "</ol>" + noneHtml;
  }
  function drawLists() {
    var st = ST;
    if (!st || !ROOT) { return; }
    var all = [];
    ((st.must_do || {}).groups || []).forEach(function (g) {
      (g.rows || []).forEach(function (r) { all.push(r); });
    });
    var recAll = ((st.recommended || {}).rows || []).filter(function (x) { return x && x.row; });
    var allOpen = all.filter(function (r) { return r.status !== "done"; });
    var rows = shown(all, function (r) { return r.course; });
    var open = rows.filter(function (r) { return r.status !== "done"; });
    var rec = shown(recAll, function (x) { return x.row.course; });
    var lead = leadOf(open, rec);
    $("hm-today").innerHTML = todayHtml(st, rows, open, allOpen, lead);
    $("hm-rec").innerHTML = recHtml(st, rec, lead);
  }
  function drawDay(st) {
    ST = st;
    $("hm-first").hidden = true;
    $("hm-day").hidden = false;
    FIRST_HTML = "";
    if (FILTER && !KnowluCourses.of(FILTER)) { FILTER = null; }
    $("hm-doms").innerHTML = domsHtml();
    drawLists();
    if (window.KnowluHomeAside) { KnowluHomeAside.update(st); }
  }
  function paint(st, fr) {
    if (!ROOT) { return; }
    if (st) {
      drawDay(st);
      return;
    }
    if (fr) { drawFirstRun(fr); }
  }

  function onClick(e) {
    var t = e.target;
    var dom = t.closest("[data-dom]");
    if (dom) {
      setFilter(dom.getAttribute("data-dom"));
      return;
    }
    var open = t.closest("[data-open]");
    if (open) {
      KnowluPanel.open("task", open.getAttribute("data-open"));
      return;
    }
    if (t.closest("[data-over]")) { KnowluPanel.open("overload", "over"); }
  }

  KnowluRouter.add("home", {
    mount: function (center, ctx) {
      center.innerHTML = LAYOUT_HTML;
      ROOT = center.querySelector(".home");
      FILTER = null;
      FIRST_HTML = "";
      ROOT.addEventListener("click", onClick);
      if (window.KnowluHomeAside) { KnowluHomeAside.mount($("hm-aside-wait"), $("hm-aside-day")); }
      paint(ctx.state, ctx.firstRun);
    },
    update: function (st) {
      paint(st, KnowluData.firstRun());
    },
    unmount: function () {
      if (window.KnowluHomeAside) { KnowluHomeAside.unmount(); }
      ROOT = null;
      ST = null;
    }
  });
})();
