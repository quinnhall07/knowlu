/* Knowlu app, Home's aside (T8: note §4 Home, plan T8; DESIGN.md "The desktop app").

   Home (home.js) mounts it in two slots, under Do today and under Recommended, and hands it the whole state on every
   paint. The domain row never filters it.
     Waiting            S.decisions: its counts (pending, deferred, awaiting your calendar, today's digest), the engine's
                        empty text when it sets one, and with only a digest waiting the digest's count (I3; never "nothing
                        waiting" then). The cards are the deck's: Waiting hands its slot to KnowluSlots.deck (P3).
     What changed       S.delta (its summary and records), S.moved.text, S.good_to_know (each with Close: close_info),
                        and with email set up "From your email" and its count (mail_applied; the rows and their Undo are
                        the mail sheet's, opened through window.KnowluMail.openApplied when it offers one).
     Needs a date       S.nav_counts.undated, linking Tasks' No date filter (#tasks/none, one of Tasks' due arguments).
     Schedule           S.the_day: its all-day dates (an accepted date offers Remove from my day: hidden at once,
                        remove_lane_date when its ten-second Undo toast closes) and its class and busy blocks; what has
                        ended folds under "N earlier".
     Free time          S.the_day.open_hours and the free blocks of the same list, so the two always agree, each with
                        what the engine puts in it (takes[]). */
