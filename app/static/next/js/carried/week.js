/* Knowlu app: Your week and Day, a Settings section (note §4 Settings "Your week, Day", §5; plan §5 T18).

   Registered as the Settings section `week` (KnowluRouter.addSection). When Settings mounts it, it reads `your_week`
   (the engine's commitments overview; Settings itself never asks the week, P4) and draws two sections:
     Your week   the registrar row (only where `registrar_label` names one; the label is the school's, never this
                 page's), the registrar's own proposals with Add, its list of what changed, ended, was kept, has a
                 status or could not be read (read from `registrar_changes`, never worked out here); the confirmed
                 commitments, each with its kind (a select) and how much it binds (a segmented choice), both saved
                 through set_fields; the courses with no class time yet; office hours with Add (confirmed at optional)
     Day         the window Knowlu plans in, a start and an end per weekday. An edit previews 400 ms later through
                 preview_window, a read whose state is never adopted (M4): the moved line, or "No change to today's
                 plan", and the first five items of the previewed day. Save sends the window through
                 commitments_confirm; the engine validates it and its message lands under the day it names.
   Nothing here computes a time: the window is string assembly from the pickers, and the engine checks it. Every
   write goes through KnowluData.call, which adds the page's state view and adopts the answer's state; the section
   then reads your_week again (update). Unsaved Day edits survive that re-read.

   window.KnowluWeek lends the pieces the first-day sheet (weeksetup.js) shares: the window editor (editorHtml,
   sequence, showError, bindEditor), the registrar controls (registrarHtml, bindRegistrar, setLabel, fromRegistrar),
   the registrar's change list (changesHtml) and the level choice (levelHtml). */
