/* Knowlu app: the Calendar (v0.4.0 note §4 "Calendar", ruling 3, A6, D6; plan T11). Read-only in v0.4.0: nothing
   drags, resizes or is created here (the cut list, note §1); the one write is the Due lane's check, the shared control
   (KnowluKit.checkHtml, with its Undo).

   What it draws, from the read model (S, view "all") and your_week (read at mount and when the day turns):
     - a top row: the range's title (Go to date), previous, next, Today, Not shown (N), and the days shown;
     - one column per day: its name, number and load (KnowluCalData.dayLoad), a Due lane (today's all-day lines,
       accepted events, the open rows due that day, each with the shared check) and the 24 hours;
     - blocks: every commitment's meetings (classes in their slot, by KnowluCourses.match; the rest neutral) and,
       on today only, the calendar's busy time no commitment covers (A6). No event of another day is drawn.
   Navigation (ruling 3): T today; J and K the next and previous days shown; G go to date; Shift with Left or Right one
   day; 1 to 7 the days shown; Ctrl with =, - or 0 the hour height; a sideways wheel pans by whole days. Below 1000 px
   of center the page opens at 3 days (D6), else the week from Monday. The hour height and the Due fold are kept in
   localStorage (knowlu.cal.zoom, knowlu.cal.due, note D4).
   "Not shown (N)" lists dropped_events, read once when the page mounts; read-only.
   Pressing a block opens its Details in the panel (kind "cal-thing"); pressing a due name opens the task's Details.
   The now line, the gutter's now label and today's past blocks follow S.now on every poll, not only on a new
   revision (I2). */
