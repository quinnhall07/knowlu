/* Knowlu app, the Classes list (T13: note §4 "Classes list", D6, M1, M7; DESIGN.md "The desktop app").

   What it draws, all of it the read model's (view "all"; the page computes nothing):
     every entry of S.courses, in the engine's order (by title): the active ones (status absent or "active", as the
     engine reads it: KnowluCourses.active, M1) open under Current, the rest under Not current, which starts closed.
     A row is the class's mark and code (its slot from the entry), its name (the school's, else the code), Next due
     and the Grade, and opens the class's page (#class/<slug>).
     Next due      the open row of that class in S.list.rows with the soonest due (M7), never the first by rank; a row
                   is matched to a class the engine's way (KnowluCourses.match, N7). It drops below 900px of center (D6).
     Grade         the class's S.grades entry through the one formatter, KnowluFmt.grade: "95.2% A", "No scores", or
                   "Hidden" when grades_status says the student hid grades (ruling 6). Nothing is drawn until
                   grades_status has answered, so a hidden grade never shows for a moment; and nothing for a class with
                   no entry while grades cannot be read (not available at the school, or not connected): "No scores"
                   is said only of an entry, or of a source that is connected.
   A class row carries no data-id: a slug is never an object id, so it never reaches telemetry (I9).
   Sends no command: the poll reads, grades_status rides it (KnowluData.last); a poll redraws the list only when the
   hidden flag or grades_status changed, so focus stays where it is. */
(function () {
  "use strict";

  var CHEV = '<svg class="cl-chev" viewBox="0 0 18 18" aria-hidden="true"><path d="M6.75 4.5L11.25 9l-4.5 4.5"/></svg>';
  var ROW_CHEV = '<svg class="cl-rchev" viewBox="0 0 18 18" aria-hidden="true"><path d="M6.75 4.5L11.25 9l-4.5 4.5"/></svg>';
  var HEAD_HTML = '<li class="cl-row cl-head" role="row"><span role="columnheader">Class</span><span role="columnheader"><span class="vh">Class name</span></span>' +
    '<span class="cl-next" role="columnheader">Next due</span><span class="cl-grade" role="columnheader">Grade</span><span aria-hidden="true"></span></li>';
  var LAYOUT_HTML = '<div class="cl"><div class="cl-in"><h1 class="page-h">Classes</h1><div class="cl-groups" id="cl-groups"></div></div></div>';
  var NONE_HTML = '<p class="cl-none">No classes</p>';

  var ROOT = null, ST = null, UNPOLL = null, SIG = "";
  var OPEN = { cur: true, other: false };

  function $(id) { return document.getElementById(id); }

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
  /* what the poll can change on this page */
  function pollSig() {
    return JSON.stringify([hiddenFlag(), KnowluData.last("grades_status")]);
  }

  /* M7: the soonest due among the class's open rows, whatever their rank */
  function nextDue(st, slug) {
    var rows = st && st.list && Array.isArray(st.list.rows) ? st.list.rows : [];
    var best = null;
    rows.forEach(function (r) {
      if (r.status === "done" || typeof r.due !== "string" || !r.due) { return; }
      var k = KnowluCourses.match(r.course);
      if (!k || k.slug !== slug) { return; }
      if (!best || r.due < best.due) { best = r; }
    });
    return best;
  }

  function nextHtml(st, slug) {
    var r = nextDue(st, slug);
    return (r ? '<span class="cl-nn">' + h(r.title) + '</span><span class="cl-nw">' + h(KnowluFmt.due(r.due)) + "</span>" : "");
  }
  function rowHtml(st, c) {
    var slot = KnowluCourses.slot(c);
    return '<li class="cl-row' + (slot ? " " + h(slot) : "") + '" role="row">' +
      '<span class="cl-code" role="cell"><i class="mk" aria-hidden="true"></i><span>' + h(c.title) + "</span></span>" +
      '<span class="cl-name" role="cell"><a href="' + h("#class/" + c.slug) + '">' + (c.name ? h(c.name) : h(c.title)) + "</a></span>" +
      '<span class="cl-next" role="cell">' + nextHtml(st, c.slug) + "</span>" +
      '<span class="cl-grade" role="cell">' + h(gradeText(st, c.slug)) + "</span>" +
      ROW_CHEV + "</li>";
  }
  function groupHtml(st, key, name, list) {
    var open = !!OPEN[key];
    var count = list.length === 1 ? "1 class" : list.length + " classes";
    return '<section class="cl-group">' +
      '<h2 class="cl-gh"><button class="cl-gbtn" type="button" data-cl-group="' + h(key) + '" aria-expanded="' + (open ? "true" : "false") + '" aria-controls="cl-g-' + h(key) + '">' +
      CHEV + '<span class="cl-gname">' + h(name) + '</span><span class="cl-gfig">' + h(count) + "</span></button></h2>" +
      '<ul class="cl-rows" id="cl-g-' + h(key) + '" role="table" aria-label="' + h(name) + '"' + (open ? "" : " hidden") + ">" +
      HEAD_HTML + list.map(function (c) { return rowHtml(st, c); }).join("") + "</ul></section>";
  }

  function draw() {
    if (!ROOT) { return; }
    SIG = pollSig();
    var st = ST;
    var all = KnowluCourses.all();
    var cur = all.filter(function (c) { return KnowluCourses.active(c); });
    var other = all.filter(function (c) { return !KnowluCourses.active(c); });
    var groupsHtml = "";
    if (st && !all.length) {
      groupsHtml = NONE_HTML;
    } else if (st) {
      groupsHtml = groupHtml(st, "cur", "Current", cur) + (other.length ? groupHtml(st, "other", "Not current", other) : "");
    }
    $("cl-groups").innerHTML = groupsHtml;
  }

  function onClick(e) {
    var b = e.target.closest("[data-cl-group]");
    if (!b) { return; }
    var key = b.getAttribute("data-cl-group");
    OPEN[key] = b.getAttribute("aria-expanded") !== "true";
    b.setAttribute("aria-expanded", OPEN[key] ? "true" : "false");
    var list = $("cl-g-" + key);
    if (list) { list.hidden = !OPEN[key]; }
  }

  KnowluRouter.add("classes", {
    mount: function (center, ctx) {
      center.innerHTML = LAYOUT_HTML;
      ROOT = center.querySelector(".cl");
      ROOT.addEventListener("click", onClick);
      ST = ctx.state;
      draw();
      UNPOLL = KnowluData.onPoll(function () { if (ROOT && ST && pollSig() !== SIG) { draw(); } });
    },
    update: function (st) {
      ST = st;
      draw();
    },
    unmount: function () {
      if (UNPOLL) { UNPOLL(); }
      UNPOLL = null;
      ROOT = null;
      ST = null;
    }
  });
})();
