/* Knowlu app, a class's page (T13: note §4 "Class page", D6, M7, N7; DESIGN.md "A class's page").

   #class/<slug>, view "all", plus your_week (read on mount). What it draws, all of it the read model's:
     the head    the class's mark (its slot), its code (the entry's title) and its name, from S.courses; a class with no
                 entry shows its slug and an ink mark.
     Due         that class's open rows of S.list.rows (matched the engine's way, KnowluCourses.match), soonest first
                 (M7), each with the kit's check, its name (opens Details), its minutes and its due. The line: how many
                 and the next one ("2 · next tonight 11:59 PM", "0").
     Grade       the class's S.grades entry as text through KnowluFmt.grade ("95.2% A", "No scores"), when it was read,
                 and Hide; a press on the figure (or Breakdown) opens the breakdown in the right panel: every entry in
                 the engine's order, read-only, and a foot that says where the number came from, with Refresh.
                 With no entry while grades cannot be read (grades_status: not available at the school, or not
                 connected), the line is blank and the body is one line pointing to Settings > Sources: "No scores" is
                 said only of an entry, or of a source that is connected. A poll redraws the Grade only when the hidden
                 flag, grades_status or this class's entry changed, and the breakdown only when its entry did.
                 Hidden (grades_status), the line reads "Hidden" and the body is Show alone: no figure anywhere. Hide
                 and Show go through set_settings {patch: {grades_hidden}}, then grades_status is read again; neither is
                 ever on an undo list. Nothing is drawn until grades_status has answered.
     Meets       your_week's commitments and office-hours proposals whose course matches this class (N7), less any that
                 has ended: each with its kind, where and when, in the engine's own words.
   Three columns from 1000px of center (Due, Grade, Meets), one below (D6). Writes nothing of its own: the kit writes a
   check; set_settings and grades_refresh are the app's settings and capture commands, never a note. */