(function () {
  "use strict";
  var D = null;  /* KnowluCalData, read at mount (scripts load in any order) */
  var ZOOM_KEY = "knowlu.cal.zoom";
  var DUE_KEY = "knowlu.cal.due";
  var ZOOMS = ["s", "m", "l"];
  var HH = { s: 44, m: 60, l: 80 };   /* an hour's height, px */
  var LANE_ROWS = 3;                  /* rows a day's lane shows before "N more" */
  var ROW_H = 26;
  var WD = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  var MO = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  var KIND_WORD = { "class": "Class", lab: "Lab", shift: "Shift", club: "Club", event: "Event", study: "Study", "office-hours": "Office hours" };
  var LEVEL_WORD = { hard: "Fixed", soft: "Usually", optional: "Optional" };
  var UNAVAILABLE = "Not shown is unavailable right now.";
  var CHEVRON = '<svg viewBox="0 0 18 18" aria-hidden="true"><path d="M4.5 6.75L9 11.25l4.5-4.5"/></svg>';
  var PREV_SVG = '<svg viewBox="0 0 18 18" aria-hidden="true"><path d="M11.25 4.5L6.75 9l4.5 4.5"/></svg>';
  var NEXT_SVG = '<svg viewBox="0 0 18 18" aria-hidden="true"><path d="M6.75 4.5L11.25 9l-4.5 4.5"/></svg>';
  var SKELETON_HTML = '<div class="cal" id="cal">' +
    '<div class="cal-bar">' +
      '<h1 class="cal-h"><button class="cal-title" id="cal-title" type="button" aria-haspopup="dialog" aria-expanded="false" aria-controls="cal-datepop" data-tip="Go to date, G"></button></h1>' +
      '<button class="obtn ico" id="cal-prev" type="button" aria-label="Previous" data-tip="Previous, K">' + PREV_SVG + '</button>' +
      '<button class="obtn ico" id="cal-next" type="button" aria-label="Next" data-tip="Next, J">' + NEXT_SVG + '</button>' +
      '<button class="obtn" id="cal-today" type="button" data-tip="Today, T">Today</button>' +
      '<div class="cal-gap"></div>' +
      '<div class="menu-wrap"><button class="obtn" id="cal-ns" type="button" aria-expanded="false" aria-controls="cal-nsmenu" hidden></button>' +
        '<div class="menu nsmenu" id="cal-nsmenu" hidden></div></div>' +
      '<div class="menu-wrap"><button class="obtn" id="cal-view" type="button" aria-expanded="false" aria-controls="cal-viewset"><span id="cal-view-t"></span>' + CHEVRON + '</button>' +
        '<ul class="menu viewset" id="cal-viewset" aria-label="Days shown" hidden>' +
          '<li><button type="button" data-n="1"><span>Day</span><kbd>1</kbd></button></li>' +
          '<li><button type="button" data-n="3"><span>3 days</span><kbd>3</kbd></button></li>' +
          '<li><button type="button" data-n="5"><span>5 days</span><kbd>5</kbd></button></li>' +
          '<li><button type="button" data-n="7"><span>Week</span><kbd>7</kbd></button></li>' +
        '</ul></div>' +
      '<div class="datepop" id="cal-datepop" role="dialog" aria-label="Go to date" hidden></div>' +
    '</div>' +
    '<div class="wk" id="cal-wk"></div>' +
  '</div>';

  var CENTER = null;
  var S = null;          /* the state on screen */
  var WEEK = null;       /* your_week.week, or null when it could not be read */
  var WEEK_DAY = "";     /* the day your_week was read for */
  var N = 7;             /* days shown */
  var FROM = null;       /* the first day shown (Date) */
  var ZOOM = "m";
  var FOLDED = false;
  var NS = null;         /* Not shown: null (nothing), {list}, or {error} */
  var POP_MONTH = null;  /* the month Go to date shows */
  var THINGS = Object.create(null);  /* what a block's Details read, by its key */
  var WHEEL = 0;
  var MOUNTED = false;
  var NOW_MIN = "";      /* the day and minute the grid's now marks were drawn at */
  var DRAWN_DAY = "";    /* the day the grid was drawn as today */
  var UNPOLL = null;

  function $(id) { return document.getElementById(id); }
  function today() { return KnowluFmt.now(); }
  function todayIso() { return D.iso(today()); }
  function nowMins() { var t = today(); return t.getHours() * 60 + t.getMinutes(); }
  function hrs(m) { return String(Math.round(m / 60 * 10000) / 10000); }
  function clockOf(m) { return KnowluFmt.clock(new Date(2000, 0, 1, Math.floor(m / 60), m % 60)); }
  function rangeText(s, e) {
    var a = clockOf(s), b = clockOf(e);
    if (a.slice(-2) === b.slice(-2)) { a = a.slice(0, -3); }
    return a + " to " + b;
  }
  function hourLabel(hr) { return (hr % 12 || 12) + " " + (hr < 12 ? "AM" : "PM"); }
  function titleText() {
    var a = FROM, b = D.add(FROM, N - 1);
    if (N === 1) { return WD[a.getDay()] + ", " + MO[a.getMonth()] + " " + a.getDate(); }
    return MO[a.getMonth()] + " " + a.getDate() + " – " + (b.getMonth() === a.getMonth() ? "" : MO[b.getMonth()] + " ") + b.getDate();
  }
  function viewWord() { return N === 1 ? "Day" : N === 7 ? "Week" : N + " days"; }
  function shown(date) { var f = D.iso(FROM), l = D.iso(D.add(FROM, N - 1)); return date >= f && date <= l; }

  /* ---- stored choices (D4): a refused storage keeps the choice for this visit ---- */
  function readStored() {
    try {
      var z = window.localStorage.getItem(ZOOM_KEY);
      ZOOM = ZOOMS.indexOf(z) >= 0 ? z : "m";
      FOLDED = window.localStorage.getItem(DUE_KEY) === "folded";
    } catch (e) { ZOOM = "m"; FOLDED = false; }
  }
  function setZoom(z) {
    if (ZOOMS.indexOf(z) < 0 || z === ZOOM) { return; }
    var old = HH[ZOOM], top = CENTER.scrollTop;
    ZOOM = z;
    try { window.localStorage.setItem(ZOOM_KEY, z); } catch (e) { /* this visit only */ }
    draw();
    CENTER.scrollTop = Math.round(top * HH[z] / old);
  }
  function stepZoom(dir) {
    var i = ZOOMS.indexOf(ZOOM) + dir;
    if (i >= 0 && i < ZOOMS.length) { setZoom(ZOOMS[i]); }
  }
  function setFold(f) {
    FOLDED = !!f;
    try { window.localStorage.setItem(DUE_KEY, FOLDED ? "folded" : "open"); } catch (e) { /* this visit only */ }
    draw();
  }

  /* ---- the range ---- */
  function firstDays(center) {
    return center.clientWidth < 1000 ? 3 : 7;
  }
  function anchorFor(n, at) { return n === 7 ? D.monday(at) : at; }
  function goToday() { FROM = anchorFor(N, today()); draw(); scrollToDay(); }
  function shift(days) { FROM = D.add(FROM, days); draw(); }
  function setN(n) {
    if (n < 1 || n > 7) { return; }
    var at = shown(todayIso()) ? today() : FROM;
    N = n;
    FROM = anchorFor(n, at);
    draw();
  }
  function goTo(d) { FROM = anchorFor(N, d); closePops(); draw(); }

  /* ---- reads ---- */
  function loadWeek() {
    var asked = todayIso();
    WEEK_DAY = asked;
    KnowluData.call("your_week", {}).then(function (r) {
      if (!MOUNTED) { return; }
      WEEK = r && r.ok && r.week ? r.week : null;
      draw();
      scrollToDay();
    });
  }
  function loadNotShown() {
    KnowluData.call("dropped_events", {}).then(function (r) {
      if (!MOUNTED) { return; }
      if (!r || !r.ok) {
        NS = { error: true };
      } else {
        var list = Array.isArray(r.dropped) ? r.dropped : [];
        NS = !list.length ? null : { list: list };
      }
      drawNotShown();
    });
  }
  function notShownHtml(list) {
    function nsRowHtml(d) {
      return '<li class="ns-r"><span class="ns-when">' + h(d.date ? KnowluFmt.day(d.date) + (d.time ? ", " + clockOf(D.mins(d.time) || 0) : "") : "") + '</span><span class="ns-t">' + h(d.title) + '</span><span class="ns-why">' + h(d.reason) + "</span></li>";
    }
    return '<ul class="ns-list">' + list.map(nsRowHtml).join("") + "</ul>";
  }
  function drawNotShown() {
    var b = $("cal-ns"), m = $("cal-nsmenu");
    if (!b) { return; }
    b.hidden = !NS;
    b.textContent = NS && NS.list ? "Not shown (" + NS.list.length + ")" : "Not shown";
    m.innerHTML = NS && NS.list ? notShownHtml(NS.list) : '<p class="ns-none">' + h(UNAVAILABLE) + "</p>";
  }

  /* ---- the columns ---- */
  function eventHtml(e) {
    return '<li class="lev" tabindex="0" data-tip="' + h(e.provenance || "Accepted") + '"><i class="mk ev" aria-hidden="true"></i><span class="due-n">' + h(e.title) + '</span><span class="due-at">' + h(e.time ? clockOf(D.mins(e.time) || 0) : "") + "</span></li>";
  }
  function allDayHtml(text) {
    return '<li class="lad"><i class="mk ad" aria-hidden="true"></i><span class="due-n">' + h(text) + "</span></li>";
  }
  /* a day's lane items: today's all-day lines, the accepted events, the open rows due */
  function laneItems(date, events) {
    var items = [];
    if (date === todayIso() && S && S.the_day && Array.isArray(S.the_day.all_day)) {
      S.the_day.all_day.forEach(function (t) { items.push({ t: "ad", v: t }); });
    }
    events.filter(function (e) { return e.date === date; }).forEach(function (e) { items.push({ t: "ev", v: e }); });
    KnowluCalData.dueOn(S, date).forEach(function (r) { items.push({ t: "due", v: r }); });
    return items;
  }
  /* a day's lane: the open rows due carry the shared check control (A8: done fills with the class colour) */
  function laneHtml(date, items) {
    function dueRowHtml(r) {
      var p = KnowluFmt.parse(r.due);
      var at = p && p.time && !(p.d.getHours() === 23 && p.d.getMinutes() === 59) ? KnowluFmt.clock(p.d) : "";
      return '<li class="due" data-row="' + h(r.id) + '">' + KnowluKit.checkHtml(r) +
        '<button class="due-nm" type="button" data-det="' + h(r.id) + '" data-kind="task"><span class="due-n">' + h(r.title) + '</span><span class="due-at">' + h(at) + "</span></button></li>";
    }
    function itemHtml(it) {
      if (it.t === "ad") { return allDayHtml(it.v); }
      if (it.t === "ev") { return eventHtml(it.v); }
      return dueRowHtml(it.v);
    }
    if (FOLDED) {
      var open = items.filter(function (it) { return it.t === "due"; }).length;
      return '<p class="due-sum">' + h(open ? open + " due" : "") + "</p>";
    }
    var shownItems = items.length > LANE_ROWS + 1 ? items.slice(0, LANE_ROWS) : items;
    var rowsHtml = shownItems.map(itemHtml).join("");
    var moreHtml = (items.length > shownItems.length ? '<li class="due-more">' + h((items.length - shownItems.length) + " more") + "</li>" : "");
    return '<ul class="lane-l" aria-label="' + h("Due, " + date) + '">' + rowsHtml + moreHtml + "</ul>";
  }
  function blockHtml(x, date, isPast) {
    var t = x.ref;
    var key = t.key;
    THINGS[key] = t;
    var cls = "blk k-" + (t.src === "busy" ? "busy" : (t.kind || "other").replace(/[^a-z-]/g, "")) + " lv-" + (t.src === "busy" ? "soft" : t.level) + " " + (t.slot || "cfix") + (isPast ? " is-past" : "");
    var label = t.title + ", " + rangeText(t.s, t.e) + (t.src === "busy" ? ", from your calendar" : "");
    return '<li class="' + h(cls) + '" data-src="' + h(t.src) + '" data-thing="' + h(key) + '" style="--s:' + h(hrs(x.s)) + ";--e:" + h(hrs(x.e)) + ";--li:" + h(x.li) + ";--ln:" + h(x.ln) + '">' +
      '<button class="blk-b" type="button" data-det="' + h(key) + '" data-cal="' + h(key) + '" aria-label="' + h(label) + '"><span class="blk-in"><b class="blk-nm">' + h(t.title) + '</b><span class="blk-t">' + h(rangeText(t.s, t.e)) + "</span></span></button></li>";
  }
  function hoursHtml(date, things, win) {
    var linesHtml = "";
    for (var hr = 1; hr < 24; hr++) {
      linesHtml += '<i class="hr" style="--h:' + h(hr) + '"></i>';
    }
    var offHtml = (win ? '<i class="off" style="--a:0;--b:' + h(hrs(win.s)) + '"></i><i class="off" style="--a:' + h(hrs(win.e)) + ';--b:24"></i>' : "");
    var isToday = date === todayIso(), past = date < todayIso(), now = nowMins();
    var placed = D.lanes(things.map(function (t) { return { ref: t, s: t.s, e: t.e }; }));
    var blocksHtml = placed.map(function (x) { return blockHtml(x, date, past || (isToday && x.e <= now)); }).join("");
    var nowHtml = (isToday ? '<i class="now" style="--now:' + h(hrs(now)) + '"></i>' : "");
    return '<div class="hcell">' + offHtml + linesHtml + nowHtml + '<ul class="hours">' + blocksHtml + "</ul></div>";
  }
  function windowOf(d) {
    var w = WEEK && Array.isArray(WEEK.window) ? WEEK.window[(d.getDay() + 6) % 7] : null;
    var s = w ? D.mins(w.start) : null, e = w ? D.mins(w.end) : null;
    return s != null && e != null && e > s ? { s: s, e: e } : null;
  }
  function gutterHtml(isTodayShown) {
    var labelsHtml = "";
    for (var hr = 1; hr < 24; hr++) {
      labelsHtml += '<span class="hl" style="--h:' + h(hr) + '">' + h(hourLabel(hr)) + "</span>";
    }
    var nowHtml = (isTodayShown ? '<span class="hl is-now" style="--h:' + h(hrs(nowMins())) + '">' + h(clockOf(nowMins())) + "</span>" : "");
    return '<div class="gut"><div class="gut-n"></div><div class="gut-l">' +
      '<button class="due-fold" id="due-fold" type="button" aria-expanded="' + (FOLDED ? "false" : "true") + '" aria-label="' + (FOLDED ? "What is due, folded" : "What is due, shown") + '"><span>Due</span>' + CHEVRON + "</button></div>" +
      '<div class="gut-h" aria-hidden="true">' + labelsHtml + nowHtml + "</div></div>";
  }

  function draw() {
    var wk = $("cal-wk");
    if (!wk || !FROM) { return; }
    THINGS = Object.create(null);
    var occ = D.expand(WEEK, FROM, N);
    var events = KnowluCalData.accepted(S);
    var TODAY = todayIso();
    DRAWN_DAY = TODAY;
    NOW_MIN = TODAY + "T" + nowMins();
    var columns = [];
    var most = 0;
    for (var i = 0; i < N; i++) {
      var d = D.add(FROM, i), date = D.iso(d);
      var things = (occ[date] || []).map(function (o) { o.src = "commitment"; return o; });
      var unconfirmed = date === TODAY ? KnowluCalData.unconfirmed(S, occ[date]).map(function (b) { b.src = "busy"; b.slot = ""; b.level = "soft"; return b; }) : [];
      var items = laneItems(date, events);
      most = Math.max(most, items.length > LANE_ROWS + 1 ? LANE_ROWS + 1 : items.length);
      columns.push({ d: d, date: date, things: things.concat(unconfirmed), items: items });
    }
    var laneH = most ? (FOLDED ? ROW_H + 4 : most * ROW_H + 6) : 0;
    function columnHtml(c) {
      var load = KnowluCalData.dayLoad(S, c.date);
      var isToday = c.date === TODAY;
      return '<div class="day' + (isToday ? " is-today" : "") + '" data-date="' + h(c.date) + '">' +
        '<p class="dn"' + (isToday ? ' aria-current="date"' : "") + '><span class="dn-name">' + h(WD[c.d.getDay()]) + '</span><span class="dn-num">' + h(c.d.getDate()) + "</span>" +
        (load ? '<span class="dn-load">' + h(KnowluFmt.minutes(load * 60)) + "</span>" : "") + "</p>" +
        '<div class="lane">' + laneHtml(c.date, c.items) + "</div>" + hoursHtml(c.date, c.things, windowOf(c.d)) + "</div>";
    }
    var colsHtml = columns.map(columnHtml).join("");
    wk.setAttribute("data-n", String(N));
    wk.setAttribute("data-from", D.iso(FROM));
    wk.classList.toggle("no-lane", !laneH);
    wk.style.setProperty("--n", String(N));
    wk.style.setProperty("--hh", HH[ZOOM] + "px");
    wk.style.setProperty("--lane-h", laneH + "px");
    wk.innerHTML = gutterHtml(shown(TODAY)) + colsHtml;
    $("cal-title").textContent = titleText();
    $("cal-view-t").textContent = viewWord();
    document.querySelectorAll("#cal-viewset [data-n]").forEach(function (b) { b.setAttribute("aria-pressed", String(+b.getAttribute("data-n") === N)); });
    if (POP_MONTH && !$("cal-datepop").hidden) { drawPop(); }
    KnowluKit.watchSeen();
  }
  /* every poll (whole-branch review I2): the revision leaves now out, so data.js copies a later minute into the state
     on screen without a repaint. Only when that minute changed, the now line, the gutter's now label and today's past
     blocks move in place, so focus and scroll stay (wave-b1 M1); the grid is drawn again only when the day turned. */
  function nowMoved() {
    if (!MOUNTED || !FROM || !D) { return; }
    var key = todayIso() + "T" + nowMins();
    if (key === NOW_MIN) { return; }
    if (todayIso() !== DRAWN_DAY) { if (todayIso() !== WEEK_DAY) { loadWeek(); } draw(); return; }
    NOW_MIN = key;
    var now = nowMins(), at = hrs(now);
    var col = document.querySelector("#cal-wk .day.is-today");
    var line = col ? col.querySelector(".hcell > .now") : null;
    if (line) { line.style.setProperty("--now", at); }
    var label = document.querySelector("#cal-wk .hl.is-now");
    if (label) {
      label.style.setProperty("--h", at);
      label.textContent = clockOf(now);
    }
    if (!col) { return; }
    Array.prototype.forEach.call(col.querySelectorAll(".blk[data-thing]"), function (b) {
      var t = THINGS[b.getAttribute("data-thing")];
      if (t) { b.classList.toggle("is-past", t.e <= now); }
    });
  }
  function scrollToDay() {
    if (!CENTER || !FROM) { return; }
    var w = windowOf(today());
    var start = w ? w.s / 60 : 8;
    CENTER.scrollTop = Math.max(0, (start - 0.5) * HH[ZOOM]);
  }

  /* ---- Go to date: a month of day numbers ---- */
  function drawPop() {
    var first = new Date(POP_MONTH.getFullYear(), POP_MONTH.getMonth(), 1);
    var start = D.monday(first), TODAY = todayIso();
    var cellsHtml = "";
    for (var i = 0; i < 42; i++) {
      var d = D.add(start, i), date = D.iso(d);
      var cls = "dp-d" + (d.getMonth() !== first.getMonth() ? " is-out" : "") + (shown(date) ? " is-in" : "") + (date === TODAY ? " is-today" : "");
      cellsHtml += '<button class="' + h(cls) + '" type="button" data-date="' + h(date) + '" aria-label="' + h(KnowluFmt.date(d)) + '">' + h(d.getDate()) + "</button>";
    }
    var headHtml = '<div class="dp-head"><p class="dp-m">' + h(MO[first.getMonth()] + " " + first.getFullYear()) + "</p>" +
      '<button class="obtn ico" type="button" data-month="-1" aria-label="Previous month">' + PREV_SVG + "</button>" +
      '<button class="obtn ico" type="button" data-month="1" aria-label="Next month">' + NEXT_SVG + "</button></div>";
    var namesHtml = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(function (w) { return '<span class="dp-w">' + h(w) + "</span>"; }).join("");
    $("cal-datepop").innerHTML = headHtml + '<div class="dp-grid">' + namesHtml + cellsHtml + "</div>";
  }
  function openPop() {
    closePops();
    POP_MONTH = new Date(FROM.getFullYear(), FROM.getMonth(), 1);
    drawPop();
    $("cal-datepop").hidden = false;
    $("cal-title").setAttribute("aria-expanded", "true");
    var cur = $("cal-datepop").querySelector(".dp-d.is-in") || $("cal-datepop").querySelector(".dp-d");
    if (cur) { cur.focus(); }
  }
  function toggleMenu(btnId, menuId) {
    var open = $(menuId).hidden;
    closePops();
    if (!open) { return; }
    $(menuId).hidden = false;
    $(btnId).setAttribute("aria-expanded", "true");
  }
  function closePops() {
    [["cal-title", "cal-datepop"], ["cal-view", "cal-viewset"], ["cal-ns", "cal-nsmenu"]].forEach(function (p) {
      var b = $(p[0]), m = $(p[1]);
      if (m) { m.hidden = true; }
      if (b) { b.setAttribute("aria-expanded", "false"); }
    });
  }
  function anyPop() { return ["cal-datepop", "cal-viewset", "cal-nsmenu"].some(function (id) { return $(id) && !$(id).hidden; }); }

  /* ---- Details for a block ---- */
  function thingHtml(t) {
    var d = D.day(t.date);
    var rowsHtml = '<div class="det-r"><dt>When</dt><dd>' + h((d ? KnowluFmt.date(d) + ", " : "") + rangeText(t.s, t.e)) + "</dd></div>";
    if (t.where) {
      rowsHtml += '<div class="det-r"><dt>Where</dt><dd>' + h(t.where) + "</dd></div>";
    }
    var course = t.slot ? KnowluCourses.match(t.course) : null;
    if (course) {
      rowsHtml += '<div class="det-r"><dt>Class</dt><dd>' + KnowluKit.markHtml(course.slug) + h(course.title) + "</dd></div>";
    }
    var kind = t.src === "busy" ? "From your calendar" : (KIND_WORD[t.kind] || "Commitment") + (LEVEL_WORD[t.level] ? ", " + LEVEL_WORD[t.level].toLowerCase() : "");
    rowsHtml += '<div class="det-r"><dt>Calendar</dt><dd>' + h(kind) + "</dd></div>";
    return '<dl class="det">' + rowsHtml + "</dl>";
  }
  KnowluPanel.add("cal-thing", {
    render: function (body, key, api) {
      var t = Object.prototype.hasOwnProperty.call(THINGS, key) ? THINGS[key] : null;
      if (!t) { api.close(); return; }
      api.title(t.title);
      body.innerHTML = thingHtml(t);
    }
  });

  /* ---- input ---- */
  function onClick(e) {
    var t = e.target;
    if (!t.closest) { return; }
    if (t.closest("#cal-prev")) { shift(-(N === 7 ? 7 : N)); return; }
    if (t.closest("#cal-next")) { shift(N === 7 ? 7 : N); return; }
    if (t.closest("#cal-today")) { goToday(); return; }
    if (t.closest("#cal-title")) {
      if ($("cal-datepop").hidden) { openPop(); } else { closePops(); }
      return;
    }
    if (t.closest("#cal-view")) { toggleMenu("cal-view", "cal-viewset"); return; }
    if (t.closest("#cal-ns")) { toggleMenu("cal-ns", "cal-nsmenu"); return; }
    var n = t.closest("#cal-viewset [data-n]");
    if (n) { closePops(); setN(+n.getAttribute("data-n")); return; }
    var mo = t.closest("#cal-datepop [data-month]");
    if (mo) { POP_MONTH = new Date(POP_MONTH.getFullYear(), POP_MONTH.getMonth() + +mo.getAttribute("data-month"), 1); drawPop(); return; }
    var dp = t.closest("#cal-datepop [data-date]");
    if (dp) { goTo(D.day(dp.getAttribute("data-date"))); $("cal-title").focus(); return; }
    if (t.closest("#due-fold")) { setFold(!FOLDED); return; }
    var nm = t.closest(".due-nm[data-det]");
    if (nm) { KnowluPanel.open("task", nm.getAttribute("data-det")); return; }
    var blk = t.closest(".blk-b[data-cal]");
    if (blk) { KnowluPanel.open("cal-thing", blk.getAttribute("data-cal")); }
  }
  function onPointerDown(e) {
    if (!anyPop() || !e.target.closest) { return; }
    if (!e.target.closest(".menu-wrap") && !e.target.closest("#cal-datepop") && !e.target.closest("#cal-title")) { closePops(); }
  }
  /* the keys are the grid's only: never while focus is in a text field, the right panel, the sheet or the layer, or
     anywhere outside #center but the page itself (nothing focused) */
  function elsewhere(t) {
    if (!t || t === document || t === document.body || t === document.documentElement) { return false; }
    if (!t.closest) { return true; }
    if (t.isContentEditable || t.closest("input, textarea, select, #panel, #sheet, #layer")) { return true; }
    return !t.closest("#center");
  }
  function onKey(e) {
    if (!MOUNTED || e.defaultPrevented || e.altKey || e.metaKey || elsewhere(e.target)) { return; }
    if (KnowluSheets.isOpen()) { return; }
    var k = e.key;
    if (k === "Escape") {
      if (anyPop()) { closePops(); }
      return;
    }
    if (e.ctrlKey) {
      if (k === "=" || k === "+") { e.preventDefault(); stepZoom(1); }
      else if (k === "-") { e.preventDefault(); stepZoom(-1); }
      else if (k === "0") { e.preventDefault(); setZoom("m"); }
      return;
    }
    if (e.shiftKey && (k === "ArrowLeft" || k === "ArrowRight")) {
      e.preventDefault();
      shift(k === "ArrowLeft" ? -1 : 1);
      return;
    }
    switch (k.length === 1 ? k.toLowerCase() : k) {
      case "t": goToday(); break;
      case "j": shift(N === 7 ? 7 : N); break;
      case "k": shift(-(N === 7 ? 7 : N)); break;
      case "g": e.preventDefault(); openPop(); break;
      default:
        if (k.length === 1 && k >= "1" && k <= "7") { setN(+k); }
    }
  }
  /* sideways pan: a sideways wheel (or Shift with the wheel) moves by whole days */
  function onWheel(e) {
    var dx = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : (e.shiftKey ? e.deltaY : 0);
    if (!dx) { return; }
    e.preventDefault();
    var day = document.querySelector("#cal-wk .day");
    var colW = day ? day.getBoundingClientRect().width : 120;
    WHEEL += dx;
    if (Math.abs(WHEEL) >= colW / 2) {
      shift(WHEEL > 0 ? 1 : -1);
      WHEEL = 0;
    }
  }

  KnowluRouter.add("calendar", {
    mount: function (center, ctx) {
      D = window.KnowluCalData;
      CENTER = center;
      MOUNTED = true;
      S = ctx && ctx.state ? ctx.state : null;
      NS = null;
      WEEK = null;
      readStored();
      center.innerHTML = SKELETON_HTML;
      N = firstDays(center);
      FROM = anchorFor(N, today());
      $("cal").addEventListener("click", onClick);
      document.addEventListener("keydown", onKey);
      document.addEventListener("pointerdown", onPointerDown);
      center.addEventListener("wheel", onWheel, { passive: false });
      if (UNPOLL) { UNPOLL(); }
      UNPOLL = KnowluData.onPoll(nowMoved);
      draw();
      scrollToDay();
      loadWeek();
      loadNotShown();
    },
    update: function (st) {
      S = st;
      if (!MOUNTED) { return; }
      if (S && todayIso() !== WEEK_DAY) { loadWeek(); }
      draw();
    },
    unmount: function () {
      MOUNTED = false;
      if (UNPOLL) { UNPOLL(); }
      UNPOLL = null;
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPointerDown);
      if (CENTER) { CENTER.removeEventListener("wheel", onWheel); }
      CENTER = null;
      S = null;
      THINGS = Object.create(null);
    }
  });
})();