(function () {
  "use strict";
  var LANE_MS = 10000;

  var WAIT_HTML = '<section class="ha-sec" id="ha-wait" aria-labelledby="ha-lbl-wait">' +
    '<div class="ha-lblrow"><h2 class="hm-lbl" id="ha-lbl-wait">Waiting</h2><span class="ha-counts" id="ha-counts"></span></div>' +
    '<p class="ha-clear" id="ha-clear" hidden></p>' +
    '<div class="ha-deck" id="ha-deck"></div>' +
    '<div class="ha-changed" id="ha-changed" hidden></div>' +
    '<p class="ha-undated" id="ha-undated" hidden></p>' +
    "</section>";
  var DAY_HTML = '<section class="ha-sec" id="ha-sched-sec" aria-labelledby="ha-lbl-sched">' +
    '<div class="ha-lblrow"><h2 class="hm-lbl" id="ha-lbl-sched">Schedule</h2>' +
    '<button class="ha-early" type="button" data-early aria-expanded="false" aria-controls="ha-sched" hidden></button></div>' +
    '<ol class="ha-sched" id="ha-sched" aria-label="Today"></ol>' +
    "</section>" +
    '<section class="ha-sec" id="ha-free-sec" aria-labelledby="ha-lbl-free">' +
    '<h2 class="hm-lbl" id="ha-lbl-free">Free time</h2>' +
    '<div class="ha-free" id="ha-free"></div>' +
    "</section>";

  var WAIT = null, DAY = null, ST = null, MAIL = null, MAIL_ASKED = false, EARLY_OPEN = false, DAY_DATE = "";
  var PENDING_LANE = Object.create(null);

  function $(id) { return document.getElementById(id); }
  function stamp(hm) { return DAY_DATE + "T" + hm; }
  function nowHM(st) { return String(st.now || "").slice(11, 16); }

  /* ---- Waiting ---- */
  function countsText(d) {
    var parts = [];
    if (d.pending) { parts.push(d.pending + " pending"); }
    if (d.deferred) { parts.push(d.deferred + " deferred"); }
    if (d.awaiting_calendar) { parts.push(d.awaiting_calendar + " waiting on your calendar"); }
    if (d.events_in_digest && parts.length) { parts.push(d.events_in_digest + " in today's digest"); }
    return parts.join(" · ");
  }
  /* the engine's own empty text, only when it sets one; with only a digest waiting, the digest's count */
  function clearText(d) {
    if (d.empty_text) { return d.empty_text; }
    if (!d.pending && !d.deferred && !d.awaiting_calendar && d.events_in_digest) {
      return d.events_in_digest + (d.events_in_digest === 1 ? " event in today's digest" : " events in today's digest");
    }
    return "";
  }
  function recordHtml(r) {
    return '<li class="ha-rec">' + h(r.text) + "</li>";
  }
  function movedHtml(st) {
    var m = st.moved;
    if (!m || !m.text) { return ""; }
    return '<p class="ha-moved" id="ha-moved">' + h(m.text) + "</p>";
  }
  function gtkHtml(x) {
    return '<li class="ha-gtk-r" data-id="' + h(x.id) + '" data-kind="info"><span class="ha-gtk-nm">' + h(x.title) + "</span>" +
      (x.expires ? '<span class="ha-gtk-t">' + h("Until " + KnowluFmt.day(x.expires)) + "</span>" : "") +
      '<button class="tbtn" type="button" data-close-info="' + h(x.id) + '">Close</button></li>';
  }
  function mailCount(r) {
    return r.courses.reduce(function (n, g) {
      return n + (g.created || []).length + (g.changed || []).length + ((g.archived && g.archived.count) || 0) + (g.notices || []).length;
    }, 0);
  }
  function mailHtml() {
    var n = MAIL ? mailCount(MAIL) : 0;
    if (!n || !window.KnowluMail || typeof window.KnowluMail.openApplied !== "function") { return ""; }
    return '<p class="ha-mail"><button class="tbtn" type="button" data-mail-applied>' + "From your email" + "</button>" +
      '<span class="ha-mail-n">' + h(n === 1 ? "1 item" : n + " items") + "</span></p>";
  }
  function changedHtml(st) {
    var dl = st.delta || {};
    var recs = Array.isArray(dl.records) ? dl.records : [];
    var gtk = Array.isArray(st.good_to_know) ? st.good_to_know : [];
    var partsHtml = (dl.summary ? '<p class="ha-delta">' + h(dl.summary) + "</p>" : "") +
      (recs.length ? '<ul class="ha-recs">' + recs.map(recordHtml).join("") + "</ul>" : "") +
      (st.moved ? movedHtml(st) : "") +
      (gtk.length ? '<ul class="ha-gtk" id="ha-gtk">' + gtk.map(gtkHtml).join("") + "</ul>" : "") +
      mailHtml();
    return (partsHtml ? '<h3 class="ha-sub">What changed</h3>' + partsHtml : "");
  }
  function undatedHtml(st) {
    var u = st.nav_counts && st.nav_counts.undated;
    if (!u || !u.count) { return ""; }
    return '<a class="ha-undated-a" href="#tasks/none">' + h(u.count === 1 ? "1 task needs a date" : u.count + " tasks need a date") + "</a>" +
      '<span class="ha-undated-h">' + h(KnowluFmt.minutes((Number(u.hours) || 0) * 60)) + "</span>";
  }
  function drawChanged(st) {
    var html = changedHtml(st);
    $("ha-changed").innerHTML = html;
    $("ha-changed").hidden = !html;
  }
  function drawWaiting(st) {
    var d = st.decisions || {};
    var clear = clearText(d);
    $("ha-counts").textContent = countsText(d);
    $("ha-clear").textContent = clear;
    $("ha-clear").hidden = !clear;
    KnowluSlots.deck($("ha-deck"));
    drawChanged(st);
    var html = undatedHtml(st);
    $("ha-undated").innerHTML = html;
    $("ha-undated").hidden = !html;
  }
  function closeInfo(id) {
    return KnowluData.call("close_info", { id: id }).then(function (env) {
      if (!env || !env.ok) { KnowluKit.toast(KnowluKit.refusal(env), null, null, true); }
    });
  }
  /* From your email: asked once a mount, only when a mailbox is set up (the shell's poll keeps mail_status) */
  function loadMailApplied() {
    var ms = KnowluData.last("mail_status");
    if (MAIL_ASKED || !ms || !Array.isArray(ms.mailboxes) || !ms.mailboxes.length) { return; }
    MAIL_ASKED = true;
    KnowluData.call("mail_applied", {}).then(function (r) {
      MAIL = r && r.ok !== false && Array.isArray(r.courses) ? r : null;
      if (ST && WAIT) { drawChanged(ST); }
    });
  }

  /* ---- today's schedule ---- */
  function laneHtml(st) {
    var d = st.the_day || {};
    var uids = Array.isArray(d.all_day_uids) ? d.all_day_uids : [];
    var rowsHtml = "";
    (Array.isArray(d.all_day) ? d.all_day : []).forEach(function (t, i) {
      var uid = uids[i];
      if (!uid) {
        rowsHtml += '<li class="ha-sc ha-allday"><i class="mk" aria-hidden="true"></i><span class="ha-sc-nm">' + h(t) + '</span><span class="ha-sc-t">All day</span></li>';
        return;
      }
      if (PENDING_LANE[uid]) { return; }
      rowsHtml += '<li class="ha-sc ha-allday" data-uid="' + h(uid) + '"><i class="mk" aria-hidden="true"></i><span class="ha-sc-nm">' + h(t) + '</span><span class="ha-sc-t">All day</span>' +
        '<button class="tbtn ha-lane-rm" type="button" data-lane-remove="' + h(uid) + '" data-title="' + h(t) + '">Remove from my day</button></li>';
    });
    return rowsHtml;
  }
  function timedOf(st) {
    var d = st.the_day || {};
    return (Array.isArray(d.blocks) ? d.blocks : []).filter(function (b) { return b.kind === "class" || b.kind === "busy"; });
  }
  function pastCount(st) {
    var now = nowHM(st);
    return now ? timedOf(st).filter(function (b) { return b.end <= now; }).length : 0;
  }
  function blockHtml(b, past, next) {
    var k = b.kind === "class" ? KnowluCourses.match(b.label) : null;
    var slot = KnowluCourses.slot(k);
    var name = b.label === "class" ? "Class" : b.label;
    return '<li class="ha-sc' + (slot ? " " + h(slot) : "") + (past ? " is-past" : "") + (next ? " is-next" : "") + '" data-from="' + h(b.start) + '" data-to="' + h(b.end) + '"' + (past && !EARLY_OPEN ? " hidden" : "") + ">" +
      '<i class="mk" aria-hidden="true"></i><span class="ha-sc-nm">' + h(name) + "</span>" +
      '<span class="ha-sc-t">' + h(KnowluFmt.span(stamp(b.start), stamp(b.end)) || b.start + " to " + b.end) + "</span></li>";
  }
  function schedHtml(st) {
    var d = st.the_day || {};
    var timed = (Array.isArray(d.blocks) ? d.blocks : []).filter(function (b) { return b.kind === "class" || b.kind === "busy"; });
    var past = pastCount(st);
    var first = (Array.isArray(d.blocks) && d.blocks.length) ? d.blocks[0].start : "";
    var now = nowHM(st);
    var nowHtml = (now && first && now >= first && past < timed.length ? '<li class="ha-now" aria-hidden="true"><span class="ha-now-t">' + h(KnowluFmt.clock(KnowluFmt.now())) + "</span></li>" : "");
    var rowsHtml = laneHtml(st);
    timed.forEach(function (b, i) {
      if (i === past) {
        rowsHtml += nowHtml;
      }
      rowsHtml += blockHtml(b, i < past, i === past);
    });
    return rowsHtml || emptyHtml(d.empty_text);
  }
  function emptyHtml(text) {
    return (text ? '<li class="ha-empty">' + h(text) + "</li>" : "");
  }

  /* ---- free time ---- */
  function takeHtml(t) {
    var m = KnowluFmt.minutes((Number(t.hours) || 0) * 60);
    var part = Number(t.of_hours) > (Number(t.hours) || 0) + 0.01 ? m + " of " + KnowluFmt.minutes(Number(t.of_hours) * 60) : m;
    return '<li class="ha-take"><span class="ha-take-nm">' + h(t.title) + '</span><span class="ha-take-h">' + h(part) + "</span></li>";
  }
  function freeRowHtml(b) {
    var takes = Array.isArray(b.takes) ? b.takes : [];
    return '<li class="ha-free-row" data-from="' + h(b.start) + '" data-to="' + h(b.end) + '"><span class="ha-free-when">' + h(KnowluFmt.span(stamp(b.start), stamp(b.end)) || b.start + " to " + b.end) + "</span>" +
      '<span class="ha-free-h">' + h(KnowluFmt.minutes((Number(b.hours) || 0) * 60)) + "</span>" +
      (takes.length ? '<ul class="ha-takes">' + takes.map(takeHtml).join("") + "</ul>" : "") + "</li>";
  }
  function freeHtml(st) {
    var d = st.the_day || {};
    var frees = (Array.isArray(d.blocks) ? d.blocks : []).filter(function (b) { return b.kind === "free"; });
    var lead = KnowluFmt.minutes((Number(d.open_hours) || 0) * 60) + " free";
    return '<p class="ha-free-lead">' + h(lead) + "</p>" + '<ul class="ha-free-rows">' + frees.map(function (b) { return freeRowHtml(b); }).join("") + "</ul>";
  }

  function drawDay(st) {
    if (!DAY || !st) { return; }
    DAY_DATE = String(st.now || "").slice(0, 10);
    var past = pastCount(st);
    var early = DAY.querySelector("[data-early]");
    early.hidden = !past;
    early.textContent = past ? past + " earlier" : "";
    early.setAttribute("aria-expanded", EARLY_OPEN ? "true" : "false");
    $("ha-sched").innerHTML = schedHtml(st);
    $("ha-free").innerHTML = freeHtml(st);
  }

  /* Remove from my day: hidden at once; the call waits behind the ten-second toast, so Undo never reaches the engine.
     A refusal puts the date back and says the engine's sentence. */
  function removeFromMyDay(uid, title) {
    if (!uid || PENDING_LANE[uid]) { return; }
    PENDING_LANE[uid] = title || true;
    if (ST) { drawDay(ST); }
    var t = document.createElement("div");
    t.className = "toast";
    t.setAttribute("role", "status");
    t.setAttribute("data-lane-toast", "");
    var line = document.createElement("span");
    line.textContent = "Removed";
    var undo = document.createElement("button");
    undo.type = "button";
    undo.className = "tbtn";
    undo.setAttribute("data-lane-undo", "");
    undo.textContent = "Undo";
    t.appendChild(line);
    t.appendChild(undo);
    var host = $("toasts");
    if (host) { host.appendChild(t); }
    var gone = function () { if (t.parentNode) { t.parentNode.removeChild(t); } };
    var timer = setTimeout(function () {
      gone();
      KnowluData.call("remove_lane_date", { uid: uid }).then(function (env) {
        if (env && env.ok) { return; }
        delete PENDING_LANE[uid];
        if (ST) { drawDay(ST); }
        KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
      });
    }, LANE_MS);
    undo.addEventListener("click", function () {
      clearTimeout(timer);
      gone();
      delete PENDING_LANE[uid];
      if (ST) { drawDay(ST); }
    });
  }

  function onClick(e) {
    var t = e.target;
    var close = t.closest("[data-close-info]");
    if (close) {
      e.stopPropagation();
      closeInfo(close.getAttribute("data-close-info"));
      return;
    }
    var rm = t.closest("[data-lane-remove]");
    if (rm) {
      removeFromMyDay(rm.getAttribute("data-lane-remove"), rm.getAttribute("data-title"));
      return;
    }
    if (t.closest("[data-early]")) {
      EARLY_OPEN = !EARLY_OPEN;
      if (ST) { drawDay(ST); }
      return;
    }
    if (t.closest("[data-mail-applied]") && window.KnowluMail && typeof window.KnowluMail.openApplied === "function") { window.KnowluMail.openApplied(); }
  }

  window.KnowluHomeAside = {
    mount: function (waitEl, dayEl) {
      if (!waitEl || !dayEl) { return; }
      WAIT = waitEl;
      DAY = dayEl;
      MAIL = null;
      MAIL_ASKED = false;
      EARLY_OPEN = false;
      WAIT.innerHTML = WAIT_HTML;
      DAY.innerHTML = DAY_HTML;
      WAIT.addEventListener("click", onClick);
      DAY.addEventListener("click", onClick);
    },
    update: function (st) {
      if (!WAIT || !DAY || !st) { return; }
      ST = st;
      drawWaiting(st);
      drawDay(st);
      loadMailApplied();
    },
    unmount: function () {
      WAIT = null;
      DAY = null;
      ST = null;
    }
  };
})();
