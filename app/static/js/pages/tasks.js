/* Knowlu app: Tasks (v0.4.0 note §4 "Tasks", plan §5 T10). The all view.

   Open    S.list.rows, grouped Late / Today / This week / Later / No date by the row's due against S.now (the one
           date rule, fmt.js); inside a group the rows keep the sort's order, which is the engine's own unless the
           student picks another. S.list.unreadable is a line above the list, whatever the tab (I1).
   Done    S.closed_this_week, with Undo only on a row whose undo the engine set (set_fields status active).
   All     both, Done last.
   Columns check, Task, Class, Due, Estimate. No Priority column (ruling 7) and no Type: no row carries a type.
   Sort    priority (the engine's order, the default), due date, name, class, estimate.
   Filters search, class and due are the page's own and never stored; #tasks/<due filter> (late, today, week,
           none) names the due filter, as Waiting's "Needs a date" link does.
   New     create_task {fields: {title, course, due, effort_hours}}; data.js adds the view and adopts the answer.
   A row's check is the kit's one control (KnowluKit.checkHtml); its name, or a press anywhere on the row, opens
   Details (KnowluPanel "task", T14). */
(function () {
  "use strict";
  var GROUPS = ["Late", "Today", "This week", "Later", "No date"];
  var WEEK_DAYS = 6;
  var WHO = { me: "You", agent: "Knowlu", system: "Knowlu", lapsed: "Lapsed" };
  var CHEVRON = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 6l4 4 4-4"/></svg>';
  var PLUS = '<svg viewBox="0 0 18 18" aria-hidden="true"><path d="M9 3.5v11M3.5 9h11"/></svg>';
  var LOUPE = '<svg viewBox="0 0 18 18" aria-hidden="true"><path d="M7.8 13.1a5.3 5.3 0 1 0 0-10.6 5.3 5.3 0 0 0 0 10.6zM11.6 11.6l3.9 3.9"/></svg>';
  var DONE_MARK = '<span class="ck tk-done" aria-hidden="true"><svg viewBox="0 0 22 22"><circle cx="11" cy="11" r="10"/><path d="M6.6 11.4l3 3 5.8-6.2"/></svg></span>';
  var NO_DATE_HTML = '<span class="tk-due tk-none">No date</span>';
  var CLEAR_HTML = '<button class="tbtn" type="button" data-tk-clear>Clear filters</button>';
  var HEAD_HTML = '<li class="tk-cols" aria-hidden="true"><span></span><span>Task</span><span>Class</span><span>Due</span><span class="tk-num">Estimate</span></li>';
  var SORT_HTML = '<option value="rank">Sort by priority</option><option value="due">Sort by due date</option><option value="name">Sort by name</option><option value="class">Sort by class</option><option value="estimate">Sort by estimate</option>';
  var DUE_HTML = '<option value="">Any due date</option><option value="late">Late</option><option value="today">Today</option><option value="week">This week</option><option value="none">No date</option>';
  var DUE_ARGS = ["late", "today", "week", "none"];
  var TABS_HTML = '<div class="seg tk-tabs" role="radiogroup" aria-label="Show">' +
    '<button class="obtn" type="button" role="radio" data-tab="open" aria-checked="true">Open</button>' +
    '<button class="obtn" type="button" role="radio" data-tab="done" aria-checked="false">Done</button>' +
    '<button class="obtn" type="button" role="radio" data-tab="all" aria-checked="false">All</button></div>';
  var FORM_HTML = '<form class="tk-form" data-tk-form hidden autocomplete="off"><input class="field tk-f-title" name="title" type="text" maxlength="200" placeholder="Task name" aria-label="Task name" required><span class="sel tk-f-cls"><select name="course" aria-label="Class"></select>' + CHEVRON + '</span><input class="field tk-f-due" name="due" type="date" aria-label="Due date"><input class="field tk-f-est" name="effort_hours" type="number" min="0" step="0.5" value="1" aria-label="Estimate in hours"><span class="tk-f-acts"><button class="ibtn" type="submit">Add</button><button class="qbtn" type="button" data-tk-cancel>Cancel</button></span></form>';
  var PAGE_HTML = '<div class="page tk">' +
    '<div class="tk-head"><h1 class="page-h">Tasks</h1><button class="obtn tk-new" type="button" data-tk-new aria-expanded="false">' + PLUS + "New task</button></div>" +
    FORM_HTML +
    '<div class="tk-bar" role="search" aria-label="Find tasks">' + TABS_HTML +
    '<label class="tk-search"><span class="vh">Search tasks</span>' + LOUPE + '<input class="field" type="search" data-tk-q placeholder="Search tasks and classes" spellcheck="false"></label>' +
    '<span class="sel tk-sel"><select data-tk-class aria-label="Class"></select>' + CHEVRON + "</span>" +
    '<span class="sel tk-sel"><select data-tk-due aria-label="Due">' + DUE_HTML + "</select>" + CHEVRON + "</span>" +
    '<span class="sel tk-sel"><select data-tk-sort aria-label="Sort">' + SORT_HTML + "</select>" + CHEVRON + "</span>" +
    '<p class="tk-count" data-tk-count aria-live="polite"></p></div>' +
    '<div class="tk-unread" data-tk-unread hidden></div>' +
    '<ul class="tk-list" data-rows aria-label="Tasks"></ul>' +
    '<div class="tk-empty" data-tk-empty hidden></div></div>';

  var TAB = "open";
  var Q = "";
  var CLS = "";
  var DUE = "";
  var SORT = "rank";
  var ROOT_EL = null;
  var LIST = null;
  var UNREAD = null;
  var EMPTY = null;
  var COUNT = null;
  var COURSE_KEY = null;

  /* ---- the one date rule, by the day: whole days from S.now's day to a due ---- */
  function day0(d) { return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime(); }
  function groupOf(r) {
    var p = KnowluFmt.parse(r.due);
    if (!p) { return 4; }
    var days = Math.round((day0(p.d) - day0(KnowluFmt.now())) / 86400000);
    return days < 0 ? 0 : days === 0 ? 1 : days <= WEEK_DAYS ? 2 : 3;
  }
  function slugOf(r) {
    var entry = KnowluCourses.match(r.course);
    return entry ? entry.slug : String(r.course || "");
  }
  function classWords(r) {
    var entry = KnowluCourses.match(r.course);
    return entry ? String(entry.title || entry.slug) : String(r.course || "");
  }

  /* ---- filters and sorts: the page's own, never stored ---- */
  function keep(r) {
    if (CLS && slugOf(r) !== CLS) { return false; }
    var g = groupOf(r);
    if (DUE === "late" && g !== 0) { return false; }
    if (DUE === "today" && g !== 1) { return false; }
    if (DUE === "week" && g !== 1 && g !== 2) { return false; }
    if (DUE === "none" && g !== 4) { return false; }
    if (Q) {
      var hay = (String(r.title || "") + " " + classWords(r)).toLowerCase();
      if (hay.indexOf(Q) < 0) { return false; }
    }
    return true;
  }
  /* a close carries no class and no due, so a class or due filter leaves it out */
  function keepClosed(k) {
    if (CLS || DUE) { return false; }
    return !Q || String(k.title || "").toLowerCase().indexOf(Q) >= 0;
  }
  function dueKey(r) { return r.due ? String(r.due) : "9999"; }
  function cmp(a, b) {
    if (SORT === "due") { return dueKey(a) < dueKey(b) ? -1 : dueKey(a) > dueKey(b) ? 1 : 0; }
    if (SORT === "name") { return String(a.title || "").localeCompare(String(b.title || "")); }
    if (SORT === "class") { return classWords(a).localeCompare(classWords(b)); }
    if (SORT === "estimate") { return (Number(a.hours) || 0) - (Number(b.hours) || 0); }
    return 0;
  }
  function sorted(rows) {
    if (SORT === "rank") { return rows.slice(); }
    var keyed = rows.map(function (r, i) { return { r: r, i: i }; });
    keyed.sort(function (a, b) { return cmp(a.r, b.r) || a.i - b.i; });
    return keyed.map(function (k) { return k.r; });
  }

  /* ---- rows ---- */
  function classHtml(r) {
    var entry = KnowluCourses.match(r.course);
    if (entry) {
      return '<span class="tk-cls"><a href="#class/' + h(encodeURIComponent(entry.slug)) + '">' + KnowluKit.markHtml(r.course) + "<span>" + h(entry.title || entry.slug) + "</span></a></span>";
    }
    return (r.course ? '<span class="tk-cls">' + KnowluKit.markHtml(r.course) + "<span>" + h(r.course) + "</span></span>" : '<span class="tk-cls tk-none">None</span>');
  }
  function dueHtml(r) {
    var g = groupOf(r);
    if (g === 4) { return NO_DATE_HTML; }
    return '<span class="tk-due">' + (g === 0 ? '<b class="tk-late">Late</b> ' : "") + h(KnowluFmt.due(r.due)) + "</span>";
  }
  function estHtml(r) {
    var hours = Number(r.hours) || 0;
    return (hours > 0 ? '<span class="tk-min">' + h(KnowluFmt.minutes(hours * 60)) + "</span>" : '<span class="tk-min tk-none">&ndash;</span>');
  }
  function rowHtml(r) {
    return '<li class="tk-row hov" data-row data-det="' + h(r.id) + '" data-id="' + h(r.id) + '" data-kind="task">' + KnowluKit.checkHtml(r) +
      '<button class="tk-nm" type="button" data-det="' + h(r.id) + '">' + h(r.title) + "</button>" + classHtml(r) + dueHtml(r) + estHtml(r) + "</li>";
  }
  function closedHtml(k) {
    var who = Object.prototype.hasOwnProperty.call(WHO, k.mark) ? WHO[k.mark] : "";
    return '<li class="tk-row is-closed" data-id="' + h(k.id || "") + '" data-kind="task">' + DONE_MARK + '<span class="tk-nm">' + h(k.title) + "</span>" +
      '<span class="tk-cls tk-who">' + h(who) + '</span><span class="tk-due">' + h(KnowluFmt.done(k.when)) + "</span>" +
      (k.undo && k.id ? '<span class="tk-min"><button class="tbtn" type="button" data-undo-close="' + h(k.id) + '">Undo</button></span>' : '<span class="tk-min"></span>') + "</li>";
  }
  function groupHtml(label) {
    return '<li class="tk-grp"><span>' + h(label) + "</span></li>";
  }
  function openHtml(rows) {
    var parts = [];
    GROUPS.forEach(function (label, g) {
      var inGroup = rows.filter(function (r) { return groupOf(r) === g; });
      if (!inGroup.length) { return; }
      parts.push(groupHtml(label));
      inGroup.forEach(function (r) { parts.push(rowHtml(r)); });
    });
    return parts.join("");
  }
  function pathHtml(p) {
    return "<li>" + h(p) + "</li>";
  }
  function unreadHtml(paths) {
    return '<p class="tk-unread-h">' + h(paths.length === 1 ? "1 task note can't be read" : paths.length + " task notes can't be read") + "</p>" +
      '<ul class="tk-unread-l">' + paths.map(pathHtml).join("") + "</ul>";
  }
  function emptyHtml(text, filtered) {
    return '<p class="tk-empty-h">' + h(text) + "</p>" + (filtered ? CLEAR_HTML : "");
  }
  function optionHtml(entry) {
    return '<option value="' + h(entry.slug) + '">' + h(entry.title || entry.slug) + "</option>";
  }

  /* ---- the class lists: every course entry, redrawn only when the entries change ---- */
  function drawClasses() {
    var all = KnowluCourses.all();
    var key = JSON.stringify(all.map(function (e) { return [e.slug, e.title]; }));
    if (key === COURSE_KEY) { return; }
    COURSE_KEY = key;
    var filter = ROOT_EL.querySelector("[data-tk-class]");
    var optionsHtml = all.map(optionHtml).join("");
    filter.innerHTML = '<option value="">All classes</option>' + optionsHtml;
    filter.value = CLS;
    if (filter.value !== CLS) { CLS = ""; filter.value = ""; }
    ROOT_EL.querySelector('.tk-form [name="course"]').innerHTML = '<option value="">No class</option>' + optionsHtml;
  }

  function draw(st) {
    if (!ROOT_EL) { return; }
    ROOT_EL.querySelectorAll("[data-tab]").forEach(function (b) { b.setAttribute("aria-checked", b.getAttribute("data-tab") === TAB ? "true" : "false"); });
    if (!st || !st.list) {
      LIST.textContent = "";
      UNREAD.hidden = true;
      EMPTY.hidden = true;
      COUNT.textContent = "";
      return;
    }
    drawClasses();
    var bad = Array.isArray(st.list.unreadable) ? st.list.unreadable : [];
    UNREAD.hidden = !bad.length;
    UNREAD.innerHTML = bad.length ? unreadHtml(bad) : "";
    var all = Array.isArray(st.list.rows) ? st.list.rows : [];
    var closedAll = Array.isArray(st.closed_this_week) ? st.closed_this_week : [];
    var open = TAB === "done" ? [] : sorted(all.filter(keep));
    var closed = TAB === "open" ? [] : closedAll.filter(keepClosed);
    var filtered = !!(Q || CLS || DUE);
    var openPartHtml = TAB === "done" ? "" : openHtml(open);
    var donePartHtml = closed.length && TAB === "all" ? groupHtml("Done") : "";
    var closedPartHtml = closed.map(closedHtml).join("");
    var listHtml = "";
    if (open.length || closed.length) {
      listHtml = HEAD_HTML + openPartHtml + donePartHtml + closedPartHtml;
    }
    LIST.innerHTML = listHtml;
    var shown = open.length + closed.length;
    COUNT.textContent = shown === 1 ? "1 task" : shown + " tasks";
    var empty = "";
    if (!shown) {
      var none = TAB === "done" ? !closedAll.length : TAB === "open" ? !all.length : !all.length && !closedAll.length;
      if (filtered && !none) { empty = "No tasks match."; } else if (TAB === "done") { empty = st.texts ? st.texts.closed : ""; } else { empty = st.list.empty_text || ""; }
    }
    EMPTY.hidden = !empty;
    EMPTY.innerHTML = empty ? emptyHtml(empty, filtered && empty === "No tasks match.") : "";
    KnowluKit.watchSeen();
  }
  function redraw() { draw(KnowluData.state()); }

  /* ---- writes ---- */
  function undoClose(id, btn) {
    btn.disabled = true;
    KnowluData.call("set_fields", { id: id, fields: { status: "active" } }).then(function (env) {
      if (env && env.ok) {
        KnowluKit.toast("Back on your list");
        return;
      }
      btn.disabled = false;
      KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
    });
  }
  function showForm(on) {
    var form = ROOT_EL.querySelector("[data-tk-form]");
    form.hidden = !on;
    ROOT_EL.querySelector("[data-tk-new]").setAttribute("aria-expanded", on ? "true" : "false");
    if (on) { form.elements.title.focus(); } else { form.reset(); }
  }
  function lockForm(form, on) {
    Array.prototype.forEach.call(form.elements, function (el) { el.disabled = on; });
  }
  function submitNew(form) {
    var title = form.elements.title.value.trim();
    if (!title) { form.elements.title.focus(); return; }
    var est = form.elements.effort_hours.value === "" ? 1 : Number(form.elements.effort_hours.value);
    var fields = { title: title, course: form.elements.course.value || null, due: form.elements.due.value || null, effort_hours: isFinite(est) && est >= 0 ? est : 1 };
    lockForm(form, true);
    KnowluData.call("create_task", { fields: fields }).then(function (env) {
      lockForm(form, false);
      if (env && env.ok) {
        showForm(false);
        KnowluKit.toast("Added");
        return;
      }
      KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
    });
  }
  function openDetails(id) {
    if (id) { KnowluPanel.open("task", id); }
  }
  function clearFilters() {
    Q = "";
    CLS = "";
    DUE = "";
    ROOT_EL.querySelector("[data-tk-q]").value = "";
    ROOT_EL.querySelector("[data-tk-class]").value = "";
    ROOT_EL.querySelector("[data-tk-due]").value = "";
    redraw();
  }

  function bind(root) {
    root.addEventListener("click", function (e) {
      var t = e.target;
      var tab = t.closest("[data-tab]");
      if (tab) { TAB = tab.getAttribute("data-tab"); redraw(); return; }
      var undo = t.closest("[data-undo-close]");
      if (undo) { undoClose(undo.getAttribute("data-undo-close"), undo); return; }
      if (t.closest("[data-tk-new]")) { showForm(root.querySelector("[data-tk-form]").hidden); return; }
      if (t.closest("[data-tk-cancel]")) { showForm(false); return; }
      if (t.closest("[data-tk-clear]")) { clearFilters(); return; }
      var row = t.closest("[data-row]");
      if (!row) { return; }
      var control = t.closest("button, a, input, select, textarea");
      if (control && !control.classList.contains("tk-nm")) { return; }
      openDetails(row.getAttribute("data-id"));
    });
    root.addEventListener("input", function (e) {
      if (e.target.matches("[data-tk-q]")) { Q = e.target.value.trim().toLowerCase(); redraw(); }
    });
    root.addEventListener("change", function (e) {
      var t = e.target;
      if (t.matches("[data-tk-class]")) { CLS = t.value; redraw(); }
      if (t.matches("[data-tk-due]")) { DUE = t.value; redraw(); }
      if (t.matches("[data-tk-sort]")) { SORT = t.value; redraw(); }
    });
    root.addEventListener("submit", function (e) {
      e.preventDefault();
      submitNew(e.target);
    });
    root.addEventListener("keydown", function (e) {
      if (e.key === "Escape" && e.target.closest("[data-tk-form]")) { e.stopPropagation(); showForm(false); root.querySelector("[data-tk-new]").focus(); }
    });
  }

  KnowluRouter.add("tasks", {
    mount: function (center, ctx) {
      center.innerHTML = PAGE_HTML;
      ROOT_EL = center.querySelector(".tk");
      LIST = ROOT_EL.querySelector(".tk-list");
      UNREAD = ROOT_EL.querySelector("[data-tk-unread]");
      EMPTY = ROOT_EL.querySelector("[data-tk-empty]");
      COUNT = ROOT_EL.querySelector("[data-tk-count]");
      COURSE_KEY = null;
      DUE = ctx && DUE_ARGS.indexOf(ctx.arg) >= 0 ? ctx.arg : "";   /* a due link filters this visit only */
      ROOT_EL.querySelector("[data-tk-q]").value = Q ? Q : "";
      ROOT_EL.querySelector("[data-tk-due]").value = DUE;
      ROOT_EL.querySelector("[data-tk-sort]").value = SORT;
      bind(ROOT_EL);
      draw(ctx ? ctx.state : null);
    },
    update: function (st) { draw(st); },
    unmount: function () {
      ROOT_EL = null;
      LIST = null;
      UNREAD = null;
      EMPTY = null;
      COUNT = null;
    }
  });
})();
