/* Knowlu app: Details, the task drawer carried into the right panel (v0.4.0 note §5, plan §5 T14). A page opens it
   with KnowluPanel.open("task", id) from any task name.

   It reads the note (`note`), whose row is the engine's own Row for an active task, and shows, in order:
     Class, Needs, Due, then Start by, Slack, Won't fit (with its reason), Why, a pending change's sentence, Progress
     and Estimate; these appear nowhere else (A4)
     More: the rest of the fourteen fields the host accepts (commands.rs EDITABLE; kind and level only on a
     commitment note), then id, source_uid, also_uids and judgment, read-only
     Notes (the body, for a note in tasks/ or courses/), History (the note's journal, newest first)
     Flag (eight categories and free text: open_issue) and Archive (delete_note, behind confirm())
   Click a value to edit it: Enter or blur commits through set_fields, Escape cancels and keeps the panel open; each
   sends edit_started, then edit_committed (on success only) or edit_cancelled, with the field's name. A number out of
   range is refused here with the bound that exists; a progress of 100 is written with status done.
   The body editor saves with set_body {expected: the body last read}; a conflict keeps the draft and offers Copy and
   Reload; a save leaves Saved · Undo for ten seconds, whose expected is the re-read body. The toast belongs to the
   panel: opening another note or closing the panel drops it.
   A newly painted state re-reads the open note by its own id, never another, and never while an editor is open.
   No usage event carries a value, a title or body text. */