(function () {
  "use strict";
  var DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
  var DAY_NAMES = { mon: "Monday", tue: "Tuesday", wed: "Wednesday", thu: "Thursday", fri: "Friday", sat: "Saturday", sun: "Sunday" };
  var KIND_NAMES = [["class", "Class"], ["lab", "Lab"], ["work", "Work"], ["club", "Club"], ["meeting", "Meeting"], ["office-hours", "Office hours"]];
  var LEVELS = [["hard", "Must keep"], ["soft", "Usually"], ["optional", "Optional"]];
  var PREVIEW_MS = 400;
  /* B5, m3 and m4: a fetch that reached the engine and failed shows this sentence and nothing the engine said. The
     school's window is gone by then, so a retry starts from a fresh sign-in. */
  var REG_FAILED = "Knowlu couldn't use what the school sent, so nothing was changed. The sign-in window is closed; to try again, press the button and sign in again.";
  var REG_FIELDS = { meets: "meeting times", where: "room", until: "last day", from: "first day" };
  var CARDS = { filed: "Knowlu has asked you in Decisions which to keep.", waiting: "Knowlu will ask you in Decisions which to keep.", listed: "Listed here only." };
  var KEPT_YOURS_HTML = ' <span class="wk-card">You had set a different last day, so Knowlu kept yours.</span>';
  var CHEVRON_HTML = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 6l4 4 4-4"/></svg>';

  var WEEK = { host: null, seq: 0, dirty: false, timer: null, pseq: 0 };
  var registrarLabel = null;   /* your_week's registrar_label; null where the school has none */

  function el(id) { return document.getElementById(id); }
  function say(node, text, bad) {
    if (!node) { return; }
    node.textContent = text || "";
    node.classList.toggle("st-bad", !!bad);
  }

  /* ---- the shared controls: each names the row it sits in (titleHtml, the row's title already through h()), so one
     row's level or kind is told from the next ---- */
  function levelHtml(level, titleHtml) {
    var buttonsHtml = "";
    LEVELS.forEach(function (l) {
      buttonsHtml += '<button class="obtn" type="button" data-level-set="' + h(l[0]) + '" aria-pressed="' + (l[0] === level ? "true" : "false") + '">' + h(l[1]) + "</button>";
    });
    return '<div class="seg wk-lvl" role="group" aria-label="How much it binds, ' + titleHtml + '">' + buttonsHtml + "</div>";
  }
  function kindHtml(kind, titleHtml) {
    var known = KIND_NAMES.some(function (k) { return k[0] === kind; });
    var optionsHtml = known ? "" : '<option value="' + h(kind) + '" selected>' + h(kind) + "</option>";
    KIND_NAMES.forEach(function (k) {
      optionsHtml += '<option value="' + h(k[0]) + '"' + (k[0] === kind ? " selected" : "") + ">" + h(k[1]) + "</option>";
    });
    return '<span class="sel wk-kind"><select data-kind-set aria-label="Kind, ' + titleHtml + '">' + optionsHtml + "</select>" + CHEVRON_HTML + "</span>";
  }

  /* ---- the window editor: one row per weekday, Mon to Sun; byDay is {mon: {start, end}} ---- */
  function dayRowHtml(d, w) {
    return '<li class="st-row wk-day" data-day="' + h(d) + '"><div class="st-l"><p class="st-n">' + h(DAY_NAMES[d]) + '</p><p class="st-hint st-bad" data-werr="' + h(d) + '"></p></div>' +
      '<div class="st-c wk-span"><input class="field wk-t" type="time" data-part="start" aria-label="' + h(DAY_NAMES[d] + " start") + '" value="' + h(w.start || "") + '">' +
      '<span class="wk-to">to</span><input class="field wk-t" type="time" data-part="end" aria-label="' + h(DAY_NAMES[d] + " end") + '" value="' + h(w.end || "") + '"></div></li>';
  }
  function windowEditorHtml(byDay) {
    var sameHtml = '<li class="st-row wk-same"><button class="tbtn" type="button" data-same-as-monday>Same as Monday for Tue&ndash;Fri</button></li>';
    var rowsHtml = "";
    DAYS.forEach(function (d) {
      rowsHtml += dayRowHtml(d, byDay[d] || {});
      if (d === "mon") {
        rowsHtml += sameHtml;
      }
    });
    return rowsHtml;
  }
  /* The flow sequence from the pickers. Days with the same start and end share one entry; a day with an empty picker
     is left out, so it keeps week_template.yaml's hours. Null when every row is blank: no window is sent. */
  function windowSequence(host) {
    var groups = [], byKey = {};
    DAYS.forEach(function (d) {
      var row = host.querySelector('[data-day="' + d + '"]');
      if (!row) { return; }
      var s = row.querySelector('[data-part="start"]').value, e = row.querySelector('[data-part="end"]').value;
      if (!s || !e) { return; }
      var k = s + "-" + e;
      if (!Object.prototype.hasOwnProperty.call(byKey, k)) { byKey[k] = { days: [], start: s, end: e }; groups.push(byKey[k]); }
      byKey[k].days.push(d);
    });
    if (!groups.length) { return null; }
    return "[" + groups.map(function (g) { return "{days: [" + g.days.join(", ") + "], start: \x22" + g.start + "\x22, end: \x22" + g.end + "\x22}"; }).join(", ") + "]";
  }
  /* The engine's own message, under the row it names ("planning day fri: ..."), else under Monday. */
  function showWindowError(host, message) {
    if (!host) { return; }
    host.querySelectorAll("[data-werr]").forEach(function (e) { e.textContent = ""; });
    if (!message) { return; }
    var m = /planning day (\w+)/.exec(message);
    var slot = (m && host.querySelector('[data-werr="' + m[1] + '"]')) || host.querySelector('[data-werr="mon"]');
    if (slot) { slot.textContent = message; }
  }
  function bindWindowEditor(host, onEdit) {
    host.addEventListener("click", function (e) {
      if (!e.target.closest("[data-same-as-monday]")) { return; }
      var mon = host.querySelector('[data-day="mon"]');
      ["tue", "wed", "thu", "fri"].forEach(function (d) {
        var row = host.querySelector('[data-day="' + d + '"]');
        row.querySelector('[data-part="start"]').value = mon.querySelector('[data-part="start"]').value;
        row.querySelector('[data-part="end"]').value = mon.querySelector('[data-part="end"]').value;
      });
      onEdit();
    });
    host.addEventListener("input", onEdit);
  }

  /* ---- Day's preview: a read (M4) ---- */
  function schedulePreview() {
    clearTimeout(WEEK.timer);
    WEEK.timer = setTimeout(runPreview, PREVIEW_MS);
  }
  function runPreview() {
    var host = el("wk-window");
    if (!host || !WEEK.host) { return; }
    var NONE = "No change to today's plan";
    var seq = windowSequence(host), mine = ++WEEK.pseq;
    if (!seq) {
      el("wk-moved").textContent = NONE;
      el("wk-items").innerHTML = "";
      return;
    }
    KnowluData.call("preview_window", { window: seq }).then(function (r) {
      if (mine !== WEEK.pseq || !el("wk-window")) { return; }
      if (!r || !r.ok || !r.state) { showWindowError(host, KnowluKit.refusal(r)); return; }
      showWindowError(host, null);
      var items = [];
      var moved = r.state.moved;
      var day = r.state.the_day;
      ((day && day.blocks) || []).forEach(function (b) { (b.takes || []).forEach(function (t) { items.push(t.title); }); });
      el("wk-moved").textContent = moved && moved.text ? moved.text : NONE;
      el("wk-items").innerHTML = items.slice(0, 5).map(function (t) { return "<li>" + h(t) + "</li>"; }).join("");
    });
  }

  /* ---- writes ---- */
  function confirmWeek(payload) {
    return KnowluData.call("commitments_confirm", { confirm: payload });
  }
  function saveCommitment(id, fields) {
    return KnowluData.call("set_fields", { id: id, fields: fields }).then(function (env) {
      if (!env || !env.ok) { KnowluKit.toast(KnowluKit.refusal(env), null, null, true); }
      return env;
    });
  }
  function saveWindow() {
    var host = el("wk-window"), seq = windowSequence(host), mine = WEEK.seq;
    if (!seq) { say(el("wk-say"), "Nothing to save"); return; }
    el("wk-save").disabled = true;
    confirmWeek({ window: seq }).then(function (env) {
      if (mine !== WEEK.seq || !el("wk-save")) { return; }
      el("wk-save").disabled = false;
      if (!env || !env.ok) { showWindowError(host, KnowluKit.refusal(env)); return; }
      WEEK.dirty = false;
      showWindowError(host, null);
      say(el("wk-say"), env.result && env.result.window === "unchanged" ? "No change" : "Saved");
    });
  }

  /* ---- the registrar (phase 3, D1, D6, §2): the page opens the school's sign-in window and, once the student says
     they are signed in, asks the app to read the schedule. The page never sees a cookie or the fetched bytes. ---- */
  function fromRegistrar(p) { return !!p.source_uid && p.source_uid.indexOf("registrar:") === 0; }
  function registrarHtml() {
    return '<div class="wk-regctl"><button class="tbtn" type="button" data-reg-open></button>' +
      '<button class="tbtn" type="button" data-reg-done hidden>' + h("I'm signed in") + '</button><button class="qbtn" type="button" data-reg-cancel hidden>Cancel</button></div>' +
      '<p class="st-hint wk-say" data-reg-say role="status"></p>';
  }
  function registrarControls(host) {
    return { open: host.querySelector("[data-reg-open]"), done: host.querySelector("[data-reg-done]"),
      cancel: host.querySelector("[data-reg-cancel]"), say: host.querySelector("[data-reg-say]") };
  }
  function registrarIdle(host, message) {
    var c = registrarControls(host);
    c.open.hidden = false; c.done.hidden = true; c.cancel.hidden = true; c.done.disabled = false;
    c.say.textContent = message || "";
  }
  /* after(env) runs once a fetch reached the engine; busy(on), if given, holds the host's screen from I'm signed in
     until the fetch and after() settle, on every path (R5 I1). */
  function bindRegistrar(host, after, busy) {
    var setBusy = busy || function () {};
    host.addEventListener("click", function (e) {
      e.stopPropagation();
      var c = registrarControls(host);
      if (e.target.closest("[data-reg-open]")) {
        KnowluData.call("open_registrar_window", {}).then(function (r) {
          if (!r || !r.ok) { registrarIdle(host, KnowluKit.refusal(r)); return; }
          c.open.hidden = true; c.done.hidden = false; c.cancel.hidden = false;
          c.say.textContent = "Sign in to " + registrarLabel + " in the window Knowlu opened, then press I'm signed in.";
        });
      } else if (e.target.closest("[data-reg-done]")) {
        c.done.disabled = true; c.cancel.hidden = true; setBusy(true);
        c.say.textContent = "Reading your class schedule…";
        KnowluData.call("capture_registrar", {}).then(function (env) {
          env = env || {};
          c.done.disabled = false;
          if (!env.ok) {
            setBusy(false);
            if (env.closed) { registrarIdle(host, REG_FAILED); } else { c.cancel.hidden = false; c.say.textContent = KnowluKit.refusal(env); }
            return;
          }
          var r = env.result || {};
          registrarIdle(host, "Found " + r.rows + (r.rows === 1 ? " class" : " classes") + " in " + registrarLabel + ": " + r.confirmed + " confirmed, " + r.proposed + " to check.");
          return Promise.resolve(after(env)).then(function () { setBusy(false); }, function () { setBusy(false); });
        });
      } else if (e.target.closest("[data-reg-cancel]")) {
        KnowluData.call("close_registrar_window", {});
        registrarIdle(host, "");
      }
    });
  }
  function regProposalHtml(p) {
    return '<li class="st-row wk-row"><div class="st-l"><p class="st-n">' + h(p.title) + '</p><p class="st-hint">' + h((p.when || "") + " · from " + registrarLabel) + "</p></div>" +
      '<div class="st-c"><button class="obtn" type="button" data-reg-add="' + h(p.source_uid) + '" data-level="' + h(p.level) + '">Add</button></div></li>';
  }
  /* R5-c: the button names the label, Refresh once a term is held; it is the red act only when a term the file does
     not hold has begun. */
  function renderRegistrar(w) {
    var box = el("wk-reg"), changes = el("wk-changes");
    box.hidden = !registrarLabel;
    if (!registrarLabel) {
      el("wk-reg-rows").innerHTML = "";
      changes.hidden = true;
      changes.innerHTML = "";
      return;
    }
    var reg = w.registrar, open = box.querySelector("[data-reg-open]");
    open.textContent = (reg && reg.held && reg.held.length ? "Refresh from " : "Get my class times from ") + registrarLabel;
    open.className = reg && reg.refresh ? "btn small" : "tbtn";
    el("wk-reg-rows").innerHTML = (w.registrar_proposals || []).map(regProposalHtml).join("");
    var listHtml = registrarChangesHtml(w.registrar_changes, w.commitments, registrarLabel);
    changes.hidden = !listHtml;
    changes.innerHTML = listHtml;
  }

  /* ---- the registrar's list (B5, spec A2 item 13): a note is named by its title (the block holds its path), an
     unread row is worded here, and every value is escaped; nothing is worked out. ---- */
  function regMeets(list) {
    return list.map(function (m) {
      return m.days.map(function (d) { return d.charAt(0).toUpperCase() + d.slice(1, 3); }).join("/") + " " + m.start + "–" + m.end;
    }).join(", ");
  }
  function regValue(v) { return v == null || v === "" ? "nothing" : Array.isArray(v) ? regMeets(v) : v; }
  function regField(f) { return Object.prototype.hasOwnProperty.call(REG_FIELDS, f) ? REG_FIELDS[f] : f; }
  /* returns markup: the title is escaped here, once */
  function regTitle(note, key, commitments) {
    var c = (commitments || []).filter(function (x) { return x.path === note; })[0];
    return c ? h(c.title) : (key ? "Class " + h(String(key).split("-").pop()) : "A class");
  }
  function cardHtml(words) {
    return '<span class="wk-card">' + h(words) + "</span>";
  }
  function changedHtml(e, ctx) {
    return regTitle(e.note, null, ctx.commitments) + ": " + h(regField(e.field)) + " " + h(regValue(e["from"])) + " → " + h(regValue(e.to));
  }
  function endedHtml(e, ctx) {
    var tail = e.kept_yours ? KEPT_YOURS_HTML : "";
    return regTitle(e.note, null, ctx.commitments) + " ended on " + h(e.until) + tail;
  }
  function keptHtml(e, ctx) {
    var card = Object.prototype.hasOwnProperty.call(CARDS, e.card) ? CARDS[e.card] : CARDS.listed;
    var said = regTitle(e.note, null, ctx.commitments) + ": your " + h(regField(e.field)) + " is " + h(regValue(e.student)) + "; " + h(ctx.label) + " has " + h(regValue(e.registrar)) + ". ";
    return said + cardHtml(card);
  }
  function statusHtml(e, ctx) {
    return regTitle(e.note, e.key, ctx.commitments) + ": " + h(ctx.label) + " lists it as " + h(e.status) + "; nothing changed.";
  }
  function unreadHtml(e, ctx) {
    var what = String(e.reason || "").indexOf("no class time") >= 0 ? "no class time" : "a meeting Knowlu can't read";
    return regTitle(e.note, e.key, ctx.commitments) + ": " + h(ctx.label) + " lists it with " + h(what) + "; nothing changed.";
  }
  function regGroupHtml(head, rows, lineHtml, ctx) {
    if (!rows || !rows.length) { return ""; }
    var itemsHtml = "";
    rows.forEach(function (e) {
      itemsHtml += "<li>" + lineHtml(e, ctx) + "</li>";
    });
    return '<p class="st-sub">From ' + h(ctx.label) + h(head) + "</p><ul>" + itemsHtml + "</ul>";
  }
  function registrarChangesHtml(ch, commitments, label) {
    if (!ch || !label) { return ""; }
    var ctx = { commitments: commitments || [], label: label };
    return regGroupHtml(" — changed", ch.applied, changedHtml, ctx) + regGroupHtml(" — ended", ch.ended, endedHtml, ctx) +
      regGroupHtml(" — kept yours", ch.kept, keptHtml, ctx) + regGroupHtml(" — status", ch.status, statusHtml, ctx) +
      regGroupHtml(" — not read", ch.unread, unreadHtml, ctx);
  }

  /* ---- the section ---- */
  function sectionHtml() {
    return '<section class="st-sec wk" data-section="week" aria-labelledby="st-h-week"><h2 class="st-h" id="st-h-week" tabindex="-1">Your week</h2>' +
      '<p class="st-hint wk-note" id="wk-note" role="status"></p>' +
      '<div class="wk-reg" id="wk-reg" hidden>' + registrarHtml() + "</div>" +
      '<ul class="st-rows" id="wk-reg-rows"></ul><div class="wk-changes" id="wk-changes" hidden></div>' +
      '<ul class="st-rows" id="wk-list" aria-label="Confirmed"></ul>' +
      '<h3 class="st-sub">Office hours</h3><ul class="st-rows" id="wk-oh"></ul></section>' +
      '<section class="st-sec wk" data-section="day" aria-labelledby="st-h-day"><h2 class="st-h" id="st-h-day" tabindex="-1">Day</h2>' +
      '<p class="st-hint wk-lede">The hours Knowlu plans in, each day. A day left blank keeps its usual hours.</p>' +
      '<ul class="st-rows wk-window" id="wk-window"></ul>' +
      '<div class="wk-preview"><p class="st-hint wk-moved" id="wk-moved" role="status"></p><ol class="wk-items" id="wk-items"></ol></div>' +
      '<div class="st-acts"><p class="st-hint" id="wk-say" role="status"></p><button class="tbtn" type="button" id="wk-save">Save</button></div></section>';
  }
  function commitmentHtml(c) {
    var meta = (c.when || "") + (c.where ? " · " + c.where : "");
    return '<li class="st-row wk-row" data-id="' + h(c.id) + '" data-kind="commitment"><div class="st-l"><p class="st-n">' + h(c.title) + '</p><p class="st-hint">' + h(meta) + "</p></div>" +
      '<div class="st-c wk-ctl">' + kindHtml(c.kind, h(c.title)) + levelHtml(c.level, h(c.title)) + "</div></li>";
  }
  function officeHtml(p) {
    return '<li class="st-row wk-row"><div class="st-l"><p class="st-n">' + h(p.title) + '</p><p class="st-hint">' + h(p.when || "") + "</p></div>" +
      '<div class="st-c"><button class="obtn" type="button" data-oh-add="' + h(p.source_uid) + '">Add</button></div></li>';
  }
  function renderWeek(w) {
    say(el("wk-note"), "");
    renderRegistrar(w);
    var emptyHtml = '<li class="st-row"><div class="st-l"><p class="st-hint">' + h("Nothing confirmed yet.") + "</p></div></li>";
    var uncoveredHtml = (w.uncovered_courses || []).map(function (u) { return '<li class="st-row wk-row"><div class="st-l"><p class="st-n">' + h(u.title) + '</p><p class="st-hint">Knowlu will ask when it meets</p></div></li>'; }).join("");
    var listHtml = (w.commitments || []).map(commitmentHtml).join("") + uncoveredHtml;
    el("wk-list").innerHTML = listHtml ? listHtml : emptyHtml;
    var ohEmptyHtml = '<li class="st-row"><div class="st-l"><p class="st-hint">' + h("No office hours found on your calendar.") + "</p></div></li>";
    var ohHtml = (w.office_hours || []).map(officeHtml).join("");
    el("wk-oh").innerHTML = ohHtml ? ohHtml : ohEmptyHtml;
    var host = el("wk-window"), byDay = {};
    (w.window || []).forEach(function (d) { byDay[d.day] = d; });
    if (!WEEK.dirty && !host.contains(document.activeElement)) {
      host.innerHTML = windowEditorHtml(byDay);
    }
    KnowluKit.watchSeen();
  }
  function loadWeek() {
    var mine = WEEK.seq;
    return KnowluData.call("your_week", {}).then(function (r) {
      if (mine !== WEEK.seq || !WEEK.host) { return; }
      if (!r || !r.ok || !r.week) { say(el("wk-note"), KnowluKit.refusal(r), true); return; }
      registrarLabel = r.registrar_label || null;
      renderWeek(r.week);
    });
  }
  function bindWeek(host) {
    host.addEventListener("click", function (e) {
      var lvl = e.target.closest("[data-level-set]");
      var row = lvl ? lvl.closest(".wk-row[data-id]") : null;
      if (lvl && row) {
        /* drawn as chosen at once; a refused save puts back the level held before */
        var levels = row.querySelectorAll("[data-level-set]");
        var held = Array.prototype.filter.call(levels, function (b) { return b.getAttribute("aria-pressed") === "true"; })[0] || null;
        levels.forEach(function (b) { b.setAttribute("aria-pressed", b === lvl ? "true" : "false"); });
        saveCommitment(row.getAttribute("data-id"), { level: lvl.getAttribute("data-level-set") }).then(function (env) {
          if (!env || !env.ok) { levels.forEach(function (b) { b.setAttribute("aria-pressed", b === held ? "true" : "false"); }); }
        });
        return;
      }
      var add = e.target.closest("[data-oh-add], [data-reg-add]");
      if (add) {
        add.disabled = true;
        var oh = add.hasAttribute("data-oh-add");
        var key = add.getAttribute(oh ? "data-oh-add" : "data-reg-add");
        /* §2.2: office hours are confirmed at optional; a registrar row at the level the engine proposed */
        var pick = oh ? { source_uid: key, level: "optional" } : { source_uid: key, level: add.getAttribute("data-level") };
        confirmWeek({ mine: [pick] }).then(function (env) {
          if (!env || !env.ok) { add.disabled = false; KnowluKit.toast(KnowluKit.refusal(env), null, null, true); }
        });
        return;
      }
      if (e.target.closest("#wk-save")) { saveWindow(); }
    });
    host.addEventListener("change", function (e) {
      var sel = e.target.closest("[data-kind-set]");
      var row = sel ? sel.closest(".wk-row[data-id]") : null;
      if (row) { saveCommitment(row.getAttribute("data-id"), { kind: sel.value }); }
    });
    bindWindowEditor(el("wk-window"), function () {
      WEEK.dirty = true;
      say(el("wk-say"), "");
      schedulePreview();
    });
    bindRegistrar(el("wk-reg"), function () { return loadWeek(); });
  }

  KnowluRouter.addSection("week", {
    mount: function (host) {
      WEEK.seq += 1;
      WEEK.host = host;
      WEEK.dirty = false;
      host.innerHTML = sectionHtml();
      bindWeek(host);
      loadWeek();
    },
    update: function () { if (WEEK.host) { loadWeek(); } },
    unmount: function () {
      var done = WEEK.host ? WEEK.host.querySelector("[data-reg-done]") : null;
      /* leaving Settings mid sign-in takes the school's window with it */
      if (done && !done.hidden) { KnowluData.call("close_registrar_window", {}); }
      WEEK.seq += 1;
      WEEK.host = null;
      WEEK.pseq += 1;
      clearTimeout(WEEK.timer);
    }
  });

  window.KnowluWeek = {
    editorHtml: windowEditorHtml,
    sequence: windowSequence,
    showError: showWindowError,
    bindEditor: bindWindowEditor,
    registrarHtml: registrarHtml,
    bindRegistrar: bindRegistrar,
    changesHtml: registrarChangesHtml,
    levelHtml: levelHtml,
    fromRegistrar: fromRegistrar,
    setLabel: function (label) { registrarLabel = label || null; }
  };
})();