(function () {
  "use strict";

  var STATUS_WORDS = { "graded": "graded", "needs-grading": "needs grading", "in-progress": "in progress", "not-submitted": "not submitted", "exempt": "exempt" };
  var KIND_WORDS = { "class": "Class", "lab": "Lab", "office-hours": "Office hours" };
  var SHOW_HTML = '<div class="cp-gl"><button class="obtn" type="button" data-grades-show>Show</button></div>';
  var HIDE_HTML = '<button class="obtn cp-ghide" type="button" data-grades-hide>Hide</button>';
  var MORE_HTML = '<div class="cp-gl cp-gmore"><button class="obtn" type="button" data-det data-grades-open>Breakdown</button></div>';
  var FOOT_HTML = '<p class="cp-foot"><a class="tbtn" href="#tasks">See all in Tasks</a></p>';
  var LAYOUT_HTML = '<div class="cp"><div class="cp-in">' +
    '<header class="cp-head"><p class="cp-meta"><i class="mk" aria-hidden="true"></i><span class="cp-code" id="cp-code"></span></p>' +
    '<h1 class="cp-title" id="cp-title"></h1></header>' +
    '<div class="cp-cols">' +
    '<section class="cp-sec cp-due" aria-labelledby="cp-due-h"><div class="cp-line"><h2 class="cp-nm" id="cp-due-h">Due</h2><p class="cp-fig" id="cp-due-fig"></p></div>' +
    '<div class="cp-body" id="cp-due"></div></section>' +
    '<section class="cp-sec cp-grade" aria-labelledby="cp-grade-h"><div class="cp-line"><h2 class="cp-nm" id="cp-grade-h">Grade</h2><p class="cp-fig" id="cp-grade-fig"></p></div>' +
    '<div class="cp-body" id="cp-grade"></div></section>' +
    '<section class="cp-sec cp-meets" aria-labelledby="cp-meets-h"><div class="cp-line"><h2 class="cp-nm" id="cp-meets-h">Meets</h2><p class="cp-fig" id="cp-meets-fig"></p></div>' +
    '<div class="cp-body" id="cp-meets"></div></section>' +
    "</div></div></div>";

  var ROOT = null, ST = null, SLUG = "", WEEK = null, WEEK_READ = false, NOTE = "", UNPOLL = null, SEQ = 0;
  var GRADES_OPEN = null;   /* the slug whose breakdown the panel shows, else null */
  var GRADE_SIG = "", BREAKDOWN_SIG = "";   /* what the Grade and the breakdown were last drawn from */

  function $(id) { return document.getElementById(id); }
  function has(o, k) { return Object.prototype.hasOwnProperty.call(o, k); }

  /* ---- grades ---- */
  /* grades_status's word: true or false once it has answered, null before (or when it could not be read) */
  function hiddenFlag() {
    var g = KnowluData.last("grades_status");
    if (!g || g.ok === false) { return null; }
    return !!g.hidden;
  }
  function gradeOf(st, slug) {
    var list = st && Array.isArray(st.grades) ? st.grades : [];
    for (var i = 0; i < list.length; i++) { if (list[i].course === slug) { return list[i]; } }
    return null;
  }
  /* grades cannot be read for this class: no entry, and the source is not available and connected */
  function unread(st, slug) {
    var s = KnowluData.last("grades_status");
    return !gradeOf(st, slug) && !(s && s.available && s.connected);
  }
  function gradeText(st, slug) {
    var hidden = hiddenFlag();
    if (hidden === null) { return ""; }
    if (unread(st, slug)) { return ""; }
    return KnowluFmt.grade(gradeOf(st, slug), hidden);
  }
  /* one line that says why there is no grade and where to fix it; the shipped words for each case */
  function sourcesHtml() {
    var s = KnowluData.last("grades_status");
    var say = s && s.available ? "Connect Blackboard to see your grades" : "Grades from Blackboard are not available at your school yet";
    return '<p class="cp-gsrc"><span>' + h(say) + '</span><a class="tbtn" href="#settings/sources">' + h("Settings > Sources") + "</a></p>";
  }
  function readText(at) {
    var d = at ? new Date(at) : null;
    return d && !isNaN(d.getTime()) ? KnowluFmt.read(d) : "";
  }
  function noteHtml() {
    return (NOTE ? '<p class="cp-gnote">' + h(NOTE) + "</p>" : "");
  }
  function gradeBigHtml(g) {
    var read = g ? readText(g.fetched_at) : "";
    var fig = KnowluFmt.grade(g, false);
    var figHtml = (g ? '<button class="cp-gbig" type="button" data-det data-grades-open>' + h(fig) + "</button>" : '<p class="cp-gbig is-none">' + h(fig) + "</p>");
    return '<div class="cp-gcol">' + figHtml + (read ? '<p class="cp-gread">' + h(read) + "</p>" : "") + "</div>";
  }
  function gradeBodyHtml(st) {
    var hidden = hiddenFlag();
    if (hidden === null) { return ""; }
    if (unread(st, SLUG)) { return sourcesHtml(); }
    if (hidden) { return SHOW_HTML; }
    var g = gradeOf(st, SLUG);
    return '<div class="cp-gfig">' + gradeBigHtml(g) + HIDE_HTML + "</div>" + (g ? MORE_HTML : "") + noteHtml();
  }

  /* Hide and Show: the app's one setting, then grades_status again. Never on an undo list. */
  function setHidden(on, btn) {
    btn.disabled = true;
    KnowluData.call("set_settings", { patch: { grades_hidden: on } }).then(function (env) {
      if (!env || !env.ok) { KnowluKit.toast(KnowluKit.refusal(env), null, null, true); }
      return KnowluData.call("grades_status", {});
    }).then(function () {
      btn.disabled = false;
      drawGrade();
      refreshBreakdown();
      var next = ROOT ? ROOT.querySelector(on ? "[data-grades-show]" : "[data-grades-hide]") : null;
      if (next) { next.focus(); }
    });
  }

  /* ---- the breakdown (the right panel; read-only) ---- */
  function statusWord(s) { return has(STATUS_WORDS, s) ? STATUS_WORDS[s] : String(s == null ? "" : s); }
  function dueText(x) {
    var p = KnowluFmt.parse(x);
    return p ? KnowluFmt.date(p.d) : "";
  }
  function entryHtml(e) {
    var score = (e.score == null ? "–" : e.score) + " / " + (e.possible == null ? "–" : e.possible);
    var sub = [e.category || "", dueText(e.due)].filter(function (x) { return !!x; }).join(" · ");
    return '<li class="gb-r">' +
      '<span class="gb-nm">' + h(e.title) + (e.counts === false ? '<span class="gb-nc">not counted</span>' : "") + "</span>" +
      '<span class="gb-sc">' + h(score) + "</span>" +
      '<span class="gb-st st-' + h(e.status) + '">' + h(statusWord(e.status)) + "</span>" +
      (sub ? '<span class="gb-sub">' + h(sub) + "</span>" : "") + "</li>";
  }
  function breakdownFootHtml(g) {
    var pts = g.earned + " / " + g.possible, src;
    if (g.basis === "overall") { src = "Overall Grade" + (g.possible > 0 ? " · points so far " + pts : ""); }
    else if (g.basis === "points") { src = "points so far " + pts; }
    else { src = "nothing graded yet"; }
    var read = readText(g.fetched_at);
    return '<div class="gb-foot"><p class="gb-src">' + h("From Blackboard · " + src) + "</p>" +
      (read ? '<p class="gb-read">' + h(KnowluFmt.read(new Date(g.fetched_at))) + "</p>" : "") +
      KnowluKit.owedHtml('<button class="obtn gb-refresh" type="button" data-start data-grades-refresh' + KnowluKit.owed() + ">Refresh</button>") +
      noteHtml() + "</div>";
  }
  function breakdownHtml(st, slug) {
    var hidden = hiddenFlag();
    var g = gradeOf(st, slug);
    if (hidden !== false || !g) {
      return '<p class="gb-none">' + h(hidden === false ? KnowluFmt.grade(null, false) : (hidden ? "Hidden" : "")) + "</p>";
    }
    return '<p class="gb-fig">' + h(KnowluFmt.grade(g, false)) + "</p>" +
      '<p class="gb-counts">' + h(g.graded + " graded · " + g.pending + " waiting") + "</p>" +
      '<ul class="gb-list" aria-label="Scores">' + (g.entries || []).map(entryHtml).join("") + "</ul>" +
      breakdownFootHtml(g);
  }
  KnowluPanel.add("grades", {
    render: function (body, slug, api) {
      GRADES_OPEN = slug;
      BREAKDOWN_SIG = breakdownSig(slug);
      var k = KnowluCourses.of(slug);
      api.title(k ? k.title : slug);
      body.innerHTML = breakdownHtml(ST || KnowluData.state(), slug);
    },
    close: function () { GRADES_OPEN = null; }
  });
  /* a repaint redraws the breakdown only while the panel shows it: opening anything else closes it first, which
     clears the marker, so a repaint never takes a note's panel */
  function breakdownSig(slug) {
    return JSON.stringify([hiddenFlag(), gradeOf(ST || KnowluData.state(), slug), NOTE]);
  }
  function refreshBreakdown() {
    if (!GRADES_OPEN || !KnowluPanel.isOpen()) { return; }
    if (breakdownSig(GRADES_OPEN) === BREAKDOWN_SIG) { return; }
    KnowluPanel.open("grades", GRADES_OPEN);
  }
  /* Refresh: the capture, then grades_status, then the poll; its outcome is a quiet line, never an error state */
  function refreshGrades(btn) {
    btn.disabled = true;
    NOTE = "";
    KnowluData.call("grades_refresh", {}).then(function (r) {
      NOTE = r && r.ok === false ? KnowluKit.refusal(r) : "";
      return KnowluData.call("grades_status", {});
    }).then(function () {
      KnowluData.refresh();
      drawGrade();
      refreshBreakdown();
    });
  }
  document.addEventListener("click", function (e) {
    var b = e.target.closest ? e.target.closest("#panel [data-grades-refresh]") : null;
    if (b && !b.disabled) { refreshGrades(b); }
  });

  /* ---- Due ---- */
  /* that class's open rows, soonest first; undated last; a tie keeps the engine's order */
  function dueRows(st) {
    var rows = st && st.list && Array.isArray(st.list.rows) ? st.list.rows : [];
    var mine = [];
    rows.forEach(function (r, i) {
      var k = KnowluCourses.match(r.course);
      var ours = k ? k.slug === SLUG : (r.course != null && r.course === SLUG);
      if (r.status !== "done" && ours) { mine.push({ r: r, i: i }); }
    });
    mine.sort(function (a, b) {
      var an = typeof a.r.due === "string" && a.r.due ? 0 : 1, bn = typeof b.r.due === "string" && b.r.due ? 0 : 1;
      if (an !== bn) { return an - bn; }
      if (!an && a.r.due !== b.r.due) { return a.r.due < b.r.due ? -1 : 1; }
      return a.i - b.i;
    });
    return mine.map(function (x) { return x.r; });
  }
  function lower1(s) { return /^(Today|Tonight|Tomorrow|Yesterday)\b/.test(s) ? s.charAt(0).toLowerCase() + s.slice(1) : s; }
  function dueFig(rows) {
    if (!rows.length) { return "0"; }
    return rows[0].due ? rows.length + " · next " + lower1(KnowluFmt.due(rows[0].due)) : String(rows.length);
  }
  function dueRowHtml(r) {
    var slot = KnowluCourses.slot(KnowluCourses.match(r.course));
    return '<li class="cp-dr' + (slot ? " " + h(slot) : "") + '" data-id="' + h(r.id) + '" data-kind="task">' +
      KnowluKit.checkHtml(r) +
      '<button class="cp-dnm" type="button" data-det data-open="' + h(r.id) + '">' + h(r.title) + "</button>" +
      '<span class="cp-dmin">' + h(KnowluFmt.minutes((Number(r.hours) || 0) * 60)) + "</span>" +
      '<span class="cp-ddue">' + h(KnowluFmt.due(r.due)) + "</span></li>";
  }
  function dueHtml(rows) {
    return (rows.length ? '<ul class="cp-dl" aria-label="Due" data-rows>' + rows.map(dueRowHtml).join("") + "</ul>" : "") + FOOT_HTML;
  }

  /* ---- Meets ---- */
  function meetsOf(week, st) {
    if (!week) { return []; }
    var today = st && st.now ? String(st.now).slice(0, 10) : "";
    var list = (Array.isArray(week.commitments) ? week.commitments : []).concat(Array.isArray(week.office_hours) ? week.office_hours : []);
    return list.filter(function (cm) {
      var k = KnowluCourses.match(cm.course);
      var ours = k ? k.slug === SLUG : (cm.course != null && cm.course === SLUG);
      var ended = !!cm.until && !!today && String(cm.until).slice(0, 10) < today;
      return ours && !ended;
    });
  }
  function meetHtml(m) {
    var label = has(KIND_WORDS, m.kind) ? KIND_WORDS[m.kind] : (m.title || "");
    return '<li class="cp-mt"><span class="cp-mlb">' + h(label) + '</span><span class="cp-mwh">' + h(m.where) + "</span>" +
      '<span class="cp-mwn">' + h(m.when) + "</span></li>";
  }

  /* ---- the page ---- */
  function drawHead() {
    var k = KnowluCourses.of(SLUG);
    var slot = k ? KnowluCourses.slot(k) : "";
    ROOT.querySelector(".cp-meta").className = "cp-meta" + (slot ? " " + slot : "");
    $("cp-code").textContent = k ? k.title : SLUG;
    $("cp-title").textContent = k ? (k.name || k.title) : "";
  }
  function drawGrade() {
    if (!ROOT) { return; }
    var sig = JSON.stringify([hiddenFlag(), KnowluData.last("grades_status"), ST ? gradeOf(ST, SLUG) : null, NOTE, !!ST]);
    if (sig === GRADE_SIG) { return; }
    GRADE_SIG = sig;
    $("cp-grade-fig").textContent = ST ? gradeText(ST, SLUG) : "";
    $("cp-grade").innerHTML = (ST ? gradeBodyHtml(ST) : "");
  }
  function drawMeets() {
    if (!ROOT) { return; }
    var list = meetsOf(WEEK, ST);
    $("cp-meets-fig").textContent = !WEEK_READ || !WEEK ? "" : (list.length ? String(list[0].when || "") : "None");
    $("cp-meets").innerHTML = (list.length ? '<ul class="cp-ml" aria-label="Meets">' + list.map(meetHtml).join("") + "</ul>" : "");
  }
  function draw() {
    if (!ROOT) { return; }
    drawHead();
    var rows = ST ? dueRows(ST) : [];
    $("cp-due-fig").textContent = ST ? dueFig(rows) : "";
    $("cp-due").innerHTML = (ST ? dueHtml(rows) : "");
    drawGrade();
    drawMeets();
  }
  function readWeek() {
    var seq = SEQ;
    KnowluData.call("your_week", {}).then(function (r) {
      if (!ROOT || seq !== SEQ) { return; }
      WEEK = r && r.ok && r.week ? r.week : null;
      WEEK_READ = true;
      drawMeets();
    });
  }

  function onClick(e) {
    var t = e.target;
    var hide = t.closest("[data-grades-hide]");
    if (hide) { setHidden(true, hide); return; }
    var show = t.closest("[data-grades-show]");
    if (show) { setHidden(false, show); return; }
    if (t.closest("[data-grades-open]")) { KnowluPanel.open("grades", SLUG); return; }
    var open = t.closest("[data-open]");
    if (open) { KnowluPanel.open("task", open.getAttribute("data-open")); }
  }

  KnowluRouter.add("class", {
    mount: function (center, ctx) {
      center.innerHTML = LAYOUT_HTML;
      ROOT = center.querySelector(".cp");
      SEQ += 1;
      SLUG = ctx.arg || "";
      ST = ctx.state;
      WEEK = null;
      WEEK_READ = false;
      NOTE = "";
      GRADE_SIG = "";
      ROOT.addEventListener("click", onClick);
      draw();
      readWeek();
      UNPOLL = KnowluData.onPoll(function () { if (ROOT && ST) { drawGrade(); } });
    },
    update: function (st) {
      ST = st;
      draw();
      refreshBreakdown();
    },
    unmount: function () {
      if (UNPOLL) { UNPOLL(); }
      UNPOLL = null;
      ROOT = null;
      ST = null;
      SEQ += 1;
    }
  });
})();