(function () {
  "use strict";
  var EDITABLE = ["title", "course", "due", "effort_hours", "importance", "importance_reason", "status", "progress", "slice_hours", "domain", "rank_override", "effort_confidence", "kind", "level"];
  var COMMITMENT_ONLY = ["kind", "level"];
  var READ_ONLY = ["id", "source_uid", "also_uids", "judgment"];
  var NUMERIC_RANGE = { importance: [1, 5, 1], progress: [0, 100, 5], effort_hours: [0, null, 0.5], slice_hours: [0, null, 0.5], rank_override: [null, null, null] };
  var BODY_FOLDERS = { tasks: 1, courses: 1 };
  var TOP = ["course", "due", "progress", "effort_hours"];
  var LABELS = { title: "Name", course: "Class", due: "Due", effort_hours: "Estimate", importance: "Importance", importance_reason: "Reason", status: "Status", progress: "Progress", slice_hours: "Session", domain: "Area", rank_override: "Rank", effort_confidence: "Confidence", kind: "Kind", level: "Level", id: "ID", source_uid: "Source", also_uids: "Also", judgment: "Judgment" };
  var CATS = [["wrong-effort", "Wrong estimate"], ["wrong-course", "Wrong class"], ["duplicate", "Duplicate"], ["should-not-exist", "Shouldn't exist"], ["wrong-tier", "Wrong tier"], ["wrong-date", "Wrong date"], ["wrong-verdict", "Wrong call"], ["other", "Other"]];
  var CHEVRON = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 6l4 4 4-4"/></svg>';
  var NONE_HTML = '<span class="dt-none">None</span>';
  var BODY_ED_HTML = '<h3 class="sec-h">Notes</h3><textarea class="field dt-ta" data-body-ta rows="8" aria-label="Notes"></textarea><div class="det-acts"><button class="ibtn" type="button" data-body-save>Save</button><button class="qbtn" type="button" data-body-cancel>Cancel</button></div>';
  var CONFLICT_HTML = '<p class="dt-conf-p">This note changed since you opened it. Your text is still here. Copy it, then reload to see the new version.</p><div class="det-acts"><button class="tbtn" type="button" data-body-copy>Copy</button><button class="qbtn" type="button" data-body-reload>Reload</button></div>';
  var FLAG_HTML = '<div class="dt-flag" role="group" aria-label="Flag this"><div class="seg dt-cats">' + CATS.map(catHtml).join("") + "</div>" +
    '<textarea class="field dt-flag-t" rows="3" aria-label="What is wrong" placeholder="What is wrong, in your words"></textarea>' +
    '<div class="det-acts"><button class="tbtn" type="button" data-flag-send disabled>Flag</button><button class="qbtn" type="button" data-flag-cancel>Cancel</button></div></div>';

  var CUR = null;   /* the open panel: {id, api, body, note, edit, bodyEd, more} */
  var BOUND = null; /* the panel body the click handler is on */

  function own(table, key) { return Object.prototype.hasOwnProperty.call(table, key); }
  function catHtml(c) {
    return '<button class="obtn" type="button" data-cat="' + h(c[0]) + '" aria-pressed="false">' + h(c[1]) + "</button>";
  }
  function courseOptionHtml(entry) {
    return '<option value="' + h(entry.slug) + '">' + h(entry.title || entry.slug) + "</option>";
  }
  function kindOf(n) {
    if (!n || !n.folder) { return "note"; }
    return n.folder === "tasks" ? "task" : String(n.folder).replace(/s$/, "");
  }
  function rawOf(v) {
    if (v == null) { return ""; }
    return typeof v === "object" ? JSON.stringify(v) : String(v);
  }

  /* the row a page shows for this id, before the note answers: its title and check */
  function rowInState(id) {
    var st = KnowluData.state();
    var found = null;
    var look = function (r) { if (!found && r && r.id === id) { found = r; } };
    if (!st) { return null; }
    if (st.list && Array.isArray(st.list.rows)) { st.list.rows.forEach(look); }
    if (st.must_do && Array.isArray(st.must_do.groups)) { st.must_do.groups.forEach(function (g) { (g.rows || []).forEach(look); }); }
    if (st.recommended && Array.isArray(st.recommended.rows)) { st.recommended.rows.forEach(function (x) { look(x ? x.row : null); }); }
    return found;
  }
  /* the check beside the title: the engine's row, else (a closed task) the note's own fields */
  function checkRow(n) {
    if (n.folder !== "tasks") { return null; }
    if (n.row) { return n.row; }
    var fm = n.frontmatter || {};
    return { id: n.id, course: fm.course, status: fm.status, progress_pct: typeof fm.progress === "number" ? fm.progress : 0, title: fm.title || n.slug };
  }

  /* ---- the rows ---- */
  function slackWords(days) {
    var n = Math.round(Math.abs(days) * 10) / 10;
    var unit = n === 1 ? " day" : " days";
    if (days > 0) { return n + unit + " to spare"; }
    if (days < 0) { return n + unit + " behind"; }
    return "None to spare";
  }
  function figRowHtml(label, text) {
    return '<div class="det-r"><dt>' + h(label) + "</dt><dd>" + h(text) + "</dd></div>";
  }
  function wideRowHtml(label, innerHtml) {
    return '<div class="det-r dt-wide"><dt>' + h(label) + "</dt><dd>" + innerHtml + "</dd></div>";
  }
  function shownHtml(field, v) {
    if (v == null || v === "") { return NONE_HTML; }
    if (field === "course") {
      var entry = KnowluCourses.match(v);
      return KnowluKit.markHtml(v) + "<span>" + h(entry ? entry.title || entry.slug : v) + "</span>";
    }
    if (field === "due") { return h(KnowluFmt.due(v) || String(v)); }
    if ((field === "effort_hours" || field === "slice_hours") && typeof v === "number") { return h(KnowluFmt.minutes(v * 60)); }
    if (field === "progress") { return h(String(v) + "%"); }
    return h(rawOf(v));
  }
  function fieldRowHtml(n, field) {
    var fm = n.frontmatter || {};
    var v = own(fm, field) ? fm[field] : null;
    return '<div class="det-r"><dt>' + h(LABELS[field]) + '</dt><dd><button class="dt-v" type="button" data-field="' + h(field) + '" data-value="' + h(rawOf(v)) + '">' + shownHtml(field, v) + "</button></dd></div>";
  }
  function roRowHtml(field, v) {
    return '<div class="det-r"><dt>' + h(LABELS[field]) + '</dt><dd class="dt-ro" data-ro="' + h(field) + '">' + h(rawOf(v)) + "</dd></div>";
  }
  function topHtml(n) {
    var row = n.row;
    var rowsHtml = fieldRowHtml(n, "course") + (row ? figRowHtml("Needs", KnowluFmt.minutes(row.hours * 60)) : "") + fieldRowHtml(n, "due");
    if (row && row.start_by) {
      rowsHtml += figRowHtml("Start by", row.start_by);
    }
    if (row && typeof row.slack_days === "number") {
      rowsHtml += figRowHtml("Slack", slackWords(row.slack_days));
    }
    if (row && row.fit_text) {
      rowsHtml += wideRowHtml("Won't fit", "<span>" + h(row.fit_text) + "</span>" + (row.fit_why ? '<span class="dt-fine">' + h(row.fit_why) + "</span>" : ""));
    }
    if (row && row.why) {
      rowsHtml += wideRowHtml("Why", h(row.why));
    }
    if (row && row.amend_badge && row.amend_badge.sentence) {
      rowsHtml += wideRowHtml("Proposed", h(row.amend_badge.sentence));
    }
    if (row && row.flag_text) {
      rowsHtml += wideRowHtml("Flagged", h(row.flag_text));
    }
    if (row && row.conflict_text) {
      rowsHtml += wideRowHtml("Conflict", h(row.conflict_text));
    }
    if (row && row.merge_text) {
      rowsHtml += wideRowHtml("Merged", h(row.merge_text));
    }
    rowsHtml += fieldRowHtml(n, "progress") + fieldRowHtml(n, "effort_hours");
    return '<dl class="det dt-top">' + rowsHtml + "</dl>";
  }
  function moreHtml(n) {
    var fm = n.frontmatter || {};
    var fields = EDITABLE.filter(function (f) { return TOP.indexOf(f) < 0 && (COMMITMENT_ONLY.indexOf(f) < 0 || n.folder === "commitments"); });
    var ro = READ_ONLY.filter(function (f) { return own(fm, f) && fm[f] != null; });
    var listHtml = fields.map(function (f) { return fieldRowHtml(n, f); }).join("") + ro.map(function (f) { return roRowHtml(f, fm[f]); }).join("");
    return '<button class="obtn dt-more" type="button" data-dt-more aria-expanded="' + (CUR && CUR.more ? "true" : "false") + '">More' + CHEVRON + "</button>" +
      '<dl class="det dt-rest"' + (CUR && CUR.more ? "" : " hidden") + ">" + listHtml + "</dl>";
  }
  function notesHtml(n) {
    var text = typeof n.body === "string" ? n.body.trim() : "";
    var bodyHtml = (text ? '<pre class="dt-body">' + h(text) + "</pre>" : "");
    if (!own(BODY_FOLDERS, n.folder)) {
      return (bodyHtml ? '<section class="dt-sec"><h3 class="sec-h">Notes</h3>' + bodyHtml + "</section>" : "");
    }
    return '<section class="dt-sec" data-body-host><h3 class="sec-h">Notes</h3>' + bodyHtml + '<button class="tbtn" type="button" data-body-edit>' + h(text ? "Edit" : "Add notes") + "</button></section>";
  }
  function whenOf(ts) {
    var d = new Date(ts);
    return isNaN(d.getTime()) ? String(ts || "") : KnowluFmt.date(d) + ", " + KnowluFmt.clock(d);
  }
  function histRowHtml(r) {
    return '<li><span class="dt-when">' + h(whenOf(r.ts)) + "</span><span>" + h(r.text) + "</span></li>";
  }
  function histHtml(n) {
    var list = Array.isArray(n.history) ? n.history.slice(0, 20) : [];
    return (list.length ? '<section class="dt-sec"><h3 class="sec-h">History</h3><ol class="dt-hist">' + list.map(histRowHtml).join("") + "</ol></section>" : "");
  }
  function actsHtml(n) {
    return '<div class="det-acts dt-acts"><button class="qbtn" type="button" data-flag>Flag</button>' + (n.folder === "tasks" ? '<button class="qbtn" type="button" data-del>Archive</button>' : "") + '</div><div data-flag-host></div>';
  }
  function detailsHtml(n) {
    return '<div class="dt" data-id="' + h(n.id) + '" data-kind="' + h(kindOf(n)) + '">' + topHtml(n) + moreHtml(n) + notesHtml(n) + histHtml(n) + actsHtml(n) + "</div>";
  }

  /* ---- reading and drawing ---- */
  function load(id) {
    KnowluData.call("note", { id: id }).then(function (env) {
      if (!CUR || CUR.id !== id) { return; }
      if (!env || !env.ok || !env.note) {
        if (!CUR.note) {
          CUR.body.innerHTML = '<p class="dt-miss">' + h(KnowluKit.refusal(env)) + "</p>";
        }
        return;
      }
      CUR.note = env.note;
      draw();
    });
  }
  function reload() {
    if (CUR) { load(CUR.id); }
  }
  function draw() {
    if (!CUR || !CUR.note) { return; }
    if (CUR.edit || CUR.bodyEd) { return; }
    var n = CUR.note;
    var fm = n.frontmatter || {};
    CUR.api.title(fm.title || n.slug);
    CUR.api.check(checkRow(n));
    var keep = CUR.body.scrollTop;
    var flag = CUR.body.querySelector(".dt-flag");
    if (flag) { flag.remove(); }
    CUR.body.innerHTML = detailsHtml(n);
    if (flag) { CUR.body.querySelector("[data-flag-host]").appendChild(flag); }
    CUR.body.scrollTop = keep;
    KnowluKit.watchSeen();
  }

  /* ---- click to edit ---- */
  function boundWords(range) {
    var lo = range ? range[0] : null;
    var hi = range ? range[1] : null;
    if (lo != null && hi != null) { return " from " + lo + " to " + hi; }
    if (lo != null) { return " at least " + lo; }
    return hi != null ? " at most " + hi : "";
  }
  function progressFields(pct) { var f = { progress: pct }; if (pct === 100) { f.status = "done"; } return f; }
  function startEdit(btn) {
    var field = btn.getAttribute("data-field");
    var raw = btn.getAttribute("data-value") || "";
    var range = own(NUMERIC_RANGE, field) ? NUMERIC_RANGE[field] : null;
    var input;
    if (field === "course") {
      input = document.createElement("select");
      input.innerHTML = '<option value="">No class</option>' + KnowluCourses.all().map(courseOptionHtml).join("");
      if (raw && !Array.prototype.some.call(input.options, function (o) { return o.value === raw; })) {
        var extra = document.createElement("option");
        extra.value = raw;
        extra.textContent = raw;
        input.appendChild(extra);
      }
    } else {
      input = document.createElement("input");
      input.type = field === "due" ? "datetime-local" : range ? "number" : "text";
      if (range && range[0] != null) { input.min = String(range[0]); }
      if (range && range[1] != null) { input.max = String(range[1]); }
      if (range && range[2] != null) { input.step = String(range[2]); }
    }
    input.className = "field dt-in";
    input.setAttribute("aria-label", LABELS[field]);
    input.value = field === "due" && /^\d{4}-\d{2}-\d{2}$/.test(raw) ? raw + "T23:59" : raw;
    CUR.edit = { field: field, btn: btn, dd: btn.parentNode, input: input, start: String(input.value) };
    ev("edit_started", CUR.id, field);
    btn.parentNode.replaceChildren(input);
    input.focus();
    if (typeof input.select === "function") { input.select(); }
    input.addEventListener("keydown", function (e) {
      if (e.key === "Enter") { e.preventDefault(); finishEdit(true); }
      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); finishEdit(false); }
    });
    input.addEventListener("blur", function () { finishEdit(true); });
    if (field === "course") { input.addEventListener("change", function () { finishEdit(true); }); }
  }
  function restoreEdit(ed) {
    if (ed.dd.isConnected) { ed.dd.replaceChildren(ed.btn); }
  }
  function finishEdit(commit) {
    var ed = CUR ? CUR.edit : null;
    if (!ed) { return; }
    CUR.edit = null;
    var id = CUR.id;
    var field = ed.field;
    var v = String(ed.input.value).trim();
    var range = own(NUMERIC_RANGE, field) ? NUMERIC_RANGE[field] : null;
    if (!commit || v === ed.start.trim()) {
      restoreEdit(ed);
      ev("edit_cancelled", id, field);
      return;
    }
    if (ed.input.type === "number") {
      if (v === "") {
        restoreEdit(ed);
        ev("edit_cancelled", id, field);
        return;
      }
      var num = Number(v);
      var low = range && range[0] != null && num < range[0];
      var high = range && range[1] != null && num > range[1];
      if (isNaN(num) || low || high || (ed.start !== "" && num === Number(ed.start))) {
        restoreEdit(ed);
        if (isNaN(num) || low || high) { KnowluKit.toast("Enter a number" + boundWords(range), null, null, true); }
        ev("edit_cancelled", id, field);
        return;
      }
      var numFields = field === "progress" ? progressFields(num) : {};
      if (field !== "progress") { numFields[field] = num; }
      commitEdit(id, numFields, field, ed);
      return;
    }
    var fields = {};
    fields[field] = v === "" ? null : v;
    commitEdit(id, fields, field, ed);
  }
  function commitEdit(id, fields, field, ed) {
    restoreEdit(ed);
    KnowluData.call("set_fields", { id: id, fields: fields }).then(function (env) {
      if (env && env.ok) {
        ev("edit_committed", id, field);
        return;
      }
      KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
      if (!env || !env.state) { reload(); }
    });
  }
  function toggleMore() {
    CUR.more = !CUR.more;
    var b = CUR.body.querySelector("[data-dt-more]");
    var rest = CUR.body.querySelector(".dt-rest");
    if (b) { b.setAttribute("aria-expanded", CUR.more ? "true" : "false"); }
    if (rest) { rest.hidden = !CUR.more; }
  }

  /* ---- the body ---- */
  function lockBody(ed, on) {
    ed.ta.disabled = on;
    ed.host.querySelectorAll("[data-body-save], [data-body-cancel]").forEach(function (b) { b.disabled = on; });
  }
  function openBodyEditor(expected) {
    var host = CUR.body.querySelector("[data-body-host]");
    if (!host || CUR.edit || CUR.bodyEd) { return; }
    var id = CUR.id;
    host.setAttribute("data-open-editor", "body");
    host.innerHTML = BODY_ED_HTML;
    var ta = host.querySelector("textarea");
    ta.value = expected;
    CUR.bodyEd = { id: id, expected: expected, ta: ta, host: host, busy: false };
    ev("edit_started", id, "body");
    ta.focus();
    host.querySelector("[data-body-save]").addEventListener("click", saveBody);
    host.querySelector("[data-body-cancel]").addEventListener("click", cancelBody);
    ta.addEventListener("keydown", function (e) {
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); saveBody(); }
      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); cancelBody(); }
    });
  }
  function cancelBody() {
    var ed = CUR ? CUR.bodyEd : null;
    if (!ed || ed.busy) { return; }
    ev("edit_cancelled", ed.id, "body");
    CUR.bodyEd = null;
    draw();
  }
  function saveBody() {
    var ed = CUR ? CUR.bodyEd : null;
    if (!ed || ed.busy) { return; }
    var id = ed.id;
    var before = ed.expected;
    ed.busy = true;
    lockBody(ed, true);
    KnowluData.call("set_body", { id: id, expected: ed.expected, body: ed.ta.value }).then(function (env) {
      ed.busy = false;
      if (env && env.conflict) {
        lockBody(ed, false);
        conflictState(ed);
        return;
      }
      if (env && env.ok) {
        ev("edit_committed", id, "body");
        if (CUR && CUR.bodyEd === ed) { CUR.bodyEd = null; reload(); }
        offerBodyUndo(id, before);
        return;
      }
      lockBody(ed, false);
      KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
    });
  }
  function conflictState(ed) {
    if (ed.host.querySelector("[data-body-conflict]")) { return; }
    var box = document.createElement("div");
    box.className = "dt-conf";
    box.setAttribute("data-body-conflict", "");
    box.innerHTML = CONFLICT_HTML;
    ed.host.appendChild(box);
    box.querySelector("[data-body-copy]").addEventListener("click", function () {
      KnowluData.call("copy_text", { text: ed.ta.value }).then(function (env) {
        KnowluKit.toast(env && env.ok ? "Copied" : KnowluKit.refusal(env), null, null, !(env && env.ok));
      });
    });
    box.querySelector("[data-body-reload]").addEventListener("click", function () {
      if (ed.ta.value !== ed.expected && !window.confirm("Reload and discard your text?")) { return; }
      ev("edit_cancelled", ed.id, "body");
      if (CUR && CUR.bodyEd === ed) { CUR.bodyEd = null; reload(); }
    });
  }
  function dropSaved() {
    document.querySelectorAll(".savedtoast").forEach(function (t) { t.remove(); });
  }
  function offerBodyUndo(id, before) {
    KnowluData.call("note", { id: id }).then(function (env) {
      if (!env || !env.ok || !env.note) { return; }
      if (!CUR || CUR.id !== id) { return; }
      var host = document.getElementById("toasts");
      if (!host) { return; }
      dropSaved();
      var t = document.createElement("div");
      t.className = "toast savedtoast";
      t.setAttribute("role", "status");
      var words = document.createElement("span");
      words.textContent = "Saved";
      var b = document.createElement("button");
      b.type = "button";
      b.className = "tbtn";
      b.setAttribute("data-body-undo", "");
      b.textContent = "Undo";
      t.appendChild(words);
      t.appendChild(b);
      host.appendChild(t);
      var gone = setTimeout(function () { t.remove(); }, 10000);
      b.addEventListener("click", function () {
        clearTimeout(gone);
        b.disabled = true;
        KnowluData.call("set_body", { id: id, expected: env.note.body || "", body: before }).then(function (r) {
          if (r && r.ok) {
            t.remove();
            ev("edit_committed", id, "body");
            if (CUR && CUR.id === id && !CUR.bodyEd) { reload(); }
            return;
          }
          KnowluKit.toast(KnowluKit.refusal(r), null, null, true);
          words.textContent = "Not undone";
          var copy = document.createElement("button");
          copy.type = "button";
          copy.className = "tbtn";
          copy.textContent = "Copy previous text";
          copy.addEventListener("click", function () { KnowluData.call("copy_text", { text: before }); });
          t.replaceChild(copy, b);
          setTimeout(function () { t.remove(); }, 10000);
        });
      });
    });
  }

  /* ---- the flag and Archive ---- */
  function flagPop() { return CUR ? CUR.body.querySelector(".dt-flag") : null; }
  function openFlag() {
    var host = CUR.body.querySelector("[data-flag-host]");
    if (!host || host.querySelector(".dt-flag")) { return; }
    host.innerHTML = FLAG_HTML;
    var first = host.querySelector("[data-cat]");
    if (first) { first.focus(); }
  }
  function sendFlag() {
    var pop = flagPop();
    if (!pop) { return; }
    var id = CUR.id;
    var kind = kindOf(CUR.note);
    var cats = Array.prototype.map.call(pop.querySelectorAll('[data-cat][aria-pressed="true"]'), function (b) { return b.getAttribute("data-cat"); });
    if (!cats.length) { return; }
    var ta = pop.querySelector("textarea");
    var send = pop.querySelector("[data-flag-send]");
    send.disabled = true;
    KnowluData.call("open_issue", { target: id, categories: cats, text: ta.value }).then(function (env) {
      if (env && env.ok) {
        ev("issue_opened", id, kind);
        pop.remove();
        KnowluKit.toast("Flagged");
        return;
      }
      send.disabled = false;
      KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
    });
  }
  function archive() {
    var n = CUR.note;
    var id = CUR.id;
    var name = (n.frontmatter && n.frontmatter.title) || n.slug;
    if (!window.confirm("Archive “" + name + "”? Nothing is deleted. It moves to the archive and shows under Done.")) { return; }
    KnowluData.call("delete_note", { id: id }).then(function (env) {
      if (env && env.ok) {
        KnowluPanel.close();
        KnowluKit.toast("Archived");
        return;
      }
      KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
    });
  }

  function bindBody(body) {
    body.addEventListener("click", function (e) {
      if (!CUR || !CUR.note || !e.target.closest(".dt")) { return; }
      var t = e.target;
      var v = t.closest("[data-field]");
      if (v) { if (!CUR.edit && !CUR.bodyEd) { startEdit(v); } return; }
      if (t.closest("[data-dt-more]")) { toggleMore(); return; }
      if (t.closest("[data-body-edit]")) { openBodyEditor(typeof CUR.note.body === "string" ? CUR.note.body : ""); return; }
      if (t.closest("[data-flag]")) { openFlag(); return; }
      var cat = t.closest("[data-cat]");
      if (cat) {
        cat.setAttribute("aria-pressed", cat.getAttribute("aria-pressed") === "true" ? "false" : "true");
        flagPop().querySelector("[data-flag-send]").disabled = !flagPop().querySelector('[data-cat][aria-pressed="true"]');
        return;
      }
      if (t.closest("[data-flag-cancel]")) { flagPop().remove(); return; }
      if (t.closest("[data-flag-send]")) { sendFlag(); return; }
      if (t.closest("[data-del]")) { archive(); }
    });
  }

  KnowluData.on(function (st) {
    if (!CUR || CUR.edit || CUR.bodyEd) { return; }
    if (st) { load(CUR.id); }
  });

  KnowluPanel.add("task", {
    render: function (body, id, api) {
      dropSaved();
      CUR = { id: id, api: api, body: body, note: null, edit: null, bodyEd: null, more: false };
      if (BOUND !== body) { bindBody(body); BOUND = body; }
      var row = rowInState(id);
      api.title(row ? row.title : "");
      api.check(row);
      body.textContent = "";
      load(id);
    },
    close: function () {
      dropSaved();
      CUR = null;
    }
  });
})();
