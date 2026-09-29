/* Knowlu console — one render function per region, no framework, no bundler (S2 §11).
   Data comes from the engine through Tauri IPC: invoke("state", {view}) returns
   {ok, error, state}; the page never computes a number the engine did not send. */
(function () {
  "use strict";
  var tauri = window.__TAURI__ && window.__TAURI__.core;
  var current = { view: "today", revision: null, state: null, pendingOrder: null };
  var EL = function (id) { return document.getElementById(id); };
  var h = function (s) { return String(s == null ? "" : s).replace(/[&<>"']/g, function (c) { return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]; }); };
  var fmtH = function (x) { return (Math.round(x * 10) / 10).toFixed(1) + "h"; };
  // Task 13: the drawer's twelve editable fields — every one `set_fields` accepts but `kind` and
  // `level`, which it takes only on a commitment, from the Schedule view (commands.rs's EDITABLE).
  var EDITABLE = ["title", "course", "due", "effort_hours", "importance", "importance_reason", "status", "progress", "slice_hours", "domain", "rank_override", "effort_confidence"];
  var editing = null;

  function invoke(cmd, args) {
    if (!tauri) {
      // Dev-only: scripts/console-shots.py serves a frozen State so the page can be screenshotted
      // without the engine. Never reached inside Tauri, where __TAURI__ exists.
      var fx = new URLSearchParams(location.search).get("fixture");
      if (fx && cmd === "state") { return fetch(fx).then(function (r) { return r.json(); }).then(function (s) { return { ok: true, error: null, state: s }; }); }
      return Promise.reject(new Error("no engine: __TAURI__ is absent"));
    }
    return window.__TAURI__.core.invoke(cmd, args || {});
  }

  // R-C1b-exec-9: the wizard's subscribe panel and the upgrade overlay both need to ask this same
  // question — is this account entitled right now — and each used to carry its own copy of the
  // status test. One copy, resolved `true` only on `ok` plus an active or trialing status, `false`
  // on anything else, a refusal or a dropped invoke included: nobody downstream of this has to
  // remember to `.catch`.
  function checkEntitled() {
    return invoke("entitlement_now", {}).then(function (r) {
      return !!(r && r.ok && (r.status === "active" || r.status === "trialing"));
    }).catch(function () { return false; });
  }

  function renderNav(state) {
    var n = state.nav_counts;
    var map = { today: n.today, overdue: n.overdue, week: n.week, later: n.later, all: n.all, decisions: n.decisions, "good-to-know": n.good_to_know, issues: n.issues, runs: n.runs_warn };
    document.querySelectorAll("#navlinks a").forEach(function (a) {
      var v = a.getAttribute("data-view"), c = map[v];
      a.classList.toggle("on", v === current.view);
      var ct = a.querySelector(".ct");
      if (c === undefined) { ct.textContent = ""; return; }   // Schedule carries no count
      if (v === "runs") { ct.textContent = c.count ? c.count + " warn" : "ok"; }
      else if (c.hours) { ct.innerHTML = h(c.count) + "<small>" + h(fmtH(c.hours)) + "</small>"; }   // every count carries its hours
      else { ct.textContent = c.count; }
    });
  }

  // Plan 2 Task 7 (anatomy 3.5): fourteen runs of runway as a strip beside the runway
  // figure, and the engine's direction word. Nothing is computed here — the points, the
  // direction and the no-history phrase are all payload.
  function renderGauge(state) {
    var g = state.gauge, el = EL("gauge");
    if (!el) { return; }
    if (!g || g.direction === "no-history" || !g.points.length) { el.textContent = state.empty.gauge; el.className = "gauge calm"; return; }
    var lo = Math.min.apply(null, g.points.map(function (p) { return p.runway_days; }));
    var hi = Math.max.apply(null, g.points.map(function (p) { return p.runway_days; }));
    var span = Math.max(hi - lo, 1);
    el.className = "gauge " + (g.direction === "losing" ? "amber" : "calm");
    el.innerHTML = g.points.map(function (p) {
      return '<i title="' + h(p.date) + " \u00b7 runway " + p.runway_days + "d \u00b7 deficit " + p.deficit_hours + "h" + '" style="height:' + Math.round(20 + 80 * (p.runway_days - lo) / span) + '%"></i>';
    }).join("") + '<span class="word">' + h(g.direction) + "</span>";
  }

  function renderTopline(state) {
    var t = state.topline, parts = [];
    parts.push("<span>" + h(t.date) + "</span>");
    parts.push('<span class="rw ' + h(t.status_word) + '">runway ' + h(t.runway_days) + "d (" + h(t.status_word) + ")</span>");
    // Plan 2 Task 7: the gauge sits beside the runway figure. It is emitted EMPTY here and
    // filled by renderGauge, which runs after this in the paint sequence -- this function
    // rewrites the whole topline every paint, so anything renderGauge wrote would be
    // destroyed if the order were the other way round.
    parts.push('<span id="gauge" class="gauge"></span>');
    parts.push("<span>" + h(t.active) + " active</span>");
    parts.push("<span>" + (t.generated_at ? "generated " + h(t.generated_at) + " by " + h(t.generated_by) : "no run recorded") + "</span>");
    // Plan 4a Task 7: one of the panel's two ways in (the other is the tray). Last, so the
    // gear sits at the end of the line and never moves as the facts before it change width.
    parts.push('<button class="b" type="button" data-settings title="Settings">&#9881;</button>');
    EL("topline").innerHTML = parts.join("<span>&middot;</span>");
  }

  function renderDelta(state) {
    var d = state.delta, el = EL("delta");
    var since = d.since_kind === "seen" ? "since you last looked" : d.since_kind === "run" ? "since the last run" : "no history";
    var html = "<span>" + h(since) + (d.summary ? " — " + h(d.summary) : " — nothing changed") + "</span>";
    // Task 15: the summary line is a rollup — d.records (the same journal-derived lines
    // `describe` builds, capped by DELTA_RECORD_CAP) is what "delta_expanded" reveals, never
    // recomputed client-side.
    if (d.records && d.records.length) {
      html += ' <button id="delta-toggle" type="button">' + d.records.length + " change" + (d.records.length === 1 ? "" : "s") + "</button>"
        + '<div id="delta-detail" hidden>' + d.records.map(function (r) { return "<div>" + h(r.text) + "</div>"; }).join("") + (d.truncated ? '<div>&hellip; newest ' + d.records.length + ' shown</div>' : "") + "</div>";
    }
    if (current.pendingOrder) { html += '<button id="refresh-order">refresh order</button>'; }
    el.innerHTML = html;
    var b = EL("refresh-order");
    if (b) { b.addEventListener("click", function () { var s = current.pendingOrder; current.pendingOrder = null; paint(s, true); }); }
    var dt = EL("delta-toggle");
    if (dt) { dt.addEventListener("click", function () { var det = EL("delta-detail"), wasHidden = det.hidden; det.hidden = !det.hidden; if (wasHidden) { ev("delta_expanded", null, null, null); } }); }
  }

  function renderVerdict(state) {
    EL("headline").textContent = state.verdict.headline;
    EL("lede").textContent = state.verdict.lede;
  }

  function renderMeter(state) {
    var m = state.meter, fits = EL("fits"), spill = EL("spill"), i;
    fits.innerHTML = ""; spill.innerHTML = "";
    for (i = 0; i < m.fit_blocks; i++) { fits.appendChild(document.createElement("i")); }
    for (i = 0; i < m.spill_blocks; i++) { var b = document.createElement("i"); if (m.spill_items[i]) { b.title = m.spill_items[i].title + " · " + fmtH(m.spill_items[i].hours); } spill.appendChild(b); }
    EL("metersub").textContent = "1 block = 30 min · " + m.capacity_text;
    EL("meterkey").innerHTML = '<span class="a">&#9632; <b>' + h(m.fit_blocks) + " blocks</b> fit inside the day</span>" +
      '<span class="c">&#9633; <b>' + h(m.spill_blocks) + " blocks</b> do not</span>" +
      (m.capped ? "<span>… and " + h(m.capped) + " more</span>" : "") +
      (m.spill_hours ? "<span>" + h(fmtH(m.spill_hours)) + " across " + h(m.spill_items.length) + " items with nowhere to go</span>" : "");
    EL("overflow").textContent = state.verdict.overflow_sentence || "";
  }

  // A chip's displayed text is often a formatted string (a "P" prefix, an "h" suffix, a T/space
  // swap, an em-dash placeholder) — never the raw literal `set_fields` expects. `data-value`
  // carries that literal alongside the display text, so editField never has to reverse-parse it.
  function editChip(cls, field, value, displayHtml, extra) {
    return '<span class="' + cls + '" data-field="' + field + '" data-value="' + h(value == null ? "" : value) + '"' + (extra || "") + ">" + displayHtml + "</span>";
  }
  function hrsCell(hours, displayHtml) { return editChip("hrs", "effort_hours", hours, displayHtml); }

  function rowHtml(r) {
    var chips = '<span>' + h(r.source_tag) + "</span>";
    if (r.start_by) { chips += '<span class="sb' + (r.overdue_start ? " od" : "") + '">' + (r.overdue_start ? "OVERDUE START" : "start by " + h(r.start_by) + " · " + h(Math.round(r.slack_days)) + "d slack") + "</span>"; }
    else { chips += '<span class="sb">no due date</span>'; }
    // Task 13: small editable metadata chips — click-to-edit, never the descriptive text above
    // (which is a computed sentence, not a raw field the engine will accept back).
    chips += editChip("ed", "course", r.course, h(r.course || "—"), ' title="course"');
    chips += editChip("ed", "due", r.due, h(r.due ? r.due.replace("T", " ") : "no due date"), ' title="due"');
    chips += editChip("ed", "importance", r.importance, "P" + h(r.importance), ' title="importance 1–5"');
    chips += editChip("ed", "status", r.status, h(r.status), ' title="status"');
    if (r.oversized) { chips += '<span class="os">' + h(fmtH(r.hours)) + " in one piece</span>"; }
    if (r.amend_badge) { chips += '<span class="amend" title="' + h(r.amend_badge.proposal) + '">' + h(r.amend_badge.sentence) + "</span>"; }
    if (r.conflict_text) { chips += '<span class="cf">' + h(r.conflict_text) + "</span>"; }
    if (r.flag_text) { chips += '<span class="fl">' + h(r.flag_text) + "</span>"; }
    if (r.merge_text) { chips += '<span class="mg">' + h(r.merge_text) + "</span>"; }
    if (r.judged) { chips += '<button class="flag" data-flag="' + h(r.id) + '" title="AI-judged — flag it">&#9873;</button>'; }
    // The progress track carries no data-field of its own (bindProgressDrag owns pointer events
    // on it exclusively) — the "status" chip above is the click-to-edit surface for status text.
    var track = '<span class="track"><i style="width:' + h(r.progress_pct) + '%"></i></span>';
    return '<div class="row ' + h(r.urgency) + '" data-id="' + h(r.id) + '" data-kind="task"><span class="pip"></span><div class="ttl">' + editChip("a", "title", r.title, h(r.title)) + '<span class="b">' + chips + "</span></div>" + track + hrsCell(r.hours, h(fmtH(r.hours))) +
      '<div class="why" hidden>' + h(r.why) + "</div></div>";
  }

  // Task 13: rebind pointer-drag progress after every list render (paint rewrites innerHTML,
  // which drops any listeners bound to the previous DOM). Only real task rows (data-id present —
  // amendments/unreadable/commitment stub rows never carry one) get a track to drag.
  function bindTracks(container) {
    container.querySelectorAll(".row[data-id] > .track").forEach(function (t) { bindProgressDrag(t, t.parentElement.getAttribute("data-id")); });
  }

  // A 60s poll (or another write elsewhere) can rewrite a region out from under an open edit —
  // drop the stale reference so a leftover Escape/blur never touches a detached input.
  function clearStaleEditing(container) { if (editing && container.contains(editing.el)) { editing = null; } }

  // Task 14 review fix (Important, 2a): generalises the "an open form/popover must survive a
  // repaint" pattern Task 13 established for .newrow — a 60s poll or an unrelated write's forced
  // repaint must not silently destroy an open .flagpop (and whatever the user had typed into it)
  // out from under them. Detach every element matching `selector` first (remembering the data-id
  // of the row/card it belongs to, or null when it belongs to no row at all — .newrow sits
  // directly in host, not inside a row), run `rewrite` (which replaces host's innerHTML), then
  // re-attach each detached element — the SAME node, so its typed values and listeners survive —
  // onto whichever element now carries that same data-id, or back onto host when it had none. A
  // row that no longer exists after the rewrite (its task was decided, deleted, or moved off this
  // view) silently drops whatever was attached to it — it is not that row's business any more.
  function preserveAcross(host, selector, rewrite) {
    var saved = [];
    host.querySelectorAll(selector).forEach(function (el) {
      var owner = el.closest("[data-id]");
      saved.push({ el: el, id: owner ? owner.getAttribute("data-id") : null });
      el.remove();
    });
    rewrite();
    saved.forEach(function (s) {
      if (s.id == null) { host.prepend(s.el); return; }
      var target = host.querySelector('[data-id="' + s.id + '"]');
      if (target) { target.appendChild(s.el); }
    });
  }

  function renderMustDo(state) {
    var md = state.must_do, html = "";
    var host = EL("mustdo");
    preserveAcross(host, ".newrow, .flagpop", function () {
      EL("mustdo-n").textContent = md.total_count + " · " + fmtH(md.total_hours);
      md.groups.forEach(function (g) { html += '<div class="grp">' + h(g.label) + " — " + h(g.count) + " · " + h(fmtH(g.hours)) + "</div>" + g.rows.map(rowHtml).join(""); });
      md.amendments.forEach(function (a) { html += '<div class="row soon"><span class="pip"></span><div class="ttl"><span class="a">&#9888; PENDING AMENDMENT — ' + h(a.target_title) + '</span><span class="b"><span>' + h(a.age_days) + "d unactioned</span>" + (a.due_move ? "<span>due " + h(a.due_move) + (a.stale ? " — stale: the note now reads a different date" : "") + "</span>" : "") + "<span>&rarr; " + h(a.proposal_path) + '</span></span></div><span class="track none"></span><span class="hrs"></span></div>'; });
      // F7/spec §12: an unreadable task note is silently absent from Today unless surfaced here —
      // state.unreadable (top level) is the same list render_list already shows on `all` for
      // l.unreadable; a would-have-been-must-do note deserves the same visibility.
      state.unreadable.forEach(function (p) { html += '<div class="row unreadable"><span class="pip"></span><div class="ttl"><span class="a">unreadable: ' + h(p) + '</span></div><span class="track none"></span><span class="hrs"></span></div>'; });
      if (md.empty_text) { html += '<div class="empty">' + h(md.empty_text) + "</div>"; }
      clearStaleEditing(host);
      host.innerHTML = html;
    });
    bindTracks(host);
  }

  function renderRecommended(state) {
    var r = state.recommended, html = "";
    var host = EL("recommended");
    preserveAcross(host, ".flagpop", function () {
      EL("recommended-n").textContent = fmtH(r.spare_hours) + " spare";
      r.commitments.forEach(function (c) { html += '<div class="row"><span class="pip"></span><div class="ttl"><span class="a">' + h(c.name) + '</span><span class="b"><span>recurring</span></span></div><span class="track none"></span><span class="hrs">' + h(fmtH(c.hours)) + "</span></div>"; });
      // The row's own effort_hours cell shows a partial take_hours here, not the full editable
      // total — the swap still edits the full effort_hours (data-value stays x.row.hours), only
      // the displayed label changes, so it goes through the same hrsCell generator as rowHtml.
      r.rows.forEach(function (x) {
        var row = rowHtml(x.row);
        var display = h(fmtH(x.take_hours)) + (x.of_hours - x.take_hours > 0.005 ? " <small>of " + h(fmtH(x.of_hours)) + "</small>" : "");
        html += row.replace(hrsCell(x.row.hours, h(fmtH(x.row.hours))), hrsCell(x.row.hours, display));
      });
      if (r.empty_text) { html += '<div class="empty">' + h(r.empty_text) + "</div>"; }
      clearStaleEditing(host);
      host.innerHTML = html;
    });
    bindTracks(host);
  }

  function renderList(state) {
    var l = state.list; if (!l) { return; }
    var host = EL("list");
    preserveAcross(host, ".flagpop", function () {
      var titles = { overdue: "Overdue", week: "This week", later: "Later", undated: "No due date", all: "Everything active" };
      EL("list-title").textContent = titles[l.horizon] || l.horizon;
      EL("list-n").textContent = l.count + " · " + fmtH(l.hours);
      var html = l.rows.map(rowHtml).join("");
      l.unreadable.forEach(function (p) { html += '<div class="row unreadable"><span class="pip"></span><div class="ttl"><span class="a">unreadable: ' + h(p) + '</span></div><span class="track none"></span><span class="hrs"></span></div>'; });
      if (l.empty_text) { html += '<div class="empty">' + h(l.empty_text) + "</div>"; }
      clearStaleEditing(host);
      host.innerHTML = html;
    });
    bindTracks(host);
  }

  function renderTheDay(state) {
    var d = state.the_day;
    EL("dayopen").textContent = fmtH(d.open_hours) + " open";
    EL("allday").innerHTML = d.all_day.map(function (t) { return "<div>(all day) " + h(t) + "</div>"; }).join("");
    EL("theday").innerHTML = d.blocks.map(function (b) {
      var takes = b.takes.map(function (t) { return '<span class="sl"><span>' + h(t.title) + "</span><span>" + h(fmtH(t.hours)) + (t.of_hours - t.hours > 0.005 ? " of " + h(fmtH(t.of_hours)) : "") + "</span></span>"; }).join("");
      return '<div class="blk ' + h(b.kind) + '"><span class="t">' + h(b.start) + "–" + h(b.end) + '</span><span class="w">' + h(b.label) + '</span><span class="h">' + h(fmtH(b.hours)) + "</span>" + takes + "</div>";
    }).join("") + (d.empty_text ? '<div class="empty">' + h(d.empty_text) + "</div>" : "");
    EL("commitments").innerHTML = d.commitments.map(function (c) { return '<div class="commit"><span>' + h(c.name) + " · recurring</span><span>" + h(fmtH(c.hours)) + "</span></div>"; }).join("");
  }

  function renderDeck(state) {
    var d = state.decisions, deck = EL("deck");
    if (!d.pending && !d.events_in_digest) {
      // R31: "clear" only when both the deck and the digest are empty — a digest with
      // pending===0 still has decisions waiting, just not as cards.
      EL("dcount").textContent = "clear";
    } else {
      var parts = [d.pending + " · oldest " + d.oldest_days + "d · " + d.budget_used + " of " + d.budget_total + " today"];
      if (d.deferred) { parts.push(d.deferred + " deferred"); }
      if (d.awaiting_calendar) { parts.push(d.awaiting_calendar + " awaiting calendar"); }
      if (d.events_in_digest) { parts.push(d.events_in_digest + " in digest"); }
      EL("dcount").textContent = parts.join(" · ");
    }
    // Task 14 review fix (Important, 2b): the note field is live now — a repaint from an
    // unrelated write or the 60s poll must not silently discard whatever Quinn was typing about
    // THIS same card. renderDeck always wipes deck.innerHTML below, so the value has to be read
    // out first and restored after — but only onto the same card (same data-id); a card that
    // moved into the top slot because the previous one was decided never inherits a note that
    // was never about it.
    // Task 15 (carried residual from Task 14): the same applies to an open `.flagpop` on the
    // top card — detach it before the wipe (the SAME node, so its checked chips and typed text
    // survive), re-append it onto the new top card only when that card carries the same
    // data-id. A card that no longer exists after the rewrite (decided, or a different card
    // moved into the top slot) silently drops it, same as preserveAcross does elsewhere.
    var prevCard = deck.querySelector(".card[data-id]");
    var prevId = prevCard ? prevCard.getAttribute("data-id") : null;
    var prevNoteInput = prevCard ? prevCard.querySelector(".nb input") : null;
    var prevNoteVal = prevNoteInput ? prevNoteInput.value : "";
    var prevFlagpop = prevCard ? prevCard.querySelector(".flagpop") : null;
    if (prevFlagpop) { prevFlagpop.remove(); }
    deck.innerHTML = "";
    if (!d.cards.length) {
      // R19: a pending events-digest is N decisions waiting, not a card — empty_text is
      // null in that case (only truly clear when both cards and the digest are empty).
      var msg = d.empty_text ? d.empty_text : (d.events_in_digest + (d.events_in_digest === 1 ? " event" : " events") + " in today's digest");
      deck.innerHTML = '<div class="clear">' + h(msg) + "</div>";
      EL("deckrest").textContent = ""; EL("deckhint").textContent = "";
      return;
    }
    // The deck's own overflow "…" snooze date always defaults to tomorrow — read from
    // state.ahead (buckets[0] is today, buckets[1] today+1) rather than computed client-side,
    // so it can never disagree with the engine's own idea of "today" across a timezone.
    var tomorrow = state.ahead.buckets[1].date;
    d.cards.slice(0, 1).forEach(function (c) {
      // Q11-b: a commitment-ask card is answered in the Decisions view's form, not approved here.
      var approve = c.kind === "commitment-ask" ? '<button class="b pri y" type="button" data-answer-in>Answer&hellip;</button>' : '<button class="b pri y" data-verdict="approved">Approve</button>';
      deck.innerHTML += '<div class="card" data-id="' + h(c.id) + '" data-kind="approval"><button class="flag" data-flag="' + h(c.id) + '" title="agent-authored — flag it">&#9873;</button><div class="t">' + h(c.title) + '</div><div class="w">' + h(c.why || c.kind) + "</div>" +
        '<div class="nb"><input type="text" placeholder="Note (optional)"></div><div class="acts">' + approve + '<button class="b n" data-verdict="rejected">Reject</button><button class="b" data-verdict="snoozed">&hellip;</button><input type="date" hidden value="' + h(tomorrow) + '"></div></div>';
    });
    var newCard = deck.querySelector(".card[data-id]");
    if (newCard && prevId && newCard.getAttribute("data-id") === prevId) {
      if (prevNoteVal) { var newNoteInput = newCard.querySelector(".nb input"); if (newNoteInput) { newNoteInput.value = prevNoteVal; } }
      if (prevFlagpop) { newCard.appendChild(prevFlagpop); }
    }
    for (var i = 1; i < Math.min(3, d.cards.length); i++) { var s = document.createElement("div"); s.className = "sliver s" + i; deck.appendChild(s); }
    EL("deckrest").textContent = d.cards.length > 1 ? (d.cards.length - 1) + " behind" : "last one";
    EL("deckhint").textContent = "approve, reject, or snooze — one click commits";
  }

  function renderAhead(state) {
    var a = state.ahead, max = Math.max(a.peak.hours, 0.1);
    EL("ribbon").innerHTML = a.buckets.map(function (b) {
      var cls = "d" + (b.weekend ? " wknd" : "") + (b.hours === 0 ? " zero" : "") + (b.date === a.peak.date && b.hours > 0 ? " pk" : "");
      return '<div class="' + cls + '" title="' + h(b.date) + " · " + h(fmtH(b.hours)) + '"><i style="height:' + Math.round(100 * b.hours / max) + '%"></i>' + (b.date === a.peak.date && b.hours > 0 ? '<span class="pklabel">' + h(fmtH(b.hours)) + "</span>" : "") + "</div>";
    }).join("");
    EL("ribbonax").innerHTML = a.buckets.map(function (b) { return "<span" + (b.date === a.peak.date ? ' class="pk"' : "") + ">" + h(b.weekday.charAt(0)) + "</span>"; }).join("");
    EL("aheadnote").innerHTML = "<span>&#9888;</span><span>" + h(a.takeaway) + (a.undated_hours ? " " + h(fmtH(a.undated_hours)) + " undated is not on this strip." : "") + "</span>";
  }

  function renderComingUp(state) {
    var cu = state.coming_up;
    EL("cu-n").textContent = cu.length ? cu.length + " accepted" : "";
    EL("cu").innerHTML = cu.length ? cu.map(function (e) { return '<div class="ln"><span class="k lp">' + h(e.when) + "</span><span>" + h(e.title) + (e.location ? " · " + h(e.location) : "") + '</span><span class="rt">' + h(e.organizer) + "</span></div>"; }).join("") : '<div class="empty">' + h(state.texts.coming_up) + "</div>";
  }

  function renderGoodToKnow(state) {
    var g = state.good_to_know;
    EL("gtk-n").textContent = g.length ? g.length + " open" : "";
    EL("gtk").innerHTML = g.length ? g.map(function (i) { return '<div class="ln"><span class="k lp">' + h(i.kind) + "</span><span>" + h(i.title) + '</span><span class="rt">' + h(i.expires || "") + '</span><button class="b" data-close-info="' + h(i.id) + '">close</button></div>'; }).join("") : '<div class="empty">' + h(state.texts.info) + "</div>";
  }

  function renderClosed(state) {
    var c = state.closed_this_week;
    EL("closed-n").textContent = c.length;
    EL("closed").innerHTML = c.length ? c.map(function (x) { var k = x.mark === "me" ? "me" : x.mark === "agent" ? "ag" : "lp"; return '<div class="ln"><span class="k ' + k + '">' + h(x.mark === "me" ? "you" : x.mark) + "</span><span>" + h(x.title) + '</span><span class="rt">' + h(x.when.slice(11)) + "</span></div>"; }).join("") : '<div class="empty">' + h(state.texts.closed) + "</div>";
  }

  // Plan 4a Task 8: the offer. Rendered only when the engine says there is one — the mid-run
  // gate is `updates::update_offer` in Rust, never a check here (R9).
  function renderUpdateOffer(u) {
    var el = EL("upd");
    current.version = u.version;
    // M8: version, last check, staged-or-not, error — and no release track is named, because
    // there is only one. (The word the settings test forbids in this file is the one M8 rules out.)
    // `u.held` is a bundle the RUN is holding back (R-P4a-24): downloaded, verified, and
    // deliberately not offered until the slot ends. Saying "up to date" there would be a lie about
    // a file already sitting on this machine — so it gets its own words, and the offer stays away.
    var when = u.last_check ? ", checked " + u.last_check.slice(0, 16).replace("T", " ") : "";
    var what = u.staged ? " — " + u.staged + " ready"
      : u.held ? " — " + u.held + " staged; installs after the run"
      : u.last_error ? " — " + u.last_error.split("\n")[0]
      : " — up to date";
    // The last ACTION's outcome rides as its own suffix, never folded into `what` (fix round 1,
    // IMPORTANT 2): a tray refusal or a failed install has to stay legible next to "up to date",
    // which is a true statement about the CHECK and says nothing about the install that failed.
    var act = u.action_error ? " · last action: " + u.action_error.split("\n")[0] : "";
    current.updateText = "version " + u.version + what + when + act;
    if (u.staged) {
      el.innerHTML = '<span class="amber">Knowlu ' + h(u.staged) + ' is ready.</span> <button class="b pri y" type="button" data-install>Restart to update</button>';
      el.hidden = false;
    } else {
      // Cleared BEFORE it is hidden (R-P4a-24). A hidden element still holding the last offer
      // would flash a version that is no longer staged the moment anything unhid it — and leaves a
      // dead `[data-install]` in the DOM for the click handler to find.
      el.innerHTML = "";
      el.hidden = true;
    }
  }
  function checkForUpdates() {
    return invoke("check_for_updates", {}).then(function (u) { renderUpdateOffer(u); if (!EL("settings").hidden) { EL("set-update-state").textContent = current.updateText; } }).catch(function () {});
  }

  // Task 15: the topline's sync/backup/scheduler line, never computed here. Every source object
  // (t.sync, t.backup, t.scheduler) can be ABSENT (a fixture predating Task 10/12's caches, or a
  // vault with no scheduler info yet) — `|| {}` and truthiness checks below must never throw on a
  // missing key.
  // C3′ final fix wave (R-C3′-exec-39 N1 and N4, R-C3′-exec-43): the engine's own words for a sync
  // that was skipped (`SyncStatus.skipped`) or failed (`last_error`), and what a student reads for
  // each. The words come from `cloudmodel::Unavailable::label` and the literals in `sync.rs`
  // (`run_lines_with`, `record_gated_skip`, `SyncError::service`); `static_assets.rs` reads those
  // sources and fails if one of them is not a key here. Amber is a state that keeps changes on this
  // computer until the student acts or the network returns; calm is an ordinary one. "no
  // entitlement" is also what a PAYING student reads after more than 72 hours offline (a cache
  // nothing could renew), so it says what is known — the subscription could not be confirmed — and
  // never that it is inactive. The diagnostics blob and the issue report keep the raw word.
  var SYNC_SAYS = {
    "no session": ["amber", "signed out — sign in to sync"],
    "signed out": ["amber", "signed out — sign in to sync"],
    "no entitlement": ["amber", "can't confirm your subscription — changes stay on this computer"],
    "offline: the account could not be reached": ["amber", "offline — changes stay on this computer"],
    "no account": ["calm", "sync skipped — no account"],
    "another sync is running": ["calm", "sync skipped — another sync is running"]
  };
  /// One engine word → `[tone, text]`. A word the table does not know keeps the engine's own first
  /// line, after `prefix`, in `tone`: a new failure is still shown, just not yet translated.
  function syncSays(word, tone, prefix) {
    var w = String(word).split("\n")[0];
    return Object.prototype.hasOwnProperty.call(SYNC_SAYS, w) ? SYNC_SAYS[w] : [tone, (prefix || "") + w];
  }
  /// *Sync now*'s refusal says what the sync line says, never the engine's raw word.
  function syncRefusal(err) { showRefusal(null, syncSays(err, "amber")[1]); }
  // C3', Task 10 (fix round 1, review I1): `t.sync` is the engine's own `SyncStatus` (state.rs
  // fills it from `sync::run_lines_with`), never a git-shaped status any more. `SyncStatus::of`
  // stamps `at` on every run, skipped ones included, so `s.skipped` MUST be read before `s.at` —
  // otherwise a lapsed subscription or a signed-out machine reads as "in step with your account"
  // for as long as the state lasts. Both fields go through `SYNC_SAYS` above.
  function renderSyncLine(state) {
    var t = state.topline, s = t.sync || {}, b = t.backup || {}, bits = [], said = null;
    if (s.last_error) { said = syncSays(s.last_error, "amber"); bits.push('<span class="' + said[0] + '">' + h(said[1]) + "</span>"); }
    else if (s.skipped) {
      // A skip word the table does not know is an ordinary state until it is added, so calm.
      said = syncSays(s.skipped, "calm", "sync skipped — ");
      bits.push('<span class="' + said[0] + '">' + h(said[1]) + "</span>");
    }
    else if (s.at) { bits.push('<span class="calm">in step with your account</span>'); }
    else { bits.push('<span class="calm">not synced yet</span>'); }
    if (b.last_error) { bits.push('<span class="amber">backup: ' + h(b.last_error.split("\n")[0]) + "</span>"); }
    else if (b.behind_days != null && b.behind_days >= 1) { bits.push('<span class="amber">backup ' + b.behind_days + " day" + (b.behind_days === 1 ? "" : "s") + " behind</span>"); }
    else if (b.last_ok) { bits.push('<span class="calm">backed up</span>'); }
    if (t.startup_missed > 0) { bits.push('<span class="amber">' + t.startup_missed + " slot" + (t.startup_missed === 1 ? "" : "s") + " missed while quit</span>"); }
    // R-T15d: topline.last_slot may be absent (no scheduler slot has run yet, or a hard-error
    // envelope) — tolerate it missing entirely, same as scheduler above.
    if (t.last_slot) {
      // Both stamps can be absent (the "a slot is already running" placeholder summary carries
      // neither) — then there is no time to show, and the old code rendered "last slot  ok" with a
      // hole in it. And a REFUSED slot is not a failed one: the engine was never invoked, so it
      // says refused and says why (final fix wave, C5 — `RunSummary.reason`, filled by B6).
      var when = (t.last_slot.ended || t.last_slot.started || "").slice(11, 16);
      var why = t.last_slot.reason;
      var tries = t.last_slot.attempts || 0;
      var body = "last slot" + (when ? " " + when : "") + (why ? " refused — " + why : (t.last_slot.ok ? " ok" : " failed")) + (tries > 1 ? " (attempt " + tries + ")" : "");
      bits.push('<span class="' + (t.last_slot.ok && !why ? "calm" : "amber") + '">' + h(body) + "</span>");
    }
    if (t.scheduler && t.scheduler.mode === "app") { bits.push('<span class="' + (t.scheduler.paused ? "amber" : "calm") + '">scheduler ' + (t.scheduler.paused ? "paused" : "on") + "</span>"); }
    EL("syncline").innerHTML = bits.join(" · ") + ' <button class="b" type="button" data-sync>sync now</button> <button class="b" type="button" data-backup>back up now</button>';
  }

  // Task 15: the Runs nav entry, made real — every expected slot, every run in the last 48h
  // with its FULL summary (never truncated the way the aside's compact #runs panel is), then
  // runs_panel.warnings + the page-wide state.warnings, then empty_text last.
  function renderRunsView(state) {
    var p = state.runs_panel, html = "";
    html += p.expected.map(function (e) {
      var k = e.status === "seen" ? "gd" : e.status === "late" ? "wn" : "ms";
      return '<div class="ln run"><span class="k ' + k + '">' + h(e.status) + '</span><span>' + h(e.runner) + '</span><span class="rt">' + h(e.due) + "</span></div>";
    }).join("");
    html += p.recent.map(function (r) {
      var k = r.result === "ok" ? "gd" : r.result === "running" ? "lp" : r.result === "WARN" ? "wn" : "ms";
      return '<div class="ln run"><span class="k ' + k + '">' + h(r.result.toLowerCase()) + '</span><span>' + h(r.runner) + " " + h((r.ended || r.started || "").slice(0, 16)) + '</span><span class="rt">' + h(r.run_id) + '</span><pre class="summary">' + h(r.summary) + "</pre></div>";
    }).join("");
    var warns = (p.warnings || []).concat(state.warnings || []);
    if (warns.length) { html += '<div class="warnings">' + warns.map(function (w) { return "<div>" + h(w) + "</div>"; }).join("") + "</div>"; }
    if (p.empty_text) { html += '<div class="empty">' + h(p.empty_text) + "</div>"; }
    EL("runs-view").innerHTML = html;
  }

  // Plan 2 Task 1 (S2 §6.2 B4, F8): EVERY pending approval as a list — the deck shows one card
  // and hides nothing but the events digest; this view is the whole queue, amendments of every
  // urgency included, so the "N hidden" count on the deck's header has somewhere to point.
  function renderDecisionsView(state) {
    var d = state.decisions, host = EL("dec-list");
    var parts = [d.pending + " pending"];
    if (d.pending) { parts.push("oldest " + d.oldest_days + "d"); }
    parts.push(d.budget_used + " of " + d.budget_total + " today");
    if (d.deferred) { parts.push(d.deferred + " deferred"); }
    if (d.awaiting_calendar) { parts.push(d.awaiting_calendar + " awaiting calendar"); }
    if (d.events_in_digest) { parts.push(d.events_in_digest + (d.events_in_digest === 1 ? " event" : " events") + " in digest"); }
    if (d.hidden_amendments) { parts.push(d.hidden_amendments + " amendment" + (d.hidden_amendments === 1 ? "" : "s") + " the deck does not surface"); }
    EL("dec-n").textContent = parts.join(" · ");
    EL("dec-hint").textContent = d.cards.length ? "approve, reject, or snooze — one click commits" : "";
    // Typed notes survive a repaint, per id (the deck's own rule, R-T14 2b).
    var notes = {};
    host.querySelectorAll(".row.dec[data-id]").forEach(function (r) { var n = r.querySelector(".dnote"); if (n && n.value) { notes[r.getAttribute("data-id")] = n.value; } });
    var tomorrow = state.ahead.buckets[1].date;
    preserveAcross(host, ".flagpop", function () {
      host.innerHTML = d.cards.map(function (c) {
        if (c.kind === "commitment-ask") { return askRow(c, tomorrow); }
        var changes = "";
        if (c.changes && typeof c.changes === "object" && !Array.isArray(c.changes)) {
          changes = '<div class="changes">' + Object.keys(c.changes).map(function (k) {
            var v = c.changes[k]; return '<span class="k lp">' + h(k) + "</span> → " + h(typeof v === "object" ? JSON.stringify(v) : String(v));
          }).join(" · ") + "</div>";
        }
        var meta = [c.kind, c.age_days + "d old"];
        if (c.urgency) { meta.push("urgency " + c.urgency); }
        if (c.target) { meta.push("→ " + c.target); }
        if (c.expires) { meta.push("expires " + c.expires); }
        if (c.snooze_until) { meta.push("snoozed to " + c.snooze_until); }
        return '<div class="row dec" data-id="' + h(c.id) + '" data-kind="approval">' +
          '<button class="flag" data-flag="' + h(c.id) + '" title="agent-authored — flag it">&#9873;</button>' +
          '<div class="ttl"><span class="a">' + h(c.title) + '</span><span class="meta">' + h(meta.join(" · ")) + "</span>" +
          '<div class="why">' + h(c.why) + "</div>" + changes + "</div>" +
          '<div class="acts"><input type="text" class="dnote" placeholder="Note (optional)" value="' + h(notes[c.id] || "") + '">' +
          '<button class="b pri y" data-verdict="approved">Approve</button><button class="b n" data-verdict="rejected">Reject</button>' +
          '<button class="b" data-verdict="snoozed" data-snooze="' + h(tomorrow) + '">Snooze</button></div></div>';
      }).join("") + (d.empty_text && !d.cards.length ? '<div class="empty">' + h(d.empty_text) + "</div>" : "");
    });
  }

  // Phase 2 (spec §5): a `commitment-ask` card is answered here. Day toggles, a start and an end
  // picker, "add another time", Save times (answer_card), No set times (reject), Snooze. The
  // engine validates the answer; an invalid one comes back as its message, shown on the card.
  function askTimeRow() {
    return '<div class="ask-time">' + DAYS.map(function (d) {
      return '<button class="b day" type="button" data-ask-day="' + d + '" aria-pressed="false">' + DAY_NAMES[d] + "</button>";
    }).join("") + '<input type="time" data-part="start" aria-label="start"><input type="time" data-part="end" aria-label="end"></div>';
  }
  function askRow(c, tomorrow) {
    return '<div class="row dec ask" data-id="' + h(c.id) + '" data-kind="approval">' +
      '<button class="flag" data-flag="' + h(c.id) + '" title="agent-authored — flag it">&#9873;</button>' +
      '<div class="ttl"><span class="a">' + h(c.title) + '</span><span class="meta">' + h(c.age_days + "d old") + "</span>" +
      '<div class="why">' + h(c.why) + '</div><div class="ask-times">' + askTimeRow() + "</div>" +
      '<button class="lnk" type="button" data-ask-more>add another time</button><div class="ask-err" hidden></div></div>' +
      '<div class="acts"><button class="b pri y" type="button" data-ask-save>Save times</button>' +
      '<button class="b n" data-verdict="rejected">No set times</button>' +
      '<button class="b" data-verdict="snoozed" data-snooze="' + h(tomorrow) + '">Snooze</button></div></div>';
  }
  function answerAsk(row) {
    var id = row.getAttribute("data-id"), meets = [];
    row.querySelectorAll(".ask-time").forEach(function (t) {
      var days = [];
      t.querySelectorAll('[data-ask-day][aria-pressed="true"]').forEach(function (b) { days.push(b.getAttribute("data-ask-day")); });
      var s = t.querySelector('[data-part="start"]').value, e = t.querySelector('[data-part="end"]').value;
      if (days.length || s || e) { meets.push({ days: days, start: s, end: e }); }
    });
    row.querySelectorAll("button").forEach(function (b) { b.disabled = true; });
    return invoke("answer_card", { view: stateView(), id: id, meets: meets }).then(function (env) {
      ev("decision_made", id, "approval", null);
      applyEnvelope(env, function (m) {
        // A refusal with no state repaints nothing: this same row takes the answer again.
        row.querySelectorAll("button").forEach(function (b) { b.disabled = false; });
        var fresh = EL("dec-list").querySelector('.row.dec[data-id="' + id.replace(/"/g, "") + '"] .ask-err');
        if (fresh) { fresh.textContent = m; fresh.hidden = false; } else { showRefusal(null, m, id); }
      });
    }).catch(function () { row.querySelectorAll("button").forEach(function (b) { b.disabled = false; }); });
  }

  // Plan 2 Task 2 (S2 §6.2 B11, F9): open info items as a list with the close action the rail
  // already has. Rows are observed as "info" (R-T15b) — the kind that had no template.
  function renderGoodToKnowView(state) {
    var g = state.good_to_know, host = EL("gtk-list");
    EL("gtkv-n").textContent = g.length ? g.length + " open" : "";
    preserveAcross(host, ".flagpop", function () {
      host.innerHTML = g.map(function (i) {
        var meta = [i.kind];
        if (i.opened_at) { meta.push("since " + i.opened_at.slice(0, 10)); }
        if (i.expires) { meta.push("expires " + i.expires); }
        if (i.close_key) { meta.push("closes on " + i.close_key); }
        return '<div class="row gtk-row" data-id="' + h(i.id) + '" data-kind="info">' +
          '<button class="flag" data-flag="' + h(i.id) + '" title="agent-authored — flag it">&#9873;</button>' +
          '<div class="ttl"><span class="a">' + h(i.title) + '</span><span class="meta">' + h(meta.filter(Boolean).join(" · ")) + "</span></div>" +
          '<div class="acts"><button class="b" data-close-info="' + h(i.id) + '">Close</button></div></div>';
      }).join("") + (g.length ? "" : '<div class="empty">' + h(state.texts.info) + "</div>");
    });
  }

  // Plan 2 Task 3 (S2 §6.2 B10, F9): open issue notes — the ⚑ flags — with their categories, the
  // object they were raised on (opens the drawer), and a resolution. S3's duplicate side-by-side
  // resolution is not built (deferred with S3; console spec §13).
  function renderIssuesView(state) {
    var p = state.issues_panel, host = EL("iss-list");
    EL("issv-n").textContent = p.open_count ? p.open_count + " open" : "";
    preserveAcross(host, ".flagpop", function () {
      host.innerHTML = p.rows.map(function (r) {
        var chips = (r.categories || []).map(function (c) { return '<span class="chip on">' + h(c) + "</span>"; }).join("");
        var when = r.opened_at ? "raised " + r.opened_at.slice(0, 10) : "";
        return '<div class="row iss" data-id="' + h(r.id) + '" data-kind="issue">' +
          '<div class="ttl"><span class="a">' + h(r.title) + '</span><span class="meta">' + h(when) + "</span><div class=\"chips\">" + chips + "</div>" +
          (r.target ? '<button class="lnk" data-open-target="' + h(r.target) + '">open the object</button>' : "") + "</div>" +
          '<div class="acts"><input type="text" class="dnote" placeholder="Resolution"><button class="b" data-resolve="' + h(r.id) + '">Resolve</button></div></div>';
      }).join("") + (p.rows.length ? "" : '<div class="empty">' + h(p.empty_text) + "</div>");
    });
  }
  function resolveIssue(id, text, btn) {
    if (btn) { btn.disabled = true; }
    return invoke("resolve_issue", { view: stateView(), id: id, resolution: text }).then(function (env) {
      if (!applyEnvelope(env, function (m) { if (btn) { btn.disabled = false; } showRefusal(null, m, id); })) { return; }
    }).catch(function () { if (btn) { btn.disabled = false; } });
  }

  // ---- Phase 2 of the commitment model (spec §4, D6–D8): the Schedule view ("Your week"), the
  // window editor and the moved line. Rows come from `your_week` (the engine's
  // `commitments::overview`). A kind or level goes through `set_fields`; the window and an
  // office-hours Add go through `commitments_confirm`. Nothing here computes a time: the flow
  // sequence is string assembly from the pickers, and the engine validates it.
  var DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
  var DAY_NAMES = { mon: "Mon", tue: "Tue", wed: "Wed", thu: "Thu", fri: "Fri", sat: "Sat", sun: "Sun" };
  var KIND_NAMES = [["class", "Class"], ["lab", "Lab"], ["work", "Work"], ["club", "Club"], ["meeting", "Meeting"], ["office-hours", "Office hours"]];
  var LEVELS = [["hard", "Must keep"], ["soft", "Usually"], ["optional", "Optional"]];
  var PREVIEW_MS = 400;
  var previewTimer = null;
  var windowDirty = false;   // Q10 review M5: unsaved picker edits survive a refresh

  // Q10-a: `schedule` is the page's view, not the read model's — the state it paints is today's.
  function stateView() { return current.view === "schedule" ? "today" : current.view; }

  // D7: the today view says what moved, when the read model carries it (§6.4).
  function renderMoved(state) {
    var m = state.moved, el = EL("moved");
    el.hidden = !m;
    el.textContent = m ? m.text : "";
  }

  function levelButtons(level) {
    return '<span class="lvl" role="group" aria-label="How much it binds">' + LEVELS.map(function (l) {
      return '<button class="b" type="button" data-level-set="' + l[0] + '" aria-pressed="' + (l[0] === level) + '">' + h(l[1]) + "</button>";
    }).join("") + "</span>";
  }

  // One row per weekday, Mon to Sun, each a start and an end picker; `byDay` is {mon: {start, end}}.
  function windowEditorHtml(byDay) {
    return DAYS.map(function (d) {
      var w = byDay[d] || {};
      return '<div class="wrow" data-day="' + d + '"><span class="wday">' + DAY_NAMES[d] + "</span>" +
        '<input type="time" data-part="start" aria-label="' + DAY_NAMES[d] + ' start" value="' + h(w.start || "") + '">' +
        '<input type="time" data-part="end" aria-label="' + DAY_NAMES[d] + ' end" value="' + h(w.end || "") + '">' +
        (d === "mon" ? '<button class="lnk" type="button" data-same-as-monday>same as Monday for Tue&ndash;Fri</button>' : "") +
        '<span class="werr" data-werr="' + d + '"></span></div>';
    }).join("");
  }

  // The flow sequence from the pickers. Days with the same start and end share one entry; a day
  // with an empty picker is left out, so it keeps week_template.yaml's hours. Null when every
  // row is blank: no window is sent (Plan ruling Q4-b).
  function windowSequence(host) {
    var groups = [], byKey = {};
    DAYS.forEach(function (d) {
      var row = host.querySelector('.wrow[data-day="' + d + '"]'); if (!row) { return; }
      var s = row.querySelector('[data-part="start"]').value, e = row.querySelector('[data-part="end"]').value;
      if (!s || !e) { return; }
      var k = s + "-" + e;
      if (!byKey[k]) { byKey[k] = { days: [], start: s, end: e }; groups.push(byKey[k]); }
      byKey[k].days.push(d);
    });
    if (!groups.length) { return null; }
    return "[" + groups.map(function (g) { return "{days: [" + g.days.join(", ") + '], start: "' + g.start + '", end: "' + g.end + '"}'; }).join(", ") + "]";
  }

  // The engine's own message, under the row it names ("planning day fri: …"), else under Monday.
  function showWindowError(host, message) {
    host.querySelectorAll("[data-werr]").forEach(function (e) { e.textContent = ""; });
    if (!message) { return; }
    var m = /planning day (\w+)/.exec(message);
    var slot = (m && host.querySelector('[data-werr="' + m[1] + '"]')) || host.querySelector('[data-werr="mon"]');
    if (slot) { slot.textContent = message; }
  }

  function bindWindowEditor(host, onEdit) {
    host.addEventListener("click", function (e) {
      if (!e.target.closest("[data-same-as-monday]")) { return; }
      var mon = host.querySelector('.wrow[data-day="mon"]');
      ["tue", "wed", "thu", "fri"].forEach(function (d) {
        var row = host.querySelector('.wrow[data-day="' + d + '"]');
        row.querySelector('[data-part="start"]').value = mon.querySelector('[data-part="start"]').value;
        row.querySelector('[data-part="end"]').value = mon.querySelector('[data-part="end"]').value;
      });
      onEdit();
    });
    host.addEventListener("input", onEdit);
  }

  function schedulePreview() {
    if (previewTimer) { clearTimeout(previewTimer); }
    previewTimer = setTimeout(runPreview, PREVIEW_MS);
  }

  // §4: beside the editor, the preview's moved line (or "No change to today's plan") and the
  // first five items of the previewed day, in order.
  function runPreview() {
    var host = EL("sched-window"), seq = windowSequence(host);
    if (!seq) { EL("sched-moved").textContent = "No change to today's plan"; EL("sched-items").innerHTML = ""; return; }
    invoke("preview_window", { window: seq }).then(function (r) {
      if (!r.ok) { showWindowError(host, r.error); return; }
      showWindowError(host, null);
      var s = r.state, items = [];
      EL("sched-moved").textContent = s.moved ? s.moved.text : "No change to today's plan";
      s.the_day.blocks.forEach(function (b) { b.takes.forEach(function (t) { items.push(t.title); }); });
      EL("sched-items").innerHTML = items.slice(0, 5).map(function (t) { return "<li>" + h(t) + "</li>"; }).join("");
    }).catch(function () {});
  }

  // The view's renderer (VIEW_RENDERERS.schedule). Its data is `your_week`, not the state `paint`
  // passes it. The editor is rebuilt only while nobody is typing in it.
  function renderScheduleView() {
    return invoke("your_week", {}).then(function (r) {
      if (!r || !r.ok || !r.week) { EL("sched-n").textContent = (r && r.error) || ""; return; }
      var w = r.week;
      EL("sched-n").textContent = w.commitments.length + (w.commitments.length === 1 ? " commitment" : " commitments");
      EL("sched-list").innerHTML = w.commitments.map(function (c) {
        var known = KIND_NAMES.some(function (k) { return k[0] === c.kind; });
        var kinds = (known ? "" : '<option value="' + h(c.kind) + '" selected>' + h(c.kind) + "</option>") + KIND_NAMES.map(function (k) {
          return '<option value="' + k[0] + '"' + (k[0] === c.kind ? " selected" : "") + ">" + h(k[1]) + "</option>";
        }).join("");
        return '<div class="row sched" data-id="' + h(c.id) + '" data-kind="commitment"><div class="ttl"><span class="a">' + h(c.title) +
          '</span><span class="meta">' + h(c.when || "") + (c.where ? " · " + h(c.where) : "") + "</span></div>" +
          '<div class="acts"><select data-kind-set aria-label="Kind">' + kinds + "</select>" + levelButtons(c.level) + "</div></div>";
      }).join("") + w.uncovered_courses.map(function (u) {
        return '<div class="row sched uncovered"><div class="ttl"><span class="a">' + h(u.title) + '</span><span class="meta">Knowlu will ask when it meets</span></div></div>';
      }).join("") + (w.commitments.length || w.uncovered_courses.length ? "" : '<div class="empty">Nothing confirmed yet.</div>');
      EL("sched-oh").innerHTML = w.office_hours.map(function (p) {
        return '<div class="row sched oh"><div class="ttl"><span class="a">' + h(p.title) + '</span><span class="meta">' + h(p.when || "") +
          '</span></div><div class="acts"><button class="b" type="button" data-oh-add="' + h(p.source_uid) + '">Add</button></div></div>';
      }).join("") || '<div class="empty">No office hours found on your calendar.</div>';
      var host = EL("sched-window"), byDay = {};
      w.window.forEach(function (d) { byDay[d.day] = d; });
      if (!windowDirty && !host.contains(document.activeElement)) { host.innerHTML = windowEditorHtml(byDay); }
      schedulePreview();
      watchSeen();   // Q10 review M4: the rows arrive after paint's own watchSeen()
    }).catch(function () {});
  }

  // No explicit renderScheduleView() here: applyEnvelope paints, and paint already runs the
  // Schedule view's renderer (review finding 9 — one your_week round trip per click, not two).
  function saveCommitment(id, fields) {
    return invoke("set_fields", { view: stateView(), id: id, fields: fields }).then(function (env) {
      applyEnvelope(env, function (msg) { showRefusal(null, msg, id); });
    }).catch(function () {});
  }

  // Every `commitments_confirm` goes through here: the Schedule view's Save and Add, and Q11's
  // Finish. A returned state paints like any write's.
  function confirmWeek(payload) {
    return invoke("commitments_confirm", { view: stateView(), confirm: payload }).then(function (env) {
      if (env.state) { current.pendingOrder = null; paint(env.state, true); }
      return env;
    });
  }

  function bindScheduleView() {
    var list = EL("sched-list");
    list.addEventListener("click", function (e) {
      // Q10 review I1: no click in a Schedule row reaches the document handler. It would open
      // the note drawer, whose Delete… and free field edits are the remove control D8 withholds.
      if (e.target.closest(".row.sched")) { e.stopPropagation(); }
      var b = e.target.closest("[data-level-set]"); if (!b) { return; }
      saveCommitment(b.closest(".row.sched").getAttribute("data-id"), { level: b.getAttribute("data-level-set") });
    });
    list.addEventListener("change", function (e) {
      var sel = e.target.closest("[data-kind-set]"); if (!sel) { return; }
      saveCommitment(sel.closest(".row.sched").getAttribute("data-id"), { kind: sel.value });
    });
    EL("sched-oh").addEventListener("click", function (e) {
      var b = e.target.closest("[data-oh-add]"); if (!b) { return; }
      b.disabled = true;
      // §4: office hours default to optional (§2.2).
      // Q10 review M1: a refusal says so; M2: a painted state already re-ran this view's renderer.
      confirmWeek({ mine: [{ source_uid: b.getAttribute("data-oh-add"), level: "optional" }] }).then(function (env) {
        if (!env.ok) { b.disabled = false; showRefusal(null, "refused: " + env.error); }
        if (!env.state) { return renderScheduleView(); }
      }).catch(function () { b.disabled = false; });
    });
    bindWindowEditor(EL("sched-window"), function () {
      windowDirty = true; EL("sched-say").textContent = "";   // Q10 review M5, M6
      schedulePreview();
    });
    EL("sched-save").addEventListener("click", function () {
      var host = EL("sched-window"), seq = windowSequence(host);
      if (!seq) { EL("sched-say").textContent = "Nothing to save"; return; }   // Q10 review M6
      confirmWeek({ window: seq }).then(function (env) {
        if (!env.ok) { showWindowError(host, env.error); return; }
        windowDirty = false;
        showWindowError(host, null);
        EL("sched-say").textContent = env.result && env.result.window === "unchanged" ? "No change" : "Saved";
        if (!env.state) { renderScheduleView(); }   // Q10 review M2: else paint already did
      }).catch(function () {});
    });
  }

  // ---- Phase 2 (spec §2, D1, D3): the confirm screen, over the first-run view. `your_week` says
  // `setup` while the vault is on its first day and has no planning-day note. Not now hides the
  // screen for this console session; Finish writes the planning-day note, so it never returns.
  var weekSetup = { dismissed: false, open: false, dirty: false };

  function checkWeekSetup() {
    if (weekSetup.dismissed || weekSetup.open) { return; }
    invoke("your_week", {}).then(function (r) {
      if (r && r.ok && r.week && r.week.setup && !weekSetup.dismissed) { openWeekSetup(r.week); }
    }).catch(function () {});
  }

  // One row: title, the §5.2 label, `where` in small type; Mine / Not mine; the level control,
  // shown by CSS only on a Mine row. Nothing typed.
  function setupRow(p) {
    var preset = { class: 1, lab: 1, work: 1 }[p.kind] ? "mine" : "";
    return '<div class="wsrow" data-key="' + h(p.source_uid) + '" data-answer="' + preset + '" data-level="' + h(p.level) + '">' +
      '<div class="ttl"><span class="a">' + h(p.title) + '</span><span class="meta">' + h(p.when || "") + "</span>" +
      (p.where ? "<small>" + h(p.where) + "</small>" : "") + "</div>" +
      '<div class="acts"><button class="b" type="button" data-answer-set="mine" aria-pressed="' + (preset === "mine") + '">Mine</button>' +
      '<button class="b" type="button" data-answer-set="not" aria-pressed="false">Not mine</button>' + levelButtons(p.level) + "</div></div>";
  }

  function paintSetupRows(ps, uncovered) {
    var classes = ps.filter(function (p) { return !p.window && (p.kind === "class" || p.kind === "lab"); });
    var office = ps.filter(function (p) { return !p.window && p.kind === "office-hours"; });
    var rest = ps.filter(function (p) { return !p.window && classes.indexOf(p) === -1 && office.indexOf(p) === -1; });
    EL("ws-class-rows").innerHTML = classes.map(setupRow).join("") + uncovered.map(function (u) {
      return '<div class="wsrow uncovered"><div class="ttl"><span class="a">' + h(u.title) + '</span><span class="meta">no class times found; Knowlu will ask this week</span></div></div>';
    }).join("");
    EL("ws-week-rows").innerHTML = rest.map(setupRow).join("");
    EL("ws-oh-rows").innerHTML = office.map(setupRow).join("");
    EL("ws-classes").hidden = !classes.length && !uncovered.length;
    EL("ws-week").hidden = !rest.length;
    EL("ws-oh").hidden = !office.length;
  }

  function openWeekSetup(week) {
    weekSetup.open = true;
    EL("week-setup").hidden = false;
    EL("ws-status").textContent = "Reading your calendar…";
    var byDay = {};
    DAYS.forEach(function (d) { byDay[d] = { start: "08:00", end: "22:00" }; });
    EL("ws-window").innerHTML = windowEditorHtml(byDay);
    paintSetupRows([], week.uncovered_courses || []);
    // Final review I1: Finish waits for the calendar read to settle, succeeded or failed. A Finish
    // before it would write the planning-day note from the placeholder hours, and the routine's
    // window proposal would never be offered again.
    weekSetup.dirty = false;
    EL("ws-finish").disabled = true;
    var ready = function () { EL("ws-finish").disabled = false; };
    invoke("commitment_proposals", {}).then(function (r) {
      var ps = (r && r.proposals) || [];
      var win = ps.filter(function (p) { return p.window; })[0];
      // An edit the student made while waiting is theirs: the proposal does not overwrite it.
      if (win && !weekSetup.dirty) {
        byDay = {};
        win.meets.forEach(function (m) { m.days.forEach(function (d) { byDay[d] = { start: m.start, end: m.end }; }); });
        EL("ws-window").innerHTML = windowEditorHtml(byDay);
      }
      paintSetupRows(ps, (r && r.uncovered_courses) || week.uncovered_courses || []);
      EL("ws-status").textContent = ps.some(function (p) { return !p.window; }) ? "Knowlu found these on your calendar. Mark each one." : "Knowlu found nothing repeating on your calendar. Set your day below.";
    }).catch(function () { EL("ws-status").textContent = "Knowlu found nothing repeating on your calendar. Set your day below."; }).then(ready);
  }

  function closeWeekSetup() { weekSetup.open = false; EL("week-setup").hidden = true; }

  function finishWeekSetup(btn) {
    var mine = [], notMine = [];
    document.querySelectorAll("#week-setup .wsrow[data-key]").forEach(function (r) {
      var key = r.getAttribute("data-key"), a = r.getAttribute("data-answer");
      if (a === "mine") { mine.push({ source_uid: key, level: r.getAttribute("data-level") }); }
      else if (a === "not") { notMine.push(key); }
    });
    // Review finding 2: D3 says Finish always writes the planning-day note, so the screen never
    // returns. An all-blank Your day would write none, so Finish asks for one day first.
    var seq = windowSequence(EL("ws-window"));
    if (!seq) { showWindowError(EL("ws-window"), "Set the hours for at least one day"); return; }
    var payload = { mine: mine, not_mine: notMine, window: seq };
    btn.disabled = true;
    confirmWeek(payload).then(function (env) {
      btn.disabled = false;
      if (!env.ok) { showWindowError(EL("ws-window"), env.error); EL("ws-status").textContent = env.error; return; }
      closeWeekSetup();
      // Final review M4: a row --confirm skipped (no longer a current proposal) comes back as a
      // question; say so on the notice line until the next paint, rather than close in silence.
      var skipped = (env.result && env.result.warnings) || [];
      if (skipped.length) {
        EL("delta").textContent = skipped.length + (skipped.length === 1 ? " row" : " rows") + " will come back as questions: " + skipped.join("; ");
      }
    }).catch(function () { btn.disabled = false; });
  }

  EL("week-setup").addEventListener("click", function (e) {
    // Q10's care: no click on this screen reaches the document handler (its drawer and its .b
    // branch belong to the console underneath).
    e.stopPropagation();
    var row = e.target.closest(".wsrow[data-key]");
    var a = e.target.closest("[data-answer-set]");
    if (a && row) {
      // Q11-c: a second press on the pressed button returns the row to unanswered.
      var v = a.getAttribute("data-answer-set"), next = row.getAttribute("data-answer") === v ? "" : v;
      row.setAttribute("data-answer", next);
      row.querySelectorAll("[data-answer-set]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.getAttribute("data-answer-set") === next)); });
      return;
    }
    var l = e.target.closest("[data-level-set]");
    if (l && row) {
      row.setAttribute("data-level", l.getAttribute("data-level-set"));
      row.querySelectorAll("[data-level-set]").forEach(function (b) { b.setAttribute("aria-pressed", String(b === l)); });
      return;
    }
    if (e.target.closest("#ws-later")) { weekSetup.dismissed = true; closeWeekSetup(); return; }
    var fin = e.target.closest("#ws-finish");
    if (fin) { finishWeekSetup(fin); }
  });
  bindWindowEditor(EL("ws-window"), function () { weekSetup.dirty = true; showWindowError(EL("ws-window"), null); });

  // Dispatch table from view name to its main-body renderer. Every nav view is built now
  // (Task 3 finishes Issues) — the not-built placeholder is gone (R-P2-3).
  var VIEW_RENDERERS = { runs: renderRunsView, decisions: renderDecisionsView, "good-to-know": renderGoodToKnowView, issues: renderIssuesView, schedule: renderScheduleView };

  function renderRuns(state) {
    var r = state.runs_panel, html = "";
    EL("runs-n").textContent = r.warn_count ? r.warn_count + " warn" : "ok";
    r.expected.forEach(function (e) { if (e.status !== "seen") { html += '<div class="ln run"><span class="k ' + (e.status === "late" ? "wn" : "ms") + '">' + h(e.status) + '</span><span class="rt">' + h(e.due.slice(11, 16)) + "</span><span>" + h(e.runner) + " — expected " + h(e.due) + "</span></div>"; } });
    r.recent.slice(0, 6).forEach(function (x) { var k = x.result === "ok" ? "gd" : x.result === "WARN" ? "wn" : "ms"; html += '<div class="ln run"><span class="k ' + k + '">' + h(x.result.toLowerCase()) + '</span><span class="rt">' + h((x.ended || x.started || "").slice(11)) + "</span><span>" + h(x.runner) + " — " + h(x.summary) + "</span></div>"; });
    if (r.empty_text) { html += '<div class="empty">' + h(r.empty_text) + "</div>"; }
    EL("runs").innerHTML = html;
  }

  function paint(state, force) {
    // R28/R29/R30: a reorder never freezes the whole page — only the order-bearing regions
    // (must-do/recommended on Today, the list on a horizon view) hold for the "refresh order"
    // button; nav/topline/delta/the-day/deck/ahead/coming-up/good-to-know/closed/runs and the
    // verdict/meter (not order-bearing) always paint. current.state stays the *displayed*
    // order's state while held — the new state lives only in pendingOrder — so orderOf(current.state)
    // keeps comparing against what is actually on screen.
    // Q10 review M3: the Schedule view shows no order-bearing list, so it never takes the hold.
    var reordered = current.view !== "schedule" && current.state && !force && orderOf(current.state) !== orderOf(state);
    // Fix (final review B): pendingOrder must reflect THIS state before renderDelta runs below —
    // a later poll that returns the order to what is displayed (reordered false) has to clear a
    // stale pendingOrder from an earlier held poll, or the button stays and "refresh order"
    // paints an older state over the current one.
    current.pendingOrder = reordered ? state : null;
    if (reordered) { current.revision = state.revision; }
    renderNav(state); renderTopline(state); renderGauge(state); renderSyncLine(state); renderDelta(state); renderTheDay(state); renderDeck(state); renderAhead(state); renderComingUp(state); renderGoodToKnow(state); renderClosed(state); renderRuns(state);
    var listViews = { overdue: 1, week: 1, later: 1, all: 1 };
    EL("main-today").hidden = current.view !== "today"; EL("main-list").hidden = !listViews[current.view]; EL("main-runs").hidden = current.view !== "runs"; EL("main-decisions").hidden = current.view !== "decisions";
    EL("main-gtk").hidden = current.view !== "good-to-know";
    EL("main-issues").hidden = current.view !== "issues";
    EL("main-schedule").hidden = current.view !== "schedule";
    if (current.view === "today") { renderVerdict(state); renderMeter(state); renderMoved(state); }
    if (reordered) { watchSeen(); return; }   // hold only renderMustDo/renderRecommended or renderList below
    current.state = state; current.revision = state.revision;
    if (current.view === "today") { renderMustDo(state); renderRecommended(state); }
    else if (listViews[current.view]) { renderList(state); }
    else if (VIEW_RENDERERS[current.view]) { VIEW_RENDERERS[current.view](state); }
    watchSeen();
  }

  function orderOf(state) {
    var ids = [];
    state.must_do.groups.forEach(function (g) { g.rows.forEach(function (r) { ids.push(r.id); }); });
    if (state.list) { state.list.rows.forEach(function (r) { ids.push(r.id); }); }
    return ids.join(",");
  }

  function route(view) {
    // R28: a view change is not a reorder — clearing current.state too means the first paint
    // after a route never takes the hold path (orderOf(null) would never match orderOf(state)).
    // R-P2-3 (rider): the not-built placeholder is gone, so an unrecognised hash must not reach
    // a view with no renderer and no toggle to hide it — fall back to "today" instead (one line).
    current.view = (view && { today: 1, overdue: 1, week: 1, later: 1, all: 1, decisions: 1, "good-to-know": 1, issues: 1, runs: 1, schedule: 1 }[view]) ? view : "today"; current.pendingOrder = null; current.revision = null; current.state = null;
    ev("view_opened", null, "view", null);
    return poll();
  }

  // D7: the minute between Finish and the first `rank`. `first_run` rides on the envelope until
  // `state/today.md` exists; while it does, the page says what is happening, lists the slot's steps
  // as they land, and asks again every three seconds so the day appears as soon as it is there
  // rather than up to a minute later.
  //
  // R-C1c-8: the view REPLACES the day. `.app` carries `first-run` while the block is on the
  // envelope, and console.css hides the nav, the rail and everything in `main` but `#first-run`.
  // The state still paints underneath (poll's revision logic is untouched), so the hand-over is
  // instant; and a `display: none` row never intersects, so the 2 s dwell below sends no
  // `object_seen` for a row nobody saw. Nothing visible paints in this mode but the view itself.
  // Leaving the view forgets the displayed day (`hideFirstRun`), so the first ranked state paints
  // whole rather than behind R28's reorder hold (R-C1c-exec-8a).
  //
  // M6: nothing ends the three-second cadence but the day arriving, so a vault whose `rank` keeps
  // failing polls on forever, and a rejected call re-arms it too. That is honest rather than
  // silent: once a first slot has ended with any failed step, listed here or not (an `engine: …`
  // line counts), the view says so under the list and that Knowlu will try again. The Runs view
  // and the sync line that would say more are hidden in this mode. A cap would replace a true
  // "will try again" with a false "gave up".
  var FIRST_RUN_MS = 3000;
  var firstRunTimer = null;
  var firstRunHtml = null;
  // The steps the student is told about, by the step name's first word (before any space or
  // parenthesis: `judge (skipped: no entitlement)` is `judge`). The slot's other steps (the sync pull
  // and push, the backup, the usage upload, an `engine: …` line) are the Runs view's, not this one's.
  //
  // `sync` is C3′'s step, not a step on this branch yet: its sentence is here before the merge so a
  // merged first run shows its first seconds in progress (R-C1c-final2 M2).
  //
  // `session` (R-C1c-13's pre-flight) is left out on purpose, not an oversight: the first slot runs
  // minutes after sign-in minted a one-hour token, so on the first-run page the step is idle every
  // time. A later refresh failure surfaces through the entitlement row or the downstream steps' own
  // text instead — no row here.
  var FIRST_RUN_SAYS = {
    sync: "Syncing with your account",
    entitlement: "Checking your account",
    coursework: "Fetching your coursework",
    ingest: "Reading your school calendar",
    judge: "Working out what each task needs",
    rank: "Putting your day in order"
  };
  function firstRunSays(name) {
    var word = String(name || "").split(/[ (]/)[0];
    return Object.prototype.hasOwnProperty.call(FIRST_RUN_SAYS, word) ? FIRST_RUN_SAYS[word] : null;
  }
  // One row: its mark (a shape per state, so it never rests on colour alone) and its sentence.
  // A skip says the word; the other three states are named for a screen reader on the mark, and a
  // row may carry a short note of its own.
  function firstRunRow(state, says, note) {
    var label = { done: "done", now: "in progress", failed: "failed" }[state];
    var said = state === "skipped" ? "skipped" : note;
    return '<li class="fr-step" data-state="' + state + '"><span class="fr-mark"' +
      (label ? ' role="img" aria-label="' + label + '"' : ' aria-hidden="true"') + "></span>" +
      '<span class="fr-say">' + h(says) + "</span>" + (said ? '<span class="fr-note">' + h(said) + "</span>" : "") + "</li>";
  }
  function renderFirstRun(fr) {
    fr = fr || {};
    document.querySelector(".app").classList.add("first-run");
    EL("first-run").hidden = false;
    var rows = [];
    (fr.steps || []).forEach(function (s) {
      var says = firstRunSays(s[0]); if (!says) { return; }
      // R-C1c-final2 M1: an account check that could not reach the service lands at code 0 (a
      // network is not a failed slot, for the tray or the retry ladder), but a check mark would say
      // it worked. It shows as failed, with a note; the "didn't finish" line below still reads codes.
      var unchecked = String(s[0]).indexOf("entitlement (refresh failed") === 0;
      var state = String(s[0]).indexOf("(skipped:") !== -1 ? "skipped" : (s[1] === 0 && !unchecked ? "done" : "failed");
      rows.push(firstRunRow(state, says, unchecked ? "couldn't check — will retry" : null));
    });
    // The live slot's step in progress (`Scheduler.live.current`), last, where the next row lands.
    var now = fr.running ? firstRunSays(fr.current) : null;
    if (now) { rows.push(firstRunRow("now", now)); }
    // Repainted only when it changed: a fresh row every three seconds would restart the in-progress
    // mark's turn and re-announce the list to a screen reader.
    var html = rows.join("");
    if (html !== firstRunHtml) { EL("first-run-steps").innerHTML = html; firstRunHtml = html; }
    // A first slot that ended without a day (M6, above): one more line, and the cadence carries on.
    // Any failed step counts, listed or not: the engine missing is `engine: …` at -1, which has no row.
    var failed = (fr.steps || []).some(function (s) { return s[1] !== 0; });
    EL("first-run-end").hidden = !(!fr.running && failed);
    armFirstRun();
  }

  // One timer, cleared before it is set: the 60 s interval and the window's focus handler both
  // call poll() too, and a chain per call would multiply every three seconds.
  function armFirstRun() {
    if (firstRunTimer) { clearTimeout(firstRunTimer); }
    firstRunTimer = setTimeout(poll, FIRST_RUN_MS);
  }

  function hideFirstRun() {
    if (firstRunTimer) { clearTimeout(firstRunTimer); firstRunTimer = null; }
    EL("first-run").hidden = true;
    var app = document.querySelector(".app");
    // R-C1c-exec-8a (I1): on the way OUT only, forget the displayed day, as `route()` does for a
    // view change. The day painted under the view is the vault as it was before the slot, and every
    // state since has a new order, so R28's hold would keep that pre-slot Must do beneath the ranked
    // headline behind "refresh order". Cleared here, the same poll paints the ranked day whole: poll
    // calls this before its revision check. Cleared on every poll without the block, the hold would
    // never work again.
    if (app.classList.contains("first-run")) {
      app.classList.remove("first-run");
      current.state = null; current.revision = null; current.pendingOrder = null;
    }
  }

  function poll() {
    return invoke("state", { view: stateView() }).then(function (env) {
      // R-C1c-plan-1: the block is on the envelope exactly while the vault has never been ranked,
      // which IS D7's "until the first read model exists" — `surface::build_state` has no failure
      // path, so there is no failed state to wait for. The paint below still runs, hidden while the
      // view stands (R-C1c-8).
      if (env.first_run) { renderFirstRun(env.first_run); } else { hideFirstRun(); }
      if (!env.ok) { EL("delta").textContent = "engine: " + env.error; return; }
      // §6's safety net answers `ok` with no state when the read model could not be built at all;
      // the line above is what the student reads while that is true.
      if (!env.state) { return; }
      if (env.state.revision === current.revision) { return; }
      paint(env.state, false);
    }).catch(function (e) {
      EL("delta").textContent = (current.state ? current.state.texts.offline : "The engine did not answer.") + " (" + e.message + ")";
      // R-C1c-exec-8a (M1): while the view stands the line above is hidden, and nothing else would
      // re-arm the three-second cadence after a rejected call.
      if (document.querySelector(".app").classList.contains("first-run")) { armFirstRun(); }
    });
  }

  // Task 13: the four fields the write path always refuses (app/src/commands.rs's comment on
  // EDITABLE) render read-only in the drawer — never a data-field, always class="ro".
  var DRAWER_RO = { id: 1, source_uid: 1, also_uids: 1, judgment: 1 };

  function openDrawer(id) {
    invoke("note", { id: id }).then(function (env) {
      var d = EL("drawer");
      if (!env.ok) { d.innerHTML = '<button class="close">&times;</button><h2>Not found</h2><p>' + h(env.error) + "</p>"; d.hidden = false; d.removeAttribute("data-id"); return; }
      var n = env.note, fm = n.frontmatter || {}, dl = "", title = fm.title || n.slug;
      Object.keys(fm).sort().forEach(function (k) {
        var val = typeof fm[k] === "object" ? JSON.stringify(fm[k]) : fm[k];
        var attrs = EDITABLE.indexOf(k) !== -1 ? ' data-field="' + h(k) + '"' : (DRAWER_RO[k] ? ' class="ro" readonly' : "");
        dl += "<dt>" + h(k) + "</dt><dd" + attrs + ">" + h(val) + "</dd>";
      });
      var hist = (n.history || []).map(function (r) { return "<div>" + h(r.ts.slice(0, 16).replace("T", " ")) + " · " + h(r.text) + "</div>"; }).join("") || "<div>no journal history</div>";
      clearStaleEditing(d);
      d.innerHTML = '<button class="close">&times;</button><h2>' + h(title) + '</h2><dl data-id="' + h(id) + '" data-kind="' + h(n.folder) + '">' + dl + "</dl>" + (n.body ? "<pre>" + h(n.body) + "</pre>" : "") +
        '<button class="b crit" data-del="' + h(id) + '" data-title="' + h(title) + '">Delete&hellip;</button>' +
        '<div class="hist"><b>history</b>' + hist + "</div>";
      // The note's own kind (its folder — tasks/approvals/issues/info/courses) rides on the
      // outer panel too, since it also carries data-id and is what the observer actually sees
      // fill the viewport.
      d.hidden = false; d.setAttribute("data-id", id); d.setAttribute("data-kind", n.folder || "");
      d.querySelector(".close").addEventListener("click", function () { d.hidden = true; });
    });
  }

  // ----- writes (Knowlu plan 1, Task 13). Every mutating call returns the fresh state; paint it
  // with force — a write's return is never held as a reorder (F19).
  // Task 15: fire-and-forget interaction logging — ids only (`app/src/uievents.rs` refuses free
  // text server-side; the page never sends a title or a body), and a failure here must never
  // surface to Quinn or block the write it rode in on.
  function ev(action, objectId, objectKind, ms) {
    invoke("ui_event", { action: action, view: current.view, objectId: objectId || null, objectKind: objectKind || null, ms: ms == null ? null : ms }).catch(function () {});
  }

  // Task 15: "seen" is a 2s dwell, not a mere paint — a row that scrolls past in a flick never
  // counts. One IntersectionObserver watches every [data-id] the page currently has (rows, deck
  // cards, the drawer's <dl>); `seenOnce` keyed by id means at most one object_seen per id per
  // page load, even across repaints that re-observe the same still-visible element.
  var seenOnce = {};
  var seenObserver = ("IntersectionObserver" in window) ? new IntersectionObserver(function (entries) {
    entries.forEach(function (en) {
      var id = en.target.getAttribute("data-id"); if (!id) { return; }
      if (en.isIntersecting) {
        en.target._seenAt = Date.now();
        if (!seenOnce[id]) {
          setTimeout(function () {
            if (en.target._seenAt && Date.now() - en.target._seenAt >= 2000 && !seenOnce[id]) {
              seenOnce[id] = true;
              // R-T15b review fix: object_kind comes from the template that put data-id there
              // (row/card/drawer all set it now) — an element with none sends null, never a
              // guessed "task" that would mislabel an approval or an issue.
              ev("object_seen", id, en.target.getAttribute("data-kind"), 2000);
            }
          }, 2000);
        }
      } else { en.target._seenAt = null; }
    });
  }, { threshold: 0.6 }) : null;
  // R-T15c review fix: watchSeen() runs on every paint (the page runs all day, polling every
  // 60s) — a repaint replaces [data-id] elements wholesale (renderMustDo/Recommended/List/Deck
  // all rebuild via innerHTML), so unconditionally re-observing would register a fresh
  // IntersectionObserver target every single poll and never release the previous, now-detached
  // one: an unbounded leak, and dropping the whole observer wholesale instead would reset the
  // in-progress 2s dwell of any card that DID survive the repaint (the reorder-hold path keeps
  // the same nodes).
  // `seenObservedEls` tracks exactly what the observer currently holds; each call first drops
  // (unobserves) whatever is no longer in the document, then observes only the elements not
  // already tracked — an element that survived a repaint is left alone, dwell timer intact.
  var seenObservedEls = new Set();
  function watchSeen() {
    if (!seenObserver) { return; }
    seenObservedEls.forEach(function (el) { if (!el.isConnected) { seenObserver.unobserve(el); seenObservedEls.delete(el); } });
    document.querySelectorAll("[data-id]").forEach(function (el) {
      if (!seenObservedEls.has(el)) { el.setAttribute("data-watched", "1"); seenObserver.observe(el); seenObservedEls.add(el); }
    });
  }

  // Task 15/R37: `mark_seen` is invoked from exactly this one place — window `blur` and
  // `visibilitychange` → hidden are the end of a look, never the boot line (which would stamp
  // away the very delta the first paint is showing).
  function endOfLook() { invoke("mark_seen", {}).catch(function () {}); }

  function applyEnvelope(env, onRefusal) {
    if (env.state) { current.pendingOrder = null; paint(env.state, true); }
    if (!env.ok) { if (onRefusal) { onRefusal(env.error || "refused"); } else { showRefusal(null, "refused: " + env.error); } }
    return env.ok;
  }

  // Review fix (Important, attribution): the note's title if it's showing anywhere right now —
  // a row on the current view, or the open drawer — so a refusal toast can say what it's about.
  function noteTitleFor(id) {
    var s = current.state, found = null;
    if (s) {
      (s.must_do && s.must_do.groups || []).forEach(function (g) { (g.rows || []).forEach(function (r) { if (r.id === id) { found = r.title; } }); });
      if (!found) { (s.recommended && s.recommended.rows || []).forEach(function (x) { if (x.row && x.row.id === id) { found = x.row.title; } }); }
      if (!found && s.list) { (s.list.rows || []).forEach(function (r) { if (r.id === id) { found = r.title; } }); }
    }
    if (found) { return found; }
    var d = EL("drawer");
    if (d && d.getAttribute("data-id") === id) { var h2 = d.querySelector("h2"); if (h2) { return h2.textContent; } }
    return null;
  }

  // A toast on document.body, not on the element the caller passed in: paint() above rebuilds
  // the region's whole innerHTML on every write, so any row/chip reference taken before the call
  // is already detached from the document by the time this runs — anything appended to it would
  // never be seen. Review fix (Important, attribution): prefix the note's title and the field
  // name when known, and — since applyEnvelope has already repainted by the time this runs —
  // find the row by id in the fresh DOM and flag it for 6s too, so the refusal is attributable
  // on the row itself, not only in the toast.
  function showRefusal(el, message, id, field) {
    var title = id ? noteTitleFor(id) : null;
    var prefix = (title ? title + " — " : "") + (field ? field + ": " : "");
    var n = document.createElement("div"); n.className = "refusal"; n.textContent = prefix + message;
    document.body.appendChild(n);
    setTimeout(function () { n.remove(); }, 6000);
    if (id) {
      var row = document.querySelector('.row[data-id="' + id.replace(/"/g, "") + '"]');
      if (row) { row.classList.add("refused"); setTimeout(function () { row.classList.remove("refused"); }, 6000); }
    }
  }

  function commitEdit(id, fields, el, field) {
    return invoke("set_fields", { view: stateView(), id: id, fields: fields }).then(function (env) { if (env.ok) { ev("edit_committed", id, field, null); } return applyEnvelope(env, function (msg) { showRefusal(el, msg, id, field); }); });
  }

  function cancelEdit() { if (editing) { var e = editing; e.el.innerHTML = e.oldHTML; editing = null; ev("edit_cancelled", e.id, e.field, null); } }

  // Review fix (Important, validation): the ranges the write path can actually accept before it
  // ever gets a request — importance 1-5, progress 0-100 (both step-quantised), effort/slice
  // hours ≥ 0 in half-hour steps.
  var NUMERIC_RANGE = { importance: [1, 5, 1], progress: [0, 100, 5], effort_hours: [0, null, 0.5], slice_hours: [0, null, 0.5], rank_override: [null, null, null] };

  // A chip's textContent is often a formatted display string (rowHtml's editChip carries the raw
  // literal in data-value for exactly this reason); the drawer's dd never formats, so its plain
  // textContent already is the literal.
  function rawValue(el) { return el.hasAttribute("data-value") ? el.getAttribute("data-value") : el.textContent; }

  // F9: reaching 100 closes the task in the SAME write — one journal moment, one CLOSED line. The
  // drag handler and click-to-edit (a row's chip, and the drawer's `progress` dd) all commit
  // through this, so a progress of 100 means the same thing however it was entered (final fix
  // wave, C6 — the drawer used to write `progress: 100` and leave the task active).
  function progressFields(pct) { var f = { progress: pct }; if (pct === 100) { f.status = "done"; } return f; }

  function editField(el, id, field) {
    if (editing) { cancelEdit(); }
    var oldHTML = el.innerHTML, val = rawValue(el);
    var input = document.createElement("input");
    var range = NUMERIC_RANGE[field];
    input.type = field === "due" ? "datetime-local" : (range ? "number" : "text");
    if (range) {
      if (range[0] != null) { input.min = range[0]; }
      if (range[1] != null) { input.max = range[1]; }
      if (range[2] != null) { input.step = range[2]; }
    }
    input.value = val;
    editing = { el: el, oldHTML: oldHTML, id: id, field: field };
    ev("edit_started", id, field, null);
    el.textContent = ""; el.appendChild(input); input.focus(); input.select();
    function done(commit) {
      if (!editing) { return; }
      var v = input.value.trim(); editing = null;
      if (!commit) { el.innerHTML = oldHTML; ev("edit_cancelled", id, field, null); return; }
      if (input.type === "number") {
        // Review fix (Important, validation): an emptied number field is a CANCEL, never a
        // write — `importance: null` (etc.) would pass the write path's own type check and can
        // leave the note unreadable. Out-of-range/non-numeric input snaps back and refuses
        // client-side, the same way a server refusal would show.
        if (v === "") { el.innerHTML = oldHTML; ev("edit_cancelled", id, field, null); return; }
        var num = Number(v);
        var tooLow = range && range[0] != null && num < range[0];
        var tooHigh = range && range[1] != null && num > range[1];
        if (isNaN(num) || tooLow || tooHigh) {
          el.innerHTML = oldHTML;
          // A one-sided range used to read "enter a number from 0 to null" — effort_hours and
          // slice_hours have a floor and no ceiling. Say the bound that exists, and only that one.
          var lo = range ? range[0] : null, hi = range ? range[1] : null;
          var bound = lo != null && hi != null ? " from " + lo + " to " + hi : lo != null ? " at least " + lo : hi != null ? " at most " + hi : "";
          showRefusal(el, "enter a number" + bound, id, field);
          ev("edit_cancelled", id, field, null);
          return;
        }
        if (num === Number(val)) { el.innerHTML = oldHTML; ev("edit_cancelled", id, field, null); return; }
        // Same commit path as the drag handler: typing 100 into a row's chip or the drawer's
        // `progress` dd closes the task exactly as dragging the track to the end does (C6).
        var numFields = field === "progress" ? progressFields(num) : {};
        if (field !== "progress") { numFields[field] = num; }
        el.textContent = String(num);
        commitEdit(id, numFields, el, field);
        return;
      }
      if (v === val) { el.innerHTML = oldHTML; ev("edit_cancelled", id, field, null); return; }
      var fields = {}; fields[field] = v === "" ? null : v;
      el.textContent = v;   // a brief plain-text flash; commitEdit's forced repaint restores real formatting
      commitEdit(id, fields, el, field);
    }
    input.addEventListener("keydown", function (e) { if (e.key === "Enter") { e.preventDefault(); done(true); } else if (e.key === "Escape") { e.preventDefault(); done(false); } });
    input.addEventListener("blur", function () { done(true); });
  }

  function bindProgressDrag(track, id) {
    var dragging = false, pct = 0, bar = track.querySelector("i"), before = bar ? bar.style.width : "";
    function at(e) { var r = track.getBoundingClientRect(); var raw = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)); return Math.round(raw * 20) * 5; }
    track.addEventListener("pointerdown", function (e) { dragging = true; before = bar ? bar.style.width : ""; track.setPointerCapture(e.pointerId); pct = at(e); if (bar) { bar.style.width = pct + "%"; } });
    track.addEventListener("pointermove", function (e) { if (dragging) { pct = at(e); if (bar) { bar.style.width = pct + "%"; } } });
    track.addEventListener("pointerup", function () {
      if (!dragging) { return; } dragging = false;
      commitEdit(id, progressFields(pct), track, "progress");
    });
    // A cancelled pointer — the browser claimed the gesture, a pen left range, the element was
    // replaced by a repaint mid-drag — ends the drag exactly as pointerup does, but commits
    // nothing and puts the bar back where it started, so the row never shows a progress the note
    // does not have (final fix wave, C6).
    track.addEventListener("pointercancel", function () { if (!dragging) { return; } dragging = false; if (bar) { bar.style.width = before; } });
  }

  function courseList() {
    var s = current.state, set = {};
    if (!s) { return []; }
    (s.must_do && s.must_do.groups || []).forEach(function (g) { (g.rows || []).forEach(function (r) { if (r.course) { set[r.course] = 1; } }); });
    (s.recommended && s.recommended.rows || []).forEach(function (x) { if (x.row && x.row.course) { set[x.row.course] = 1; } });
    return Object.keys(set).sort();
  }

  function renderNewTaskRow() {
    var host = EL("mustdo"); if (host.querySelector(".newrow")) { return; }
    var courses = courseList();
    var row = document.createElement("form"); row.className = "row newrow";
    row.innerHTML = '<span class="pip"></span><div class="ttl"><input name="title" type="text" placeholder="What needs doing? (Knowlu)" required autofocus>'
      + '<input name="course" list="courses" placeholder="course"><datalist id="courses">' + courses.map(function (c) { return '<option value="' + h(c) + '">'; }).join("") + "</datalist>"
      + '<input name="due" type="date"><input name="effort_hours" type="number" step="0.5" min="0" value="1.0"></div>'
      + '<button class="b pri" type="submit">Add</button><button class="b" type="button" data-cancel>Cancel</button>';
    host.prepend(row);
    row.addEventListener("submit", function (e) { e.preventDefault(); submitNewTask(row); });
    row.querySelector("[data-cancel]").addEventListener("click", function () { row.remove(); });
    row.querySelector("[name=title]").focus();
  }

  function submitNewTask(row) {
    var f = new FormData(row), fields = { title: f.get("title"), course: f.get("course") || null, due: f.get("due") || null, effort_hours: Number(f.get("effort_hours") || 1) };
    invoke("create_task", { view: stateView(), fields: fields }).then(function (env) { applyEnvelope(env, function (m) { showRefusal(row, m); }); });
  }

  function confirmDelete(id, title) {
    if (!window.confirm("Archive \"" + title + "\"? Nothing is deleted — it moves to archive/ and shows in CLOSED THIS WEEK.")) { return; }
    invoke("delete_note", { view: stateView(), id: id }).then(function (env) { if (applyEnvelope(env)) { EL("drawer").hidden = true; } });
  }

  // ----- the deck: approve/reject/snooze (Knowlu plan 1, Task 14). One click commits — the
  // overflow "…" button is the only one with a second step (it reveals the date input first).
  // Review fix (Important): a card stays hit-testable for the 260ms slide-out and however long
  // the IPC round-trip takes — a fast double-click, or Approve then Reject before the first
  // invoke resolves, must not fire two decide calls for the same id. `card.dataset.busy` marks
  // it in flight; bindDeck ignores clicks on a busy card, and the buttons/note are disabled too
  // (belt and suspenders — pointer-events:none on .gone covers the mouse, disabled covers a
  // click that still reaches a not-yet-hidden control). The success path never re-enables
  // anything itself — applyEnvelope's forced repaint replaces the whole card.
  function decideCard(id, verdict, snoozeUntil, host) {
    host = host || EL("deck");
    var card = host.querySelector('[data-id="' + id + '"]');
    if (card && card.dataset.busy) { return; }
    var noteEl = card ? card.querySelector(".nb input, .dnote") : null;
    var note = noteEl ? (noteEl.value || "") : "";
    if (card) {
      card.classList.add("gone");
      card.dataset.busy = "1";
      card.querySelectorAll("button[data-verdict]").forEach(function (b) { b.disabled = true; });
      var noteInput = card.querySelector(".nb input, .dnote"); if (noteInput) { noteInput.disabled = true; }
    }
    return invoke("decide", { view: stateView(), id: id, verdict: verdict, note: note, snoozeUntil: snoozeUntil || null }).then(function (env) {
      ev(verdict === "snoozed" ? "decision_deferred" : "decision_made", id, "approval", null);
      if (!applyEnvelope(env, function (m) {
        if (card) {
          card.classList.remove("gone");
          delete card.dataset.busy;
          card.querySelectorAll("button[data-verdict]").forEach(function (b) { b.disabled = false; });
          var ni = card.querySelector(".nb input, .dnote"); if (ni) { ni.disabled = false; }
          showRefusal(card, m);
        }
      })) { return; }
      if (env.decision && env.decision.warnings && env.decision.warnings.length) { EL("delta").textContent = "decision: " + env.decision.warnings.join("; "); }
    });
  }
  function bindDeck() {
    var deck = EL("deck");
    deck.addEventListener("click", function (e) {
      if (e.target.closest("[data-answer-in]")) { location.hash = "#decisions"; return; }
      var b = e.target.closest("button[data-verdict]"); if (!b) { return; }
      var card = b.closest(".card");
      if (card.dataset.busy) { return; }   // a decide is already in flight for this card
      var id = card.getAttribute("data-id");
      if (b.getAttribute("data-verdict") === "snoozed") {
        var d = card.querySelector("input[type=date]"); if (d.hidden) { d.hidden = false; d.focus(); return; }
        decideCard(id, "snoozed", d.value);
      } else { decideCard(id, b.getAttribute("data-verdict"), null); }
    });
  }

  function bindDecisionsView() {
    EL("dec-list").addEventListener("click", function (e) {
      // Final fix wave A3: a row's .acts is its own control strip -- the note input, the gaps
      // between the buttons, the buttons themselves. None of it may reach the document handler,
      // which would open the drawer (the input, a gap) or take its .b branch and emit
      // why_expanded with the literal "task" kind on an approval.
      if (e.target.closest(".row.dec .acts")) { e.stopPropagation(); }
      // Phase 2: the ask form's own controls live in .ttl and must not open the drawer either.
      if (e.target.closest(".row.dec.ask .ttl")) { e.stopPropagation(); }
      var day = e.target.closest("[data-ask-day]");
      if (day) { day.setAttribute("aria-pressed", String(day.getAttribute("aria-pressed") !== "true")); return; }
      if (e.target.closest("[data-ask-more]")) { e.target.closest(".row.dec").querySelector(".ask-times").insertAdjacentHTML("beforeend", askTimeRow()); return; }
      var save = e.target.closest("[data-ask-save]");
      if (save) { answerAsk(save.closest(".row.dec")); return; }
      var b = e.target.closest("button[data-verdict]"); if (!b) { return; }
      var row = b.closest(".row.dec"); if (!row) { return; }
      decideCard(row.getAttribute("data-id"), b.getAttribute("data-verdict"), b.getAttribute("data-snooze") || null, EL("dec-list"));
    });
  }

  function bindGoodToKnowView() {
    EL("gtk-list").addEventListener("click", function (e) {
      var b = e.target.closest("button[data-close-info]"); if (!b) { return; }
      // Final fix wave A2: the document handler has a [data-close-info] branch of its own (for
      // the rail's copy of this button). Without this the same click closed the item twice.
      e.stopPropagation();
      closeInfoItem(b.getAttribute("data-close-info"), b);
    });
  }

  function bindIssuesView() {
    EL("iss-list").addEventListener("click", function (e) {
      // Final fix wave A3, as in bindDecisionsView: the whole .acts strip stays in this view --
      // the resolution input included, which otherwise opened the drawer on its first click.
      if (e.target.closest(".row.iss .acts")) { e.stopPropagation(); }
      var resolveBtn = e.target.closest("button[data-resolve]");
      if (resolveBtn) {
        e.stopPropagation();   // never fall through to the document-level .row[data-id] drawer open
        var row = resolveBtn.closest(".row.iss");
        resolveIssue(resolveBtn.getAttribute("data-resolve"), (row && row.querySelector(".dnote").value) || "", resolveBtn);
        return;
      }
      var openBtn = e.target.closest("button[data-open-target]");
      if (openBtn) { e.stopPropagation(); openDrawer(openBtn.getAttribute("data-open-target")); }
    });
  }

  // ----- the ⚑ flag popover — eight fixed categories as toggle chips plus free text, on any
  // judged row or (always) deck card. GOOD TO KNOW items never carry one (info::close_info instead).
  function openFlag(anchor, targetId) {
    var host = anchor.closest(".row, .card, .ln");
    var already = host.querySelector(".flagpop"); if (already) { already.remove(); }
    var cats = ["wrong-effort", "wrong-course", "duplicate", "should-not-exist", "wrong-tier", "wrong-date", "wrong-verdict", "other"];
    var pop = document.createElement("div"); pop.className = "flagpop";
    pop.innerHTML = "<div class=\"chips\">" + cats.map(function (c) { return '<button type="button" class="chip" data-cat="' + c + '">' + c + "</button>"; }).join("") + "</div>"
      + '<textarea placeholder="what is wrong, in your words"></textarea><div class="acts"><button class="b pri" data-send disabled>Flag</button><button class="b" data-cancel>Cancel</button></div>';
    host.appendChild(pop);
    // A click anywhere inside the popover must never bubble to the row/card it is nested in —
    // otherwise the document handler's .row[data-id] branch would open the drawer underneath it.
    pop.addEventListener("click", function (e) {
      e.stopPropagation();
      var chip = e.target.closest(".chip"); if (chip) { chip.classList.toggle("on"); pop.querySelector("[data-send]").disabled = !pop.querySelector(".chip.on"); return; }
      if (e.target.closest("[data-cancel]")) { pop.remove(); }
      if (e.target.closest("[data-send]")) { submitFlag(pop, targetId); }
    });
  }
  function submitFlag(pop, targetId) {
    var cats = Array.prototype.map.call(pop.querySelectorAll(".chip.on"), function (c) { return c.getAttribute("data-cat"); });
    // The popover lives inside the thing it is about — a task row (data-kind="task"), a deck card
    // (data-kind="approval"), the drawer's dl (the note's own folder). Read the kind back off that
    // host instead of labelling every flag "task": telling an approval from a task is the one
    // thing `object_kind` exists for, and the deck's ⚑ is where flags mostly come from. Read it
    // BEFORE the invoke — applyEnvelope repaints, so `pop` may be detached by the time it resolves
    // (final fix wave, C3).
    var host = pop.closest("[data-kind]");
    var kind = host ? host.getAttribute("data-kind") : null;
    invoke("open_issue", { view: stateView(), target: targetId, categories: cats, text: pop.querySelector("textarea").value }).then(function (env) {
      if (applyEnvelope(env, function (m) { showRefusal(pop, m); })) { ev("issue_opened", targetId, kind, null); pop.remove(); }
    });
  }
  // Plan 2 Task 2: an optional busy-guard button, the same pattern Task 3 will use for
  // resolveIssue(id, text, btn) — disabled until the envelope returns, re-enabled (and shown a
  // toast on the row) on a refusal, and re-enabled if the IPC call itself rejects. The rail's own
  // call (no btn) keeps working unchanged.
  function closeInfoItem(id, btn) {
    if (btn) { btn.disabled = true; }
    return invoke("close_info", { view: stateView(), id: id }).then(function (env) {
      // Final fix wave A7: showRefusal's first argument is unread; the id is what carries the
      // title prefix and re-finds the row (applyEnvelope has already repainted, so the button's
      // old .row is detached by now).
      applyEnvelope(env, function (m) { if (btn) { btn.disabled = false; } showRefusal(null, m, id); });
    }).catch(function () { if (btn) { btn.disabled = false; } });
  }

  document.addEventListener("click", function (e) {
    var a = e.target.closest("#navlinks a"); if (a) { e.preventDefault(); location.hash = a.getAttribute("href"); return; }
    var nt = e.target.closest("#newtask"); if (nt) { renderNewTaskRow(); return; }
    var del = e.target.closest("[data-del]"); if (del) { confirmDelete(del.getAttribute("data-del"), del.getAttribute("data-title")); return; }
    // [data-flag]/[data-close-info] route before the .row[data-id] drawer branch below — a flag
    // or close click must never also open the drawer underneath it.
    var flagBtn = e.target.closest("[data-flag]"); if (flagBtn) { openFlag(flagBtn, flagBtn.getAttribute("data-flag")); return; }
    var closeBtn = e.target.closest("[data-close-info]"); if (closeBtn) { closeInfoItem(closeBtn.getAttribute("data-close-info")); return; }
    // Task 15: manual sync/backup from the topline — both return the fresh state like every
    // other mutating command (spec's envelope shape), so applyEnvelope repaints it the same way.
    var syncBtn = e.target.closest("[data-sync]"); if (syncBtn) { invoke("sync", { view: stateView() }).then(function (env) { applyEnvelope(env, syncRefusal); ev("sync_run", null, null, null); }).catch(function () {}); return; }
    var backupBtn = e.target.closest("[data-backup]"); if (backupBtn) { invoke("backup_now", { view: stateView() }).then(applyEnvelope).catch(function () {}); return; }
    var gear = e.target.closest("[data-settings]"); if (gear) { openSettings(); return; }
    // Plan 4a Task 8: *Restart to update*. A refusal (a slot started between the offer and the
    // click, or the install failed) comes back in the envelope and is shown, never retried.
    var ins = e.target.closest("[data-install]");
    if (ins) { ins.disabled = true; invoke("install_update", {}).then(function (r) { if (!r.ok) { ins.disabled = false; showRefusal(null, r.error); } }).catch(function () { ins.disabled = false; }); return; }
    // A click on [data-field] edits in place and never opens the drawer or toggles "why" — this
    // has to run before the .row/.b handling below, since the editable chips live inside .b.
    var fld = e.target.closest("[data-field]");
    if (fld) {
      if (fld.classList.contains("ro")) { return; }
      var rowEl = fld.closest(".row[data-id]");
      if (rowEl) { editField(fld, rowEl.getAttribute("data-id"), fld.getAttribute("data-field")); return; }
      var dl = fld.closest("dl[data-id]");
      if (dl) { editField(fld, dl.getAttribute("data-id"), fld.getAttribute("data-field")); return; }
    }
    // Review fix (CRITICAL): bindProgressDrag never stops propagation, so the native click that
    // follows every pointerup bubbles here — without this guard it fails the [data-field] check
    // above (.track carries none, deliberately, see rowHtml), falls through the .b check below,
    // and opens the drawer on every single drag. A track is a drag target only, never a click target.
    if (e.target.closest(".track")) { return; }
    var row = e.target.closest(".row[data-id]");
    if (row) {
      if (e.target.closest(".b")) {
        var w = row.querySelector(".why");
        if (w) { var wasHidden = w.hidden; w.hidden = !w.hidden; if (wasHidden) { ev("why_expanded", row.getAttribute("data-id"), "task", null); } }
        return;
      }
      openDrawer(row.getAttribute("data-id"));
    }
  });
  bindDeck();   // #deck's node persists across renderDeck's innerHTML rewrites — bound once
  bindDecisionsView();
  bindScheduleView();
  bindGoodToKnowView();
  bindIssuesView();

  // ---- Plan 4a Task 7: the settings panel (spec §4). An overlay, not a view: a view name would
  // reach `surface::View::parse` on every poll and be refused. Two ways in, one function —
  // the topline gear and the tray's Settings item, which calls KNOWLU_OPEN_SETTINGS.
  function renderSettings(s) {
    EL("set-name-in").value = (current.profileName || "");
    EL("set-vault-path").textContent = current.vaultPath || "";
    EL("set-bdir").textContent = s.backup_dir || "not set";
    EL("set-autostart-in").checked = !!s.autostart;
    // Plan 4a Task 8 fills current.updateText from the last check — the boot runs one, and *Check
    // now* runs another. Until the first answer lands the row says the version and nothing it
    // cannot know.
    EL("set-update-state").textContent = current.updateText || ("version " + (current.version || "") + " — not checked yet");
    // "never silently": an unreadable profile registry means the name above is the vault folder's,
    // not this profile's, and settings_context says so rather than letting the fallback pass for
    // the real thing. The panel's one message line carries it until the next action overwrites it.
    EL("set-diag-note").textContent = current.registryError || "";
  }
  // Knowlu plan 3a Task 10: the Local judgment row. Four states and every one of them has words —
  // "not installed" is a normal state (spec §5.3), not an error, so it is never rendered red.
  //
  // **Task 10 review, m2**: the Download button's own label changes once both halves are present
  // — "Replace model…", never "Download" — so replacing an installed model (a legitimate thing to
  // want) does not look identical to a first-time install with the same wording and no warning
  // that it re-fetches gigabytes. Reset to "Download" on every other branch so it never gets stuck
  // on the wrong word if the model is later removed.
  function renderInference(s) {
    var el = EL("set-judge-state");
    // The Remove button is set on EVERY branch, including this one: leaving it in whatever
    // visibility the last successful render gave it would offer "Remove model" over a status the
    // page could not read.
    if (!s || !s.ok) { el.textContent = (s && s.error) || "unavailable"; EL("set-judge-remove").hidden = true; EL("set-judge-download").textContent = "Download"; return; }
    if (!s.runtime) { el.textContent = "runtime not installed — assignments arrive unestimated"; }
    else if (!s.model) { el.textContent = "model not installed — assignments arrive unestimated"; }
    else { el.textContent = "ready — " + (Math.round((s.model_bytes / 1073741824) * 10) / 10) + " GB model"; }
    EL("set-judge-remove").hidden = !s.model;
    EL("set-judge-download").textContent = (s.runtime && s.model) ? "Replace model…" : "Download";
  }
  function loadInference() {
    // The status object is handed back to the caller, not just rendered from — **Task 10 review,
    // m2**: reading which half is missing back out of the row's own rendered text was the actual
    // defect (a UI string is not state), and it broke the moment the text became "ready — X GB".
    return invoke("inference_status", {}).then(function (s) { renderInference(s); return s; }).catch(function () { return null; });
  }
  function installInference(kind, viaFile) {
    EL("set-judge-state").textContent = "installing " + kind + "… this can take a while";
    var step = viaFile
      ? invoke("pick_file", { title: kind === "model" ? "Choose the model file" : "Choose the runtime archive", extension: kind === "model" ? "gguf" : "zip" })
          .then(function (p) { if (!p || !p.path) { return null; } return invoke("install_inference_file", { kind: kind, path: p.path }); })
      : invoke("install_inference_download", { kind: kind });
    return step.then(function (r) {
      if (r && !r.ok) { EL("set-judge-state").textContent = r.error; return; }
      return loadInference();
    }).catch(function () { EL("set-judge-state").textContent = "that did not finish"; });
  }
  function renderAccountRow(a) {
    EL("set-account-state").textContent = (!a || !a.ok) ? "" : (a.needs_account
      ? "not attached to an account yet"
      : [a.email || "", a.status || "", a.plan || ""].filter(function (x) { return x; }).join(" · "));
    // R-C1-59 (M6): an entitled cloud vault never runs the local runtime — `judge_plan` (scheduler.rs)
    // never passes `--runtime`/`--model` once `config/cloud.yaml` exists — so offering to download a
    // multi-gigabyte model there is dead weight the student would act on for nothing. Hidden only
    // once the account status is KNOWN to have an account: a missing or failed reply (the vault-less
    // shell, or a build from before Task 18) leaves the row exactly as visible as it always was.
    EL("set-judge").hidden = !!(a && a.ok && !a.needs_account);
  }
  // The account row and the upgrade overlay's gate, from ONE reply — `account_status` answers from
  // this machine only (no network call), and asking twice for the same three fields is two answers
  // that can disagree. A failed call is treated as "not reachable", not as "no account": the overlay
  // stays down and the console is exactly as usable as it was before C1. Best effort in both
  // directions — the vault-less shell does not register the command, and neither does a build made
  // before Task 18.
  function checkAccount() {
    return invoke("account_status", {}).then(function (s) {
      renderAccountRow(s);
      maybeUpgrade(s);
    }).catch(function () { renderAccountRow(null); EL("upgrade").hidden = true; });
  }
  function openSettings() {
    EL("settings").hidden = false;
    checkAccount();
    invoke("settings_context", {}).then(function (c) {
      current.vaultPath = c.vault; current.version = c.version; current.profileName = c.profile_name;
      current.registryError = c.registry_error || "";
      return invoke("get_settings", {});
    }).then(function (r) { if (r.ok) { renderSettings(r.settings); } }).then(loadInference).catch(function () {});
  }
  window.KNOWLU_OPEN_SETTINGS = openSettings;

  EL("settings").addEventListener("click", function (e) {
    if (e.target.closest("#set-close")) { EL("settings").hidden = true; return; }
    if (e.target.closest("#set-name-save")) {
      invoke("set_profile_name", { name: EL("set-name-in").value }).then(function (r) {
        EL("set-diag-note").textContent = r.ok ? "saved" : r.error;
        if (r.ok) { current.profileName = r.profile.name; }
      }).catch(function () {});
      return;
    }
    if (e.target.closest("#set-vault-copy")) { invoke("copy_text", { text: current.vaultPath || "" }).catch(function () {}); return; }
    // R-P4a-15: back to the picker. The window goes with it — this process is the one relaunching.
    if (e.target.closest("#set-switch")) { invoke("switch_profile", {}).catch(function () {}); return; }
    if (e.target.closest("#set-bdir-pick")) {
      invoke("pick_folder", { title: "Choose a backup folder" }).then(function (p) {
        if (!p || !p.path) { return; }
        return invoke("set_settings", { patch: { backup_dir: p.path } }).then(function (r) { if (r.ok) { renderSettings(r.settings); } });
      }).catch(function () {});
      return;
    }
    if (e.target.closest("#set-backup-now")) { invoke("backup_now", { view: stateView() }).then(applyEnvelope).catch(function () {}); return; }
    if (e.target.closest("#set-diag-copy")) {
      invoke("copy_diagnostics", {}).then(function (r) { EL("set-diag-note").textContent = r.ok ? "copied" : r.error; }).catch(function () {});
      return;
    }
    // Plan 4a Task 8: the Updates row's button. `checkForUpdates` repaints this row itself while
    // the panel is open, so there is one place that formats the update line and not two.
    if (e.target.closest("#set-update-check")) { checkForUpdates(); return; }
    // The runtime first, then the model: the runtime is the prerequisite, and one button does the
    // next thing rather than two doing halves. **Task 10 review, m2**: which half is missing comes
    // from the status object `loadInference` hands back, never from re-reading the row's own
    // rendered text — a UI string ("ready — X GB model") is not state, and once ready it no longer
    // contains "runtime" at all, so parsing it would silently target "model" forever, including a
    // stray re-fetch of gigabytes once both halves were already installed.
    if (e.target.closest("#set-judge-download")) { loadInference().then(function (s) {
      installInference((s && s.ok && !s.runtime) ? "runtime" : "model", false);
    }); return; }
    if (e.target.closest("#set-judge-file")) { loadInference().then(function (s) {
      installInference((s && s.ok && !s.runtime) ? "runtime" : "model", true);
    }); return; }
    if (e.target.closest("#set-judge-remove")) {
      invoke("remove_inference_model", {}).then(loadInference).catch(function () {});
      return;
    }
  });
  EL("set-autostart-in").addEventListener("change", function () {
    invoke("set_settings", { patch: { autostart: EL("set-autostart-in").checked } })
      .then(function (r) { if (r.ok) { renderSettings(r.settings); } else { EL("set-diag-note").textContent = r.error; } }).catch(function () {});
  });
  // The overlay covers the page, so it needs the page's usual way out as well as its Close button.
  // Guarded on the panel being open: while it is hidden, Escape stays the in-place editor's
  // (editField binds its own on the input) and this handler must not shadow it.
  document.addEventListener("keydown", function (e) {
    // Tested topmost-first, which is DOM order among panels that share a z-index: the upgrade
    // overlay is the last `.setpanel` in the page, the report is next, and the settings panel is
    // underneath both. The upgrade overlay's Escape is *Not now*, flag and all — a panel that came
    // back on the next `checkAccount` would not be dismissed, it would be postponed by a second.
    if (e.key === "Escape" && !EL("upgrade").hidden) { UPGRADE_DISMISSED = true; EL("upgrade").hidden = true; return; }
    // The report overlay is the same `.setpanel` shape over the same page, so it gets the page's own
    // way out (R-C1-55, M3). Tested before the settings panel because it opens on top of it.
    if (e.key === "Escape" && !EL("report").hidden) { EL("report").hidden = true; return; }
    if (e.key === "Escape" && !EL("settings").hidden) { EL("settings").hidden = true; }
  });
  // ---- C1 Task 17: the issue report (legal note §9). **The text the user reads is the payload** —
  // `report.rs` builds and scrubs it, the textarea shows it, and the send posts exactly what is on
  // screen. Nothing is rebuilt after the user has looked away.
  function openReport() {
    EL("report").hidden = false;
    EL("report-note").textContent = "";
    EL("report-text").value = "Loading…";
    invoke("report_preview", { view: stateView() }).then(function (r) {
      EL("report-text").value = r.ok ? r.text : ("could not build the report: " + r.error);
    }).catch(function () { EL("report-text").value = "could not build the report"; });
  }
  window.KNOWLU_OPEN_REPORT = openReport;
  EL("report-cancel").addEventListener("click", function () { EL("report").hidden = true; });
  EL("set-report-go").addEventListener("click", openReport);
  EL("report-send").addEventListener("click", function () {
    EL("report-send").disabled = true;
    // Exactly what is on screen. Rebuilding it here would send something the user never read.
    invoke("report_send", { text: EL("report-text").value }).then(function (r) {
      EL("report-note").textContent = r.ok ? "Sent. Thank you." : r.error;
      EL("report-send").disabled = false;
      if (r.ok) { setTimeout(function () { EL("report").hidden = true; }, 1500); }
    }).catch(function () { EL("report-note").textContent = "could not send"; EL("report-send").disabled = false; });
  });
  // R-C1-57 (I2): *Manage subscription* says why nothing opened. `open_portal` refuses with a
  // sentence whenever there is no live session, no account on this vault, or Stripe answers
  // anything but a link — and a button that silently does nothing reads as a broken app.
  EL("set-portal").addEventListener("click", function () {
    invoke("open_portal", {}).then(function (r) {
      if (!r.ok) { EL("set-account-state").textContent = r.error; }
    }).catch(function () { EL("set-account-state").textContent = UNREACHABLE; });
  });
  // Two presses, because the second one deletes a folder full of somebody's work and their account
  // with it. The first press only reveals the second.
  EL("set-delete-1").addEventListener("click", function () {
    EL("set-delete-2").hidden = false;
    EL("set-delete-note").textContent = "This deletes your vault, your backups, your account and everything we hold. It cannot be undone.";
  });
  EL("set-delete-2").addEventListener("click", function () {
    EL("set-delete-2").disabled = true;
    invoke("delete_my_data", {}).then(function (r) {
      EL("set-delete-note").textContent = r.ok ? "Deleted. Knowlu will close." : r.error;
      EL("set-delete-2").disabled = !r.ok;
    }).catch(function () { EL("set-delete-note").textContent = "could not delete"; EL("set-delete-2").disabled = false; });
  });

  // ---- C1 Task 18: an install made before C1 is upgraded here, in place (spec §11a). It is never
  // re-onboarded, it is never asked where its folder is, and nothing about it moves — the overlay
  // ends in `attach_account`, which writes `config/cloud.yaml` beside the config files that are
  // already there.
  //
  // **Two things it must never do**, and both were real: cover today's page with no way past, and
  // cover it at all when the account service cannot be reached. Spec §5.1 and D4 both promise that a
  // dead connection never hides today's page — the scheduler's 72-hour grace exists for exactly this
  // — and an overlay that walks past that promise is worse than no overlay. So: *Not now* dismisses
  // it for the session, and an unreachable service does not raise it in the first place. It comes
  // back on the next launch, which is the right cadence for a thing that has to happen once.
  // `#upgrade` is a `setpanel`, the same side panel `#settings` is — it does not cover the page, and
  // the console under it keeps ranking, scrolling and being clicked. These two flags are the rest of
  // the promise: `UPGRADE_DISMISSED` is *Not now*, and `UPGRADE_UNREACHABLE` is what a failed attempt
  // sets, so a student on a train is asked once and then left alone until the next launch.
  var UPGRADE_DISMISSED = false;
  var UPGRADE_UNREACHABLE = false;
  function maybeUpgrade(s) {
    // R-C1-57 (M4): never raise it over the settings panel. They are both `.setpanel`s in the same
    // fixed corner at the same z-index and `#upgrade` is last in the DOM, so it would land on top of
    // whatever the user had just opened — and `openSettings` calls this on its way in. Nothing is
    // hidden either: an overlay already up stays up. The next `checkAccount` raises it.
    if (!EL("settings").hidden) { return; }
    if (UPGRADE_DISMISSED || UPGRADE_UNREACHABLE || !s || !s.needs_account) { EL("upgrade").hidden = true; return; }
    EL("upgrade").hidden = false;
  }
  EL("up-later").addEventListener("click", function () {
    UPGRADE_DISMISSED = true;
    EL("upgrade").hidden = true;
  });
  /** A sign-in that could not reach the service at all stands the overlay down for this session and
   *  says why once. Not an error dialog: there is nothing the user can do about a dead network, and
   *  today's page is right there underneath. */
  function upgradeUnreachable() {
    UPGRADE_UNREACHABLE = true;
    EL("up-error").textContent = "";
    EL("upgrade").hidden = true;
  }
  /** The overlay is the console window's own surface and nothing paints it, so its busy state is one
   *  flag and two writes rather than a `WIZ` field. Without it, two presses on Continue with Google
   *  are two commands, two loopback listeners and two browser tabs — and the second callback meets a
   *  closed port. */
  var UP_BUSY = false;
  function upBusy(on) {
    UP_BUSY = on;
    EL("up-google").disabled = on;
    EL("up-magic").disabled = on;
  }
  /** What both doors do once a session exists — the tail the old branch ended with, now that two
   *  branches share it. */
  function afterUpgradeSignIn() {
    EL("up-error").textContent = "";
    EL("up-code-row").hidden = true;
    EL("up-subscribe").hidden = false;
    return checkEntitled().then(function (yes) {
      if (yes) { return finishUpgrade(); }
    });
  }
  EL("upgrade").addEventListener("click", function (e) {
    // The same branch the wizard has, and it matters more here: this window has a working console to
    // lose, and a plain navigation to `terms.html` would lose it while the user is ticking the box
    // that says they accept it.
    var policy = e.target.closest("a.policy");
    if (policy) {
      e.preventDefault();
      invoke("open_policy", { which: policy.getAttribute("data-policy") }).catch(function () {});
      return;
    }
    if (e.target.closest("#up-google")) {
      if (UP_BUSY) { return; }
      if (!(EL("up-18").checked && EL("up-terms").checked)) {
        EL("up-error").textContent = "Tick both boxes to create an account."; return;
      }
      upBusy(true);
      EL("up-error").textContent = "Finish signing in, in your browser…";
      // **`ageAttested`, not the Rust spelling** — Tauri v2 camel-cases every argument key, and this
      // is the checkbox `google_sign_in` now refuses on before it binds a listener (F2).
      invoke("google_sign_in", { ageAttested: EL("up-18").checked }).then(function (r) {
        upBusy(false);
        // A dead network stands the overlay down rather than trapping someone behind it (D4).
        if (!r.ok && String(r.error || "").indexOf(UNREACHABLE) === 0) { upgradeUnreachable(); return; }
        if (!r.ok) { EL("up-error").textContent = r.error; return; }
        return afterUpgradeSignIn();
      }).catch(function () { upBusy(false); upgradeUnreachable(); });
      return;
    }
    if (e.target.closest("#up-magic")) {
      if (UP_BUSY) { return; }
      if (!(EL("up-18").checked && EL("up-terms").checked)) {
        EL("up-error").textContent = "Tick both boxes to create an account."; return;
      }
      upBusy(true);
      // **`ageAttested`, not the Rust spelling** — Tauri v2 camel-cases every argument key, and the
      // wizard's own call carries the same comment for the same reason.
      invoke("send_magic_link", { email: EL("up-email").value.trim(), ageAttested: EL("up-18").checked }).then(function (r) {
        upBusy(false);
        if (!r.ok && String(r.error || "").indexOf(UNREACHABLE) === 0) { upgradeUnreachable(); return; }
        EL("up-error").textContent = r.ok ? "We emailed you a code. Type it below." : r.error;
        EL("up-code-row").hidden = !r.ok;
      }).catch(function () { upBusy(false); upgradeUnreachable(); });
      return;
    }
    if (e.target.closest("#up-code-go")) {
      if (UP_BUSY) { return; }
      upBusy(true);
      invoke("verify_email_code", { email: EL("up-email").value.trim(), code: EL("up-code").value.trim() }).then(function (r) {
        upBusy(false);
        if (!r.ok && String(r.error || "").indexOf(UNREACHABLE) === 0) { upgradeUnreachable(); return; }
        if (!r.ok) { EL("up-error").textContent = r.error; return; }
        return afterUpgradeSignIn();
      }).catch(function () { upBusy(false); upgradeUnreachable(); });
      return;
    }
    if (e.target.closest("#up-subscribe")) {
      // R-C1b-exec-9: ask before opening a second Checkout page. An account the service already
      // calls entitled — its own poll below gave up, or the student left and came back — goes
      // straight through; only an account that is still not entitled gets a Checkout tab.
      checkEntitled().then(function (yes) {
        if (yes) { return finishUpgrade(); }
        return invoke("open_checkout", { plan: "monthly" }).then(function (r) {
          // R-C1-57 (I2): read the envelope. Every refusal `open_checkout` can answer with — a dead
          // session, a refused base, a non-2xx from Stripe, a reply with no link in it — used to be
          // silence plus a two-minute poll for an entitlement no Checkout page was ever opened to buy.
          if (!r.ok) { EL("up-error").textContent = r.error; return; }
          EL("up-error").textContent = "";
          var tries = 0;
          var tick = function () {
            // The overlay can close — a sign-out, a re-render — while this is still ticking; nothing
            // asked it to stop, so it kept polling behind a hidden panel until the count ran out.
            if (EL("upgrade").hidden) { return; }
            tries += 1;
            checkEntitled().then(function (yes2) {
              // The re-review's blocking finding: the overlay can close while this reply is still in
              // flight, and a yes that lands after must not act on a panel nobody is looking at
              // either — the same stand-down, checked again now that the wait is over.
              if (EL("upgrade").hidden) { return; }
              if (yes2) { return finishUpgrade(); }
              if (tries < 40) { setTimeout(tick, 3000); }
              else { EL("up-error").textContent = "Still not subscribed. When the payment page is done, press Subscribe again."; }
            });
          };
          setTimeout(tick, 3000);
        });
      }).catch(function () { EL("up-error").textContent = UNREACHABLE; });
    }
  });
  // `poll()` is the console's repaint — this file has no `refresh`. The attach wrote
  // `config/cloud.yaml` and moved the session, so the next paint is the first one with an account.
  function finishUpgrade() {
    return invoke("attach_account", {}).then(function (r) {
      if (!r.ok) { EL("up-error").textContent = r.error; return; }
      EL("upgrade").hidden = true;
      poll();
    });
  }

  // Plan 4a Task 2: the console is one of three things this window can be. `launch_state` says
  // which: a console over a resolved profile, the picker (more than one profile), or the wizard
  // (none). The console's own listeners are bound only on the console path — a picker window has
  // no deck, no nav and no poll.
  function bootConsole() {
    window.addEventListener("hashchange", function () { route(location.hash.slice(1)); });
    window.addEventListener("focus", poll);
    setInterval(poll, 60000);
    // Task 15/R37: mark_seen is the END of a look, not the start — the boot's first paint must
    // show what changed since the PREVIOUS look, so stamping it away before the reader has even
    // seen it would erase the very delta this paint exists to show. `blur` (switched away) and
    // `visibilitychange` → hidden (minimized, tab/app switched, screen locked) are the only two
    // ways a look ends that the page can observe; the boot line calls route() and nothing else.
    window.addEventListener("blur", endOfLook);
    document.addEventListener("visibilitychange", function () { if (document.hidden) { endOfLook(); } });
    route(location.hash.slice(1));
    // Phase 2 (D1, D3, Q11-d): on the vault's first day the confirm screen opens, once a launch.
    checkWeekSetup();
    // Plan 4a Task 8: one check at launch. The housekeeping thread repeats it once a day; a
    // failure is quiet on both paths — until the release host exists, unreachable IS the answer.
    checkForUpdates();
    // Spec §11a: and one look at the account, which is what raises the upgrade overlay on the first
    // launch after C1 for an install that already existed. Once per launch, never on a timer — an
    // install that has an account never sees it, and one that dismissed it is asked again tomorrow.
    checkAccount();
  }

  function renderPicker(l) {
    var ps = l.profiles || [];
    EL("picker").hidden = false;
    document.querySelector(".app").hidden = true;
    EL("pick-lede").textContent = ps.length ? "Which vault?" : "No profile on this machine yet.";
    EL("pick-list").innerHTML = ps.map(function (p) {
      return '<div class="row pick" data-profile="' + h(p.id) + '"><div class="ttl"><span class="a">' + h(p.name) +
        '</span><span class="meta">' + h(p.vault) + "</span></div>" +
        '<div class="acts"><button class="b pri y" data-open="' + h(p.id) + '">Open</button></div></div>';
    }).join("");
  }

  EL("pick-list").addEventListener("click", function (e) {
    var b = e.target.closest("button[data-open]"); if (!b) { return; }
    b.disabled = true;
    invoke("open_profile", { id: b.getAttribute("data-open") }).catch(function () { b.disabled = false; });
  });
  EL("pick-adopt").addEventListener("click", function () {
    invoke("pick_folder", { title: "Choose your Knowlu vault folder" }).then(function (r) {
      if (!r || !r.path) { return; }
      return invoke("adopt_vault", { path: r.path, name: null }).then(function (a) {
        if (!a.ok) { EL("pick-lede").textContent = a.error; return; }
        return invoke("open_profile", { id: a.profile.id });
      });
    }).catch(function () {});
  });

  // C3' Task 9, step 6 (P3: built to yes): the LOCAL `Backups\` mirror, never the account — the
  // account's own copy arrives by signing in, through the wizard's own Finish (H11a), with no route
  // and no code to type. `pick_folder` and `restore_vault` have both existed since C1 with no caller;
  // this is the first one. Only two fields of a restored vault's plan are ever read
  // (`finish_or_roll_back` -> `finish_profile_in`: the autostart choice and the local-judgment offer
  // marker), so the rest of this minimal plan is never looked at — the fields still have to be the
  // shapes `WizardPlan` requires to deserialize.
  //
  // M3 (fix round 1): a name and an autostart choice, asked rather than assumed — the SAME two
  // things panel 4 and panel 8 of the nine-panel wizard ask, with the SAME defaults ("Knowlu",
  // Start Knowlu with Windows checked). Without a name, a second restore always landed at
  // `<home>\Knowlu\Knowlu` and the second one was refused outright ("already exists"); forcing
  // autostart enabled it without asking.
  var PICK_RESTORE_BACKUP = "";
  EL("pick-restore-backup").addEventListener("click", function () {
    invoke("pick_folder", { title: "Choose the Backups folder to restore from" }).then(function (r) {
      if (!r || !r.path) { return; }
      PICK_RESTORE_BACKUP = r.path;
      EL("pick-restore-name").value = "Knowlu";
      EL("pick-restore-autostart").checked = true;
      EL("pick-restore-options").hidden = false;
    }).catch(function () {});
  });
  EL("pick-restore-go").addEventListener("click", function () {
    // N6 (fix round 2): disabled before the call, the same guard the wizard's own Finish already
    // has (R2-3) — a double click used to start two `restore_vault` calls, the second losing the
    // rename race and writing its error into #pick-lede while the first was already relaunching.
    // Re-enabled on EITHER outcome: a refusal must leave the button pressable again.
    var go = EL("pick-restore-go");
    go.disabled = true;
    var name = EL("pick-restore-name").value.trim() || "Knowlu";
    var plan = {
      ics_url: null, personal_calendar: null, timezone: "", slots: ["12:00", "18:00"],
      zybooks: false, vhl: false, autostart: EL("pick-restore-autostart").checked,
    };
    invoke("restore_vault", { backup: PICK_RESTORE_BACKUP, name: name, plan: plan }).then(function (a) {
      if (!a.ok) { EL("pick-lede").textContent = a.error; go.disabled = false; return; }
      // N6 nit (fix round 3): open_profile can itself answer ok: false (the profile vanished, the
      // relaunch failed to spawn) — before this, that left the button disabled with no message at
      // all, the one outcome neither the wizard's own Finish nor pick-adopt's own chain guards
      // against either. Shown and re-enabled here rather than left silent.
      return invoke("open_profile", { id: a.profile.id }).then(function (o) {
        if (o && o.ok === false) { EL("pick-lede").textContent = o.error; go.disabled = false; }
      });
    }).catch(function () { go.disabled = false; });
  });

  // R-P4a-15, the picker's other door: the same wizard, opened from a machine that already has a
  // profile. The shell carries every wizard command, so nothing has to be relaunched to get here.
  EL("pick-add").addEventListener("click", function () {
    invoke("launch_state", {}).then(function (l) { startWizard(l || {}); }).catch(function () {});
  });

  // ---- C1 Task 17: onboarding (cloud design §4.2). NINE panels in this same document — no second
  // webview, no bundler. NOTHING reaches this machine's disk until Finish, except the account (which
  // is not this machine) and the coursework credentials, written the moment the user leaves panel 6
  // (decision 3).
  var PANELS = ["welcome", "account", "subscribe", "vault", "calendars", "logins", "gmail", "slots", "finish"];
  // **One string, two languages.** `account::UNREACHABLE` is the first clause of every transport
  // failure the Rust side emits, and this is the page's copy of it. They are pinned to each other by
  // `static_assets.rs::the_unreachable_clause_is_one_string_on_both_sides`, the same way `PRIVACY` is
  // pinned to the site's — because a guard that silently stops matching is worse than no guard.
  var UNREACHABLE = "the account service could not be reached";
  var PRIVACY = "Your tasks and notes live in a plain-text folder on this machine and in your Knowlu account, so every computer you sign in on opens on the same day; our servers keep them encrypted at rest, beside your account, the judgments made for you and what you correct, and none of it is ever sold or shared.";
  // Escaped on purpose: the shipped page carries no bare network literal (console spec §7).
  var ICS_OK = /^https:\/\/\S+(\.ics($|\?)|\/calendar\/)/i;
  // Windows' reserved device names, matched on the part before the first dot — the same list
  // `onboarding.rs`'s RESERVED holds (R-P4a-23).
  var RESERVED_NAME = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;
  // `parent` comes from `launch_state.default_parent` (`%USERPROFILE%\Knowlu`) and is never picked:
  // spec §4.1 — the app creates the folder and nobody is asked about it. It is still in WIZ because
  // `dest()` is what the credential target is derived from, and a rename on the vault panel has to
  // move the coursework logins with it (R-P4a-23).
  var WIZ = { step: 0, parent: "", name: "Knowlu", email: "", accountId: "", entitled: false,
              // The Google round trip runs in the system browser and can take a minute; `busy` is
              // what disables #wiz-google-signin meanwhile and `accountNote` is the status line —
              // both painted by `renderWizard`, the file's own A-5 rule (state lives on WIZ, never
              // written to the DOM straight from inside a click handler).
              busy: false, accountNote: "",
              ics: "", icsNote: "", cal: "", calNote: "",
              // Task 11 re-review N1 (R-C3'-exec-41): what the paste or capture said about the link it
              // checked — `{ url, stored }` when it validated, `null` when it was refused. `feedFlags`
              // turns it into the plan's per-feed flags at Finish, for the value the field holds then.
              icsFeed: null, calFeed: null,
              // C2 final review A-5 (m59+m60): the Google flow's own state, rendered by
              // `renderWizard()` like every other wizard field — a direct DOM write from inside
              // the click handler was invisible to it and got wiped by the next render a Back, a
              // Next or an unrelated poll caused. `google` is whether the calendar grant is
              // connected (what `wizFinish` used to read off a bare module-level `WIZ_GOOGLE`);
              // `googleNote` is the status line (its own element now — `wiz-cal-note` belongs to
              // the personal iCal field, not this one); `googlePolling` drives the button's
              // disabled state for the WHOLE poll, not only after it succeeds; `googleSeq` is the
              // cancellation token `wizGo` bumps on leaving the panel, the same shape
              // `schoolSeq` uses for the typeahead.
              google: false, googleNote: "", googlePolling: false, googleSeq: 0,
              // R-OB-4: the school the student picked — a unitid, a name, a state and (once
              // something establishes it) an LMS kind. The LIST is never here: `campus_search` is a
              // command, and the page holds only the ten rows it is showing.
              campus: { unitid: "", name: "", state: "", lms: "" },
              // R-OB-1 and R-OB-2. `map` is one row per discovered book/section, each with the
              // student's confirmed course; `courses` is the enrolment, captured or typed. Both end
              // up in the plan, and both are allowed to be empty — a student with no coursework
              // logins has nothing to map, and a campus whose API we cannot read is typed in.
              map: [], courses: [],
              // **`campus` is declared once, above.** A second `campus:` key here would silently
              // replace the choice object with a string, `WIZ.campus.unitid` would be `undefined` in
              // every `lms_link` call, and `wizFinish` would send a `campus_choice` serde cannot read.
              // The bundled list is gone with it: Task 14c takes it out of `launch_state`.
              // R-C1-42: the sign-in window's own state is one boolean — the session directory is the
              // app's and never crosses the IPC, and `close_lms_window` takes no argument.
              // R-C1-55: `discovering` is the in-flight latch on `coursework-discover` (a second
              // Next would log into both vendors again, concurrently, and the two answers would
              // decide the mapping panel between them); `checkoutOpened` is what makes the
              // subscribe panel silent until the browser has actually been sent somewhere;
              // `schoolSeq` drops a typeahead answer a later keystroke has already overtaken.
              // R-C1b-exec-10: `discovered` is whether a discovery has FINISHED at least once,
              // rows or none — the first `coursework` run files an empty parse as an issue rather
              // than an empty semester, so a student who saw nothing here can still go on honestly.
              // R-C1c-plan-3: `mapWarned` is whether the blank-row sentence has been shown once.
              // `wizStep` sets `WIZ.step` BEFORE the panel branch runs and `renderWizard` hides
              // every other panel, so a sentence written on the way out is a sentence nobody reads
              // — the first Next stays on the panel to show it, the second goes on.
              lmsOpen: false, discovering: false, discovered: false, mapWarned: false, checkoutOpened: false, schoolSeq: 0,
              tz: "", tzTouched: false, slots: ["12:00", "18:00"], autostart: true,
              zy: false, vhl: false, credVault: "", error: "",
              // M1 (fix round 1): what Finish's own restore found, painted the ordinary A-5 way —
              // set once `create_vault`/`restore_vault` resolves (`restoreSentence`, in `wizFinish`),
              // read here by `renderWizard` like every other wizard field. Blank until then: a
              // sentence claiming an outcome before Finish has even run is the bug this replaces.
              restoreNote: "" };

  // The trim and the trailing-separator strip are not cosmetic. `dest_for` in onboarding.rs trims
  // both halves and joins them with PathBuf::join, which never doubles a separator — and
  // storeCredentials derives the credential target from the string THIS function produced. A page
  // that spelled the path differently from the backend would file a password under a target name
  // the new vault's own ingest.yaml never mentions, and the first coursework run would find nothing.
  function dest() {
    var p = WIZ.parent.trim().replace(/[\\/]+$/, ""), n = WIZ.name.trim();
    return p && n ? p + "\\" + n : "";
  }

  // R-C1-42: one sentence for a validated school feed, wherever it came from. The courses clause is
  // added ONLY above zero — a real Blackboard feed reported 156 events across 0 courses, because
  // `summarise` counts only SUMMARY lines that lead with a course code, and "across 0 courses" reads
  // as a failure to a student whose feed is perfectly good.
  function feedSummary(link) {
    var s = "Found " + link.events + " events";
    if (link.courses > 0) { s += " across " + link.courses + " courses"; }
    return s + ".";
  }

  function startWizard(l) {
    EL("wizard").hidden = false;
    EL("picker").hidden = true;
    document.querySelector(".app").hidden = true;
    WIZ.tz = l.tz || "";
    WIZ.parent = l.default_parent || "";
    EL("wiz-name").value = WIZ.name;
    EL("wiz-privacy").textContent = PRIVACY;
    EL("wiz-tz").value = WIZ.tz;
    EL("wiz-slot1").value = WIZ.slots[0];
    EL("wiz-slot2").value = WIZ.slots[1];
    renderWizard();
  }

  function renderWizard() {
    PANELS.forEach(function (p, i) { EL("wiz-" + p).hidden = i !== WIZ.step; });
    EL("wiz-step").textContent = "step " + (WIZ.step + 1) + " of " + PANELS.length;
    // Spec §7 (a): hidden, not disabled. A greyed Back that does nothing is the control Quinn
    // pressed; a Back that is not on screen at step 0 asks no question.
    EL("wiz-back").hidden = WIZ.step === 0;
    EL("wiz-next").textContent = WIZ.step === PANELS.length - 1 ? "Finish" : "Next";
    // Spec §7 (b): the ONE writer of this property. `wizGo`'s discovery latch and `wizFinish`'s
    // re-entry guard both set `WIZ.busy` and call renderWizard, so a path that forgets to clear it
    // is a path renderWizard still recovers from on the next render — which a direct DOM write was
    // not. The file's own A-5 rule, applied to the last field that escaped it.
    EL("wiz-next").disabled = WIZ.busy;
    EL("wiz-error").textContent = WIZ.error;
    EL("wiz-account-note").textContent = WIZ.accountId ? "Signed in as " + WIZ.email : WIZ.accountNote;
    EL("wiz-google-signin").disabled = WIZ.busy;
    // F10: the emailed-code door spends the same 20/hour budget as the Google button, so two quick
    // presses are worth the same guard.
    EL("wiz-magic").disabled = WIZ.busy;
    EL("wiz-code-go").disabled = WIZ.busy;
    // Silent until the checkout page has actually been opened: on a panel nobody has pressed yet,
    // "waiting for your browser" reads as *a page failed to open* (R-C1-55, M1).
    EL("wiz-sub-note").textContent = WIZ.entitled ? "Your subscription is active."
      : (WIZ.checkoutOpened ? "Waiting for the payment page in your browser…" : "");
    EL("wiz-vault-path").textContent = dest() ? "Your files will be at " + dest() : "";
    EL("wiz-lms-state").textContent = WIZ.icsNote;
    // R-C1-42: the capture is a SECOND press, and it only exists once a window is open. Firing it as
    // the window opens would read the identity provider's page, not the school's — the student has
    // not signed in yet, and the answer would be "finish signing in first" every time.
    EL("wiz-lms-capture").hidden = !WIZ.lmsOpen;
    EL("wiz-ics-note").textContent = WIZ.ics && !ICS_OK.test(WIZ.ics) ? "That does not look like a calendar feed link." : "";
    EL("wiz-cal-note").textContent = WIZ.calNote;
    // A-5: driven entirely by WIZ state, so returning to this panel (or any other re-render while
    // on it) always shows the truth — connected, mid-poll, or neither — never a stale DOM write.
    EL("wiz-google-note").textContent = WIZ.googleNote;
    EL("wiz-google").disabled = WIZ.google || WIZ.googlePolling;
    EL("wiz-summary").textContent = dest() + ", looking at " + WIZ.slots.join(" and ") + " " + WIZ.tz + ".";
    // M1 (fix round 1): painted from WIZ, like every other wizard field — blank until `wizFinish`
    // has an actual answer from `restore_into`, never a claim made before Finish has even run.
    EL("wiz-restore-note").textContent = WIZ.restoreNote;
  }

  // Panel 6 leaves: write whatever was typed straight into Credential Manager, then clear the
  // inputs — a crash must never leave a password in page memory longer than it has to be. On a
  // FAILED write the fields stay full (review round 1, IMPORTANT 2): clearing them would leave a
  // panel the user cannot retype into and a flag claiming a login that is not there.
  function clearCredentialFields() {
    ["wiz-zy-pass", "wiz-vhl-pass", "wiz-zy-user", "wiz-vhl-user"].forEach(function (id) { EL(id).value = ""; });
  }
  function storeCredentials() {
    var jobs = [], stored = [], half = false;
    [["zybooks", "wiz-zy-user", "wiz-zy-pass"], ["vhl", "wiz-vhl-user", "wiz-vhl-pass"]].forEach(function (t) {
      var u = EL(t[1]).value.trim(), s = EL(t[2]).value;
      if (!u && !s) { return; }
      // Half a login is a typo, not a choice. Skipping it silently left the user certain they had
      // connected zyBooks (review round 1, minor).
      if (!u || !s) { half = true; return; }
      stored.push(t[0]);
      // `dest()`: the vault does not exist yet, so the credential target is derived from the path
      // Finish will create — the same path `create_vault` derives the vault's own
      // `credential_target:` lines from, so the two always name the same entry.
      jobs.push(invoke("store_credentials", { vault: dest(), source: t[0], user: u, secret: s }));
    });
    if (half) { WIZ.error = "Enter both the email and the password, or leave both empty."; return Promise.resolve(false); }
    if (!jobs.length) { return Promise.resolve(true); }
    return Promise.all(jobs).then(function (rs) {
      var bad = rs.filter(function (r) { return r && !r.ok; });
      if (bad.length) { WIZ.error = bad[0].error; return false; }
      // Only now. These flags become `enabled: true` and a `credential_target:` line in the new
      // vault's ingest.yaml; set before the write resolved, they promised a login that a failed
      // write had not stored (review round 1, IMPORTANT 2).
      stored.forEach(function (src) { if (src === "zybooks") { WIZ.zy = true; } else { WIZ.vhl = true; } });
      // Re-typed logins are a new answer: a discovery already finished for the OLD ones must not
      // stand in for one against these.
      if (stored.length) { WIZ.discovered = false; WIZ.mapWarned = false; }
      WIZ.credVault = dest();   // R-P4a-23: the path these entries are keyed to.
      clearCredentialFields();
      return true;
    }).catch(function () { WIZ.error = "the credential could not be stored"; return false; });
  }

  // R-P4a review round 1, IMPORTANT 3: the slots panel's answers live in WIZ, read on every keystroke
  // and again on the way out, so the Finish summary and the plan `create_vault` receives are the same
  // numbers the user is looking at. **No campus read here** (R-OB-4): the school is the question
  // *which school?*, it belongs on the calendars panel where it is used, and reading it three panels
  // after `open_lms_window` needed it is why every sign-in used to answer "no sign-in page is known
  // for that school yet".
  function readSlotsPanel() {
    WIZ.tz = EL("wiz-tz").value.trim() || WIZ.tz;
    WIZ.slots = [EL("wiz-slot1").value.trim() || "12:00", EL("wiz-slot2").value.trim() || "18:00"];
    WIZ.autostart = EL("wiz-autostart").checked;
  }

  // A refusal belongs on screen AND has to register as an answer to THIS press. `#wiz-error` sits
  // at the left of the nav row, so a second Next against the same unmet gate rewrote the same red
  // sentence and looked like nothing happened at all. One re-flow, one animation frame, and the
  // sentence arrives again.
  function flashError() {
    var el = EL("wiz-error");
    el.classList.remove("flash");
    void el.offsetWidth;
    el.classList.add("flash");
  }

  // The discovery command states a reason and nothing else now (onboarding.rs's own rule); the
  // page is what appends the way forward, and a reason that already ends in a stop needs its own
  // trimmed before the page's own sentence follows it.
  function tidy(s) { return String(s || "").replace(/[.\s]+$/, ""); }

  function wizValid() {
    WIZ.error = "";
    if (WIZ.step === 1 && !WIZ.accountId) { WIZ.error = "Sign in first."; }
    if (WIZ.step === 2 && !WIZ.entitled) { WIZ.error = "Finish the payment page in your browser, then come back."; }
    if (WIZ.step === 3) {
      var n = WIZ.name.trim();
      if (!n) { WIZ.error = "Give this setup a name."; }
      else if (/[\\/:]/.test(n)) { WIZ.error = "A name has no slashes or colons."; }
      else if (/[*?"<>|]/.test(n)) { WIZ.error = "A name has no * ? \" < > or | either."; }
      else if (/[. ]$/.test(n)) { WIZ.error = "A name cannot end in a dot or a space."; }
      else if (RESERVED_NAME.test(n.split(".")[0])) { WIZ.error = "That name is reserved by Windows."; }
    }
    // Panel 4 is the calendars panel. Neither feed is compulsory — a student with no personal
    // calendar still gets a ranked day, and one whose school defeats the capture can come back — but
    // a link that is there and malformed is caught here rather than at Finish.
    if (WIZ.step === 4 && WIZ.ics && !ICS_OK.test(WIZ.ics)) { WIZ.error = "That does not look like a calendar feed link."; }
    if (WIZ.step === 4 && WIZ.cal && !ICS_OK.test(WIZ.cal)) { WIZ.error = "That does not look like a secret iCal address."; }
    return !WIZ.error;
  }

  function wizGo(n) {
    // R-C1b-exec-9: the only thing that ever set WIZ.entitled true used to be the two-minute poll
    // below, and a student who came back to the wizard after that poll had already given up found
    // Next refusing forever with no way to ask again. One re-ask, here, before the refusal — and
    // never from inside it, since an account that truly is not entitled must not ask the service
    // forever.
    if (n > WIZ.step && WIZ.step === 2 && !WIZ.entitled) {
      WIZ.busy = true; renderWizard();
      return checkEntitled().then(function (yes) {
        WIZ.busy = false;
        if (yes) { WIZ.entitled = true; }
        return wizStep(n);
      });
    }
    return wizStep(n);
  }

  function wizStep(n) {
    if (n > WIZ.step && !wizValid()) { renderWizard(); flashError(); return Promise.resolve(); }
    // A refusal belongs to the panel that raised it: stepping back clears it rather than carrying
    // a red line about a field that is no longer on screen.
    if (n < WIZ.step) { WIZ.error = ""; }
    var leaving = WIZ.step;
    if (leaving === 7) { readSlotsPanel(); }
    // Leaving the calendar panel closes the sign-in window and deletes its session, whether or not a
    // link was captured: a campus login must not outlive the panel that opened it. R-C1-40/R-C1-42 —
    // no argument: the app knows which directory it opened, and the page never holds a temp path.
    if (leaving === 4 && WIZ.lmsOpen) {
      invoke("close_lms_window", {}).catch(function () {});
      WIZ.lmsOpen = false;
    }
    // A-5: cancel any Google poll in flight — bumping the token is enough, the loop checks it on
    // its own next wake and stops touching the page. Only the "still polling" flag is cleared here
    // (not `googleNote`): a consent that already finished, or one still open in the browser, is
    // still true when the student comes back to this panel, and the note should still say so.
    if (leaving === 4 && WIZ.googlePolling) {
      WIZ.googleSeq += 1;
      WIZ.googlePolling = false;
    }
    WIZ.step = Math.max(0, Math.min(PANELS.length - 1, n));
    if (leaving === 5 && n > leaving) {
      // R-C1-55, I3: one discovery at a time. `discover_coursework` spawns the engine and logs into
      // zyBooks and VHL; a second Next while the first is in flight starts a second child, with two
      // more vendor logins racing the first, and the two `WIZ.map` assignments decide the panel
      // between them. The latch refuses the re-entry and the disabled button says so on screen.
      if (WIZ.discovering) { WIZ.step = leaving; renderWizard(); return Promise.resolve(); }
      return storeCredentials().then(function (ok) {
        if (!ok) { WIZ.step = leaving; renderWizard(); return; }
        // R-OB-1: the credentials are in Credential Manager now, so this is the first moment discovery
        // can run. Stay on the panel while it does — the mapping is the whole point of having asked
        // for the logins — and let Next work again the moment the rows are on screen.
        if (!WIZ.zy && !WIZ.vhl) { renderWizard(); return; }
        if (WIZ.map.length || WIZ.discovered) {
          // R-C1c-plan-3: the first Next after a finished discovery with blank, un-ignored rows
          // writes the sentence and stays here — `WIZ.step` was advanced above, so putting it back
          // is what keeps the panel, and its note, on screen. The next Next goes on whatever the
          // rows say: this is a sentence, not a gate.
          if (!WIZ.mapWarned && noteUnmapped()) { WIZ.mapWarned = true; WIZ.step = leaving; }
          renderWizard();
          return;
        }
        WIZ.step = leaving;
        WIZ.discovering = true;
        WIZ.busy = true;
        EL("wiz-map").hidden = false;
        EL("wiz-map-note").textContent = "Looking up your books and sections…";
        renderWizard();
        return invoke("discover_coursework", { vault: dest(), zybooks: WIZ.zy, vhl: WIZ.vhl }).then(function (d) {
          WIZ.map = ((d && d.rows) || []).map(function (r) {
            return { source: r.source, key: r.key, detail: r.detail, suggested: r.suggested, course: r.suggested || "", ignore: !!r.ignored };
          });
          // R-C1b-exec-10: set BEFORE renderMapping() runs — the first `coursework` run files an
          // empty parse as an issue rather than an empty semester, so a finished discovery that
          // found nothing is still a finished discovery, and renderMapping() has to know that on
          // this very paint or the panel it is about to draw is the empty one nobody could see.
          WIZ.discovered = true;
          EL("wiz-map-note").textContent = WIZ.map.length
            ? (d && d.note ? tidy(d.note) + ". " : "Knowlu found these on your accounts. ") + "Confirm the course each one belongs to — without this, Knowlu can see the work but not what it is for."
            : tidy((d && d.note) || "We could not reach your coursework sites") + ". You can go on — Knowlu will try again on its first run.";
          renderMapping();
        }).catch(function () {
          WIZ.discovered = true;
          EL("wiz-map-note").textContent = "We could not look those up. You can go on — Knowlu will try again on its first run.";
          renderMapping();
        }).then(function () {
          // Both outcomes, always: a latch a rejected promise leaves set is a Next button that never
          // comes back. Repainted here rather than left for a later render: nothing else touches the
          // DOM once this chain settles, and a WIZ field nobody repaints is a Next button nobody can
          // press.
          WIZ.discovering = false;
          WIZ.busy = false;
          renderWizard();
        });
      });
    }
    renderWizard();
    return Promise.resolve();
  }

  function wizRegister(plan) {
    return invoke("create_vault", { name: WIZ.name, plan: plan });
  }

  // M1 (fix round 1): the one place all three of Finish's restore outcomes are worded, so a student
  // never reads a claim the actual result disagrees with. `restored` is `create_vault`'s own
  // `{notes, records, empty, warnings}` (H11a's `sync::Restored`, C3' Task 9). `empty` alone cannot
  // tell "the account truly has nothing yet" from "the account could not be reached just now" —
  // `restore_into` folds a failed pull into the same `empty: true` so Finish never rolls the vault
  // back over a hotel Wi-Fi — so `warnings` (non-empty only on the failed-pull fold) is what tells
  // the two apart here.
  // N3 (fix round 2): keyed on `ok` (an explicit field `create_vault`'s own envelope carries,
  // C3' Task 9's `sync::Restored.ok`), never on `warnings.length` — a perfectly successful pull can
  // carry warnings too (a refused sync card, an oversize row), and reading those as "could not be
  // reached" was the round 1 bug. Item 2 (N2's own follow-on): the offline sentence now says the
  // notes arrive over the next SYNCS, plural — one page at a time, so a large account takes more
  // than one.
  function restoreSentence(restored) {
    if (!restored) { return ""; }
    if (!restored.ok) { return "Your account could not be reached just now; your vault will fill in over the next syncs."; }
    if (!restored.empty) { return "Your account already had a vault here — it just came back."; }
    return "Your account had no vault yet — this is the first one.";
  }

  // R-P4a-23: the logins panel's entries are keyed to the path as it stood then, and Back → rename is
  // exactly what the refused-Finish panel asks for. Say where they went and send the user back to
  // panel 6 — the logins panel, which is where the fields are — rather than build a vault whose
  // ingest.yaml points at an entry that is not there.
  function credentialsStranded() {
    WIZ.error = "Your logins were saved for " + WIZ.credVault + "; re-enter them.";
    WIZ.zy = false;
    WIZ.vhl = false;
    WIZ.credVault = "";
    WIZ.step = 5;
    WIZ.busy = false;
    renderWizard();
  }

  function wizFinish() {
    // R2-3: disabled FIRST, before any await (including the `google_connected` re-read below) — a
    // second Finish click landing in that window used to start a second `retarget_credentials`/
    // `wizRegister` flow racing the first. Every failure path below re-enables it exactly as before.
    // Painted immediately, synchronously, before readSlotsPanel or any await: the DOM `disabled`
    // property is what actually stops a second physical click from ever reaching this function again
    // — `WIZ.busy` alone is an in-memory flag nothing reads at the door.
    WIZ.busy = true;
    renderWizard();
    readSlotsPanel();
    // R-OB-1 and R-OB-2: the confirmed mapping and the course list, in the shapes `WizardPlan` takes.
    // An ignored row contributes nothing but its place in `zybooks_ignore:`; a row with no course
    // contributes nothing at all, which leaves that source unmapped and is the student's choice to
    // have made. **`course` carries the confirmed course code and `label` the human title** (R-C1-47,
    // I3): `create_vault_in` slugs `course` and only `course` — a mapping sent with an empty one is
    // dropped there, which is a login taken and a book left `not in config; skipped` forever.
    var zyRows = WIZ.map.filter(function (r) { return r.source === "zybooks" && !r.ignore && r.course; });
    var vhlRows = WIZ.map.filter(function (r) { return r.source === "vhl" && !r.ignore && r.course; });
    var codes = {};
    zyRows.concat(vhlRows).forEach(function (r) { codes[r.course] = true; });
    // Final review, I4 (a C1 Task 17 bug predating this branch): only a TYPED course belongs here —
    // it carries `slug: ""`, and this is the only place anything derives one for it. A CAPTURED
    // course already carries its own slug and is covered by the engine's `course_fragments`
    // (`app/src/scaffold.rs`); sending it here too would give its LMS id a *second*, phantom
    // `[id, ""]` entry that `create_vault_in` derives an ENGINE slug for on the empty second
    // element — and because `course_map_lines` is first-wins by key with the page's entries first,
    // that phantom slug wins over the real one `course_fragments` would have written, so the id
    // ends up pointing at a course that doesn't exist (D4's `[LMS id -> slug]` guarantee broken).
    WIZ.courses.forEach(function (c) { if (c.code && !c.slug) { codes[c.code] = true; } });
    // A-5 (b): re-read the truth rather than trust the poll loop's last tick. A consent that
    // finished (in the browser, or after the poll was cancelled by leaving and returning to the
    // panel) after the loop last checked must still birth the vault with the `cloud:google` entry
    // — `WIZ.google` is only the FALLBACK, used when this one extra call itself fails.
    return invoke("google_connected").then(function (status) {
      return (status && status.ok) ? status.calendar === true : WIZ.google;
    }).catch(function () { return WIZ.google; }).then(function (googleCalendar) {
      var icsFlags = feedFlags(WIZ.icsFeed, WIZ.ics), calFlags = feedFlags(WIZ.calFeed, WIZ.cal);
      var plan = { ics_url: WIZ.ics || null, personal_calendar: WIZ.cal || null,
                   ics_validated: icsFlags.validated, ics_stored: icsFlags.stored,
                   personal_calendar_validated: calFlags.validated, personal_calendar_stored: calFlags.stored,
                   google_calendar: googleCalendar,
                   timezone: WIZ.tz, slots: WIZ.slots,
                   zybooks: WIZ.zy, vhl: WIZ.vhl, autostart: WIZ.autostart,
                   campus_choice: WIZ.campus,
                   zybooks_courses: zyRows.map(function (r) { return { code: r.key, course: r.course, label: r.course }; }),
                   vhl_sections: vhlRows.map(function (r) { return { section: r.key, course: r.course, label: r.course }; }),
                   // The second element is the slug, and the page has none: an empty string is what
                   // tells `create_vault_in` to derive one from the fragment with the ENGINE's rule.
                   course_map: Object.keys(codes).map(function (c) { return [c, ""]; }),
                   // Final review, I2: only the rows the student explicitly ticked as ignored — never
                   // a blank one. A blank zyBooks row must stay UNMAPPED so the engine files a
                   // coursework-map card for it (R-OB-1), the same as a blank VHL row; putting it in
                   // this list instead made it `not in config; skipped` on every healthy run, forever,
                   // and made noteUnmapped's "N of these will be asked about in the app" false for
                   // zyBooks.
                   zybooks_ignore: WIZ.map.filter(function (r) { return r.source === "zybooks" && r.ignore; })
                                          .map(function (r) { return r.key; }),
                   courses: WIZ.courses };
      // Before anything is created: move the credentials if the path has changed since they were
      // written, so Credential Manager and the vault's `credential_target:` lines agree the moment
      // the vault exists. The secret never comes back to the page — the move happens in the command.
      var moved = (WIZ.credVault && WIZ.credVault !== dest())
        ? invoke("retarget_credentials", { from_vault: WIZ.credVault, to_vault: dest() })
        : Promise.resolve({ ok: true, error: null });
      return moved.then(function (rt) {
        if (!rt || !rt.ok) { credentialsStranded(); return; }
        if (WIZ.credVault) { WIZ.credVault = dest(); }
        return wizRegister(plan).then(function (r) {
          if (!r.ok) { WIZ.error = r.error; WIZ.busy = false; renderWizard(); return; }
          // M1 (fix round 1): painted before the relaunch — `restoreSentence` reads what Finish's own
          // restore actually found, never the claim the OLD static sentence made before Finish ran.
          WIZ.restoreNote = restoreSentence(r.restored);
          renderWizard();
          // R-C1-31, corrected by the final review (Minor 8): `entitlement_now` writes NO cache —
          // it only reads the PENDING session, which `create_vault` has just moved onto this
          // profile, so this call typically finds nothing there any more and resolves to null. The
          // real first cache write is the in-slot refresh D1 added (`scheduler::run_slot_inner`,
          // spec §2), keyed off the profile's own vault and data dir. Left in place as a harmless
          // best-effort poll rather than removed here.
          // Best effort in both directions: a refusal, or a build where the command is not yet
          // registered, must never stop a finished wizard from opening.
          return invoke("entitlement_now", {}).catch(function () { return null; }).then(function () {
            return invoke("finish_onboarding", { id: r.profile.id });
          });
        });
      });
    }).catch(function (e) { WIZ.error = String(e.message || e); WIZ.busy = false; renderWizard(); });
  }

  // The Checkout page is in the system browser, so the app cannot be told when it is done: it asks.
  // Every three seconds for two minutes, then it stops and the button can be pressed again — a poll
  // that never ends is a poll that runs all night on a laptop somebody closed.
  function pollEntitlement() {
    var tries = 0;
    var tick = function () {
      // The student can leave panel 2 without waiting on this — Back, or the pre-ask in `wizGo`
      // already got a yes — and a poll that kept ticking behind a panel nobody is looking at is a
      // poll that outlives the question it was asked.
      if (WIZ.entitled || WIZ.step !== 2) { return; }
      tries += 1;
      checkEntitled().then(function (yes) {
        // The re-review's blocking finding: the student can leave panel 2 while this reply is still
        // in flight, and a yes that lands after they moved on must not act on a panel nobody is
        // looking at either — the same stand-down, checked again now that the wait is over.
        if (WIZ.entitled || WIZ.step !== 2) { return; }
        if (yes) {
          WIZ.entitled = true; WIZ.error = ""; renderWizard(); wizGo(3); return;
        }
        if (tries < 40) { setTimeout(tick, 3000); }
        else { WIZ.error = "Still not subscribed. When the payment page is done, press Next."; renderWizard(); }
      });
    };
    setTimeout(tick, 3000);
  }

  EL("wizard").addEventListener("click", function (e) {
    if (e.target.closest("#wiz-back")) { wizGo(WIZ.step - 1); return; }
    if (e.target.closest("#wiz-next")) { if (WIZ.step === PANELS.length - 1) { wizFinish(); } else { wizGo(WIZ.step + 1); } return; }
    // Spec D1. One command, no arguments: the URL, the loopback port, the verifier and both tokens
    // are Rust's, and the page never sees any of them. The browser round trip can take a minute, so
    // the button says what is happening — `WIZ.busy` is what renderWizard paints (Task 6).
    if (e.target.closest("#wiz-google-signin")) {
      if (!(EL("wiz-18").checked && EL("wiz-terms").checked)) {
        WIZ.error = "Tick both boxes to create an account."; renderWizard(); return;
      }
      WIZ.busy = true; WIZ.error = ""; WIZ.accountNote = "Finish signing in, in your browser…";
      renderWizard();
      // **`ageAttested`, not the Rust spelling** — the checkbox `google_sign_in` now refuses on
      // before it binds a listener (F2), the same gate `send_magic_link` has always had.
      invoke("google_sign_in", { ageAttested: EL("wiz-18").checked }).then(function (r) {
        WIZ.busy = false; WIZ.accountNote = "";
        if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        wizGo(2);
      }).catch(function () { WIZ.busy = false; WIZ.accountNote = ""; WIZ.error = UNREACHABLE; renderWizard(); });
      return;
    }
    if (e.target.closest("#wiz-magic")) {
      if (WIZ.busy) { return; }
      if (!(EL("wiz-18").checked && EL("wiz-terms").checked)) {
        WIZ.error = "Tick both boxes to create an account."; renderWizard(); return;
      }
      // **`ageAttested`, not the Rust spelling.** Tauri v2 camel-cases every argument key; sent
      // snake_case the invoke is rejected before the command's body runs.
      WIZ.busy = true; renderWizard();
      invoke("send_magic_link", { email: EL("wiz-email").value.trim(), ageAttested: EL("wiz-18").checked }).then(function (r) {
        WIZ.busy = false;
        WIZ.error = r.ok ? "" : r.error;
        EL("wiz-code-row").hidden = !r.ok;
        WIZ.accountNote = r.ok ? "We emailed you a code. Type it below." : "";
        renderWizard();
      }).catch(function () { WIZ.busy = false; WIZ.error = UNREACHABLE; renderWizard(); });
      return;
    }
    if (e.target.closest("#wiz-code-go")) {
      if (WIZ.busy) { return; }
      WIZ.busy = true; renderWizard();
      invoke("verify_email_code", { email: EL("wiz-email").value.trim(), code: EL("wiz-code").value }).then(function (r) {
        WIZ.busy = false;
        EL("wiz-code").value = "";
        if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
        WIZ.accountId = r.account_id; WIZ.email = r.email; WIZ.error = "";
        EL("wiz-code-row").hidden = true;
        wizGo(2);
      }).catch(function () { WIZ.busy = false; WIZ.error = UNREACHABLE; renderWizard(); });
      return;
    }
    // **The two policies open in the system browser, not in this window.** `app/static/` holds four
    // files, so a plain navigation to `terms.html` loses the only window Knowlu has — while the user
    // is being asked to tick a box saying they accept it. The `href` stays because it is the honest
    // markup and the static test reads it; this is what actually happens.
    var policy = e.target.closest("a.policy");
    if (policy) {
      e.preventDefault();
      invoke("open_policy", { which: policy.getAttribute("data-policy") }).catch(function () {});
      return;
    }
    if (e.target.closest("#wiz-sub-month") || e.target.closest("#wiz-sub-year")) {
      var which = e.target.closest("#wiz-sub-year") ? "academic_year" : "monthly";
      // R-C1b-exec-9: ask before opening a second Checkout page — an account the service already
      // calls entitled goes straight to the vault panel, never back through Stripe.
      checkEntitled().then(function (yes) {
        if (yes) { WIZ.entitled = true; WIZ.error = ""; renderWizard(); wizGo(3); return; }
        return invoke("open_checkout", { plan: which }).then(function (r) {
          if (!r.ok) { WIZ.error = r.error; renderWizard(); return; }
          WIZ.checkoutOpened = true;
          renderWizard();
          pollEntitlement();
        });
      }).catch(function () {});
      return;
    }
    if (e.target.closest("#wiz-lms-open")) {
      WIZ.icsNote = "Opening your school’s sign-in page…";
      renderWizard();
      invoke("open_lms_window", { unitid: WIZ.campus.unitid }).then(function (r) {
        if (!r.ok) {
          WIZ.icsNote = r.error;
          // Nobody has curated this school, so nobody knows which LMS it runs. Ask, once — the answer
          // is what C2's server-side fetch will need too.
          EL("wiz-lms-kind").hidden = !!WIZ.campus.lms;
          renderWizard();
          return;
        }
        // **R-C1-42: the capture is not chained onto the open.** The window has just landed on the
        // identity provider, the student has typed nothing yet, and a capture fired here reads the
        // login page. The second button is what says "I am signed in now".
        WIZ.lmsOpen = true;
        WIZ.icsNote = "Sign in there, then press the button beside this one.";
        renderWizard();
      }).catch(function () { WIZ.icsNote = "That did not work — paste the link below instead."; renderWizard(); });
      return;
    }
    if (e.target.closest("#wiz-lms-capture")) {
      WIZ.icsNote = "Looking for your calendar link…";
      renderWizard();
      invoke("capture_calendar_link", { unitid: WIZ.campus.unitid }).then(function (c) {
        if (c.ok && c.link) {
          WIZ.ics = c.link.url;
          WIZ.icsFeed = { url: c.link.url, stored: c.stored === true };
          WIZ.icsNote = feedSummary(c.link);
          // `note` is the one thing `PUT /account/sources` could not do. Finish tries that save again
          // and keeps the link on this machine if it still can't, so it is a sentence beside the
          // count, not a failure.
          if (c.note) { WIZ.icsNote += " " + c.note; }
        } else {
          WIZ.icsNote = (c.error || "No link found") + " — paste it below instead.";
        }
        renderWizard();
        // R-OB-2, in the same sitting and the same window: the enrolled course list. Its own
        // outcome — a campus can give the calendar and not the courses — so a failure here shows
        // the typed field and says nothing about the link that just worked.
        return invoke("capture_courses", { unitid: WIZ.campus.unitid }).then(function (cl) {
          WIZ.courses = (cl && cl.courses) || [];
          EL("wiz-courses").hidden = false;
          EL("wiz-courses-note").textContent = WIZ.courses.length
            ? "These are the classes Knowlu found. Remove any you are not taking."
            : "Knowlu could not read your class list — type the codes yourself, e.g. CS 100.";
          renderCourses();
        });
      }).catch(function () { WIZ.icsNote = "That did not work — paste the link below instead."; renderWizard(); });
      return;
    }
  });
  EL("wiz-name").addEventListener("input", function () { WIZ.name = EL("wiz-name").value; renderWizard(); });
  EL("wiz-ics").addEventListener("input", function () { WIZ.ics = EL("wiz-ics").value.trim(); renderWizard(); });
  EL("wiz-ics").addEventListener("change", function () {
    WIZ.ics = EL("wiz-ics").value.trim();
    if (!WIZ.ics) { WIZ.icsNote = ""; renderWizard(); return; }
    // The pasted path validates exactly as the captured one does — same command, same sentence.
    var sent = WIZ.ics;
    invoke("paste_calendar_link", { kind: "lms_ics", url: sent }).then(function (r) {
      WIZ.icsFeed = r.ok ? { url: sent, stored: r.stored === true } : null;
      WIZ.icsNote = r.ok ? feedSummary(r.link) : r.error;
      if (r.ok && r.note) { WIZ.icsNote += " " + r.note; }
      renderWizard();
    }).catch(function () {});
  });
  /// Task 11 re-review N1 (R-C3'-exec-41): the plan's two flags for one feed, true only for the
  /// value the field holds NOW. A link typed over after its check is neither validated nor stored,
  /// so Finish never sends it to the account (`onboarding::create_vault_in` keeps it in the vault).
  function feedFlags(feed, value) {
    var same = !!(feed && value && feed.url === value);
    return { validated: same, stored: same && feed.stored === true };
  }
  /// One row per enrolled course, removable (Task 14b, I2: the capture is unfiltered — past terms,
  /// organisations and TA roles come back with the rest, and the student is the only one who can say
  /// which are this semester's). The vault identifier is made in Rust at Finish, from the code; the
  /// page never invents one.
  function renderCourses() {
    EL("wiz-course-rows").innerHTML = WIZ.courses.map(function (c, i) {
      // R-C1c-plan-2: the human code first, the school's own name beside it — and just the name
      // when there was no code to read, or when the two are the same string (a typed course is
      // both). The page never reads a code out of a name: that rule lives in
      // `scaffold::course_code_in_name`, and a second copy here would drift from it in silence.
      var lead = (c.label && c.label !== c.name) ? h(c.label) + " &middot; " + h(c.name) : h(c.name || c.code);
      return '<div class="wiz-row" data-course="' + i + '"><span class="meta">' + lead +
             '</span><button class="b" data-drop="' + i + '">Remove</button></div>';
    }).join("");
    renderCourseCodes();
  }

  /// D5: the codes a mapping row offers, so a student picks a class rather than typing one from
  /// memory — the VHL row that nobody filled is why `sections: {}` reached the engine.
  ///
  /// The VALUE is the human code the capture read (R-C1c-plan-2's `label`) and the course's own
  /// slug otherwise: `create_vault_in` slugs whatever the row carries, and both of those slug to
  /// the note the course already has. The LMS's opaque key would not — it would make a second
  /// course. The LABEL is what the student recognises (M4), so the list reads as their class list
  /// rather than as identifiers.
  function renderCourseCodes() {
    EL("wiz-course-codes").innerHTML = WIZ.courses.map(function (c) {
      return '<option value="' + h(c.label || c.slug) + '">' + h(c.name || c.code) + "</option>";
    }).join("");
  }
  EL("wiz-courses").addEventListener("click", function (e) {
    var drop = e.target.closest("[data-drop]");
    if (drop) { WIZ.courses.splice(Number(drop.getAttribute("data-drop")), 1); renderCourses(); return; }
    if (e.target.closest("#wiz-course-add-go")) {
      var code = EL("wiz-course-add").value.trim();
      if (code) { WIZ.courses.push({ code: code, name: code, slug: "", label: code }); EL("wiz-course-add").value = ""; renderCourses(); }
    }
  });

  /// R-OB-1. One row per discovered book or section: what it is, what we think it is, and a field the
  /// student corrects. A row left blank is a source that stays unmapped — which is a choice, and is
  /// why the panel says what the consequence is rather than refusing Next.
  function renderMapping() {
    // A finished discovery that found nothing still has a note to show — only an UNfinished one
    // (nothing asked yet) has no panel to paint.
    EL("wiz-map").hidden = WIZ.map.length === 0 && !WIZ.discovered;
    EL("wiz-map-heading").hidden = WIZ.map.length === 0;
    EL("wiz-map-rows").innerHTML = WIZ.map.map(function (r, i) {
      // D6: a row nobody can guess for says so. Computed at paint, never on every keystroke —
      // re-rendering the rows under the cursor would take the focus out of the field being typed
      // into — so the hint goes on the next paint, which is what the student has already answered.
      var hint = (!r.suggested && !r.course) ? '<span class="meta">type the course this belongs to</span>' : "";
      return '<div class="wiz-row" data-map="' + i + '"><span class="meta">' + h(r.key) +
             (r.detail ? " &middot; " + h(r.detail) : "") + '</span>' +
             '<input type="text" list="wiz-course-codes" data-course-for="' + i + '" value="' + h(r.course || r.suggested || "") +
             '" placeholder="Course code, e.g. CS 100">' + hint +
             '<label><input type="checkbox" data-ignore-for="' + i + '"' + (r.ignore ? " checked" : "") + '> Ignore</label></div>';
    }).join("");
  }

  /// D6: leaving the logins panel with rows still blank is a choice, not a refusal
  /// (R-C1b-exec-10 already lets Next through) — but it has a consequence, and the panel says what
  /// it is: the engine files a coursework-map card for each one (R-OB-1) and the app asks about it
  /// there. Silent when nothing is blank; the singular reads correctly without a special case.
  ///
  /// Returns the count, so the caller can decide whether there is anything to stay for.
  function noteUnmapped() {
    var n = WIZ.map.filter(function (r) { return !r.ignore && !r.course; }).length;
    if (n) { EL("wiz-map-note").textContent = n + " of these will be asked about in the app"; }
    return n;
  }
  EL("wiz-map").addEventListener("input", function (e) {
    var f = e.target.getAttribute("data-course-for");
    if (f !== null) { WIZ.map[Number(f)].course = e.target.value.trim(); }
  });
  EL("wiz-map").addEventListener("change", function (e) {
    var g = e.target.getAttribute("data-ignore-for");
    if (g !== null) { WIZ.map[Number(g)].ignore = e.target.checked; renderMapping(); }
  });

  // The personal calendar: same command, same validation, a different kind — and a different sentence,
  // because "courses" means nothing about somebody's own week.
  EL("wiz-cal-ics").addEventListener("change", function () {
    WIZ.cal = EL("wiz-cal-ics").value.trim();
    if (!WIZ.cal) { WIZ.calNote = ""; renderWizard(); return; }
    var sent = WIZ.cal;
    invoke("paste_calendar_link", { kind: "calendar_ics", url: sent }).then(function (r) {
      WIZ.calFeed = r.ok ? { url: sent, stored: r.stored === true } : null;
      // Zero is a connection, not a failure: an address that fetches and holds nothing is somebody
      // who has not put anything in their calendar yet (`validate_for`).
      WIZ.calNote = !r.ok ? r.error
        : (r.link.events === 0 ? "Connected — nothing on it yet." : "Found " + r.link.events + " things already on your calendar.");
      if (r.ok && r.note) { WIZ.calNote += " " + r.note; }
      renderWizard();
    }).catch(function () {});
  });

  // The wizard's one place to show an out-of-band failure on the current panel — the same
  // `WIZ.error` / `renderWizard()` pair every other wizard error already uses.
  function showWizardError(msg) { WIZ.error = msg; renderWizard(); }

  // C2 (§11a): one Google connect for the calendars now, Gmail later and only if asked. The scope
  // named here is "calendar" — the *sensitive* one, which carries a lighter review and no CASA.
  // Nothing is written to the vault here: the wizard creates it at Finish, so this sets a flag on
  // the plan and `scaffold::ingest_yaml` writes the `calendars:` entry when the vault is born.
  //
  // A-5 (m59+m60): every field this poll touches lives on `WIZ` and is painted by `renderWizard()`
  // — never a direct DOM write — and `mySeq` is the cancellation check: `wizGo` bumps
  // `WIZ.googleSeq` on leaving this panel, and a poll whose own captured value no longer matches
  // stops touching the page at all, rather than writing a note onto a panel the student is no
  // longer looking at (or, worse, one a LATER click on this same button has since restarted).
  document.getElementById("wiz-google").addEventListener("click", async () => {
    var got = await invoke("google_connect_url", { scope: "calendar" });
    if (!got.ok) { showWizardError(got.error); return; }
    var opened = await invoke("open_external", { url: got.url });
    if (!opened.ok) { showWizardError(opened.error); return; }
    var mySeq = ++WIZ.googleSeq;
    WIZ.googlePolling = true;
    WIZ.googleNote = "Finish signing in to Google in your browser — this may take a moment.";
    renderWizard();
    // The consent window closes itself, so there is nothing else to tell us the round trip finished.
    // Twenty tries at three seconds is a minute, which is longer than a consent takes and shorter
    // than a student will sit staring at it; giving up is a message, never a silent stall.
    for (var i = 0; i < 20; i++) {
      await new Promise(function (r) { setTimeout(r, 3000); });
      if (WIZ.googleSeq !== mySeq) { return; }
      var status = await invoke("google_connected");
      if (WIZ.googleSeq !== mySeq) { return; }
      if (status.ok && status.calendar) {
        WIZ.google = true;
        WIZ.googlePolling = false;
        WIZ.googleNote = "Google Calendar connected — already on your calendar.";
        renderWizard();
        return;
      }
    }
    WIZ.googlePolling = false;
    WIZ.googleNote = "Google did not finish connecting. You can try again, or use the secret address above.";
    renderWizard();
  });

  // ---- R-OB-4: the school typeahead.
  //
  // **The page never holds the list and never fetches it.** `app/tauri.conf.json`'s CSP names the IPC
  // origin and nothing else — no `'self'` — so a `fetch` of a bundled asset is refused before it
  // reaches the asset protocol, and that file is the controller's outside C0's three keys, so widening
  // it would be a hand-off for a thing that needs none. `campus_search` is Rust's, answers with ten
  // rows, and the page holds ten rows however long the list gets.
  // R-C1-55, M6: one search per keystroke, and only the newest answer may paint — `renderSchoolHits`
  // overwrites `__hits`, which is what a click indexes into, so an overtaken answer would hand the
  // student a row that is not the one they clicked. `null` means "overtaken, do not paint".
  // `campus_search` is an in-process filter over a `OnceLock`ed list today, so this is insurance.
  function schoolHits(q) {
    var seq = (WIZ.schoolSeq += 1);
    return invoke("campus_search", { query: q })
      .then(function (r) { return seq === WIZ.schoolSeq ? ((r && r.hits) || []) : null; })
      .catch(function () { return seq === WIZ.schoolSeq ? [] : null; });
  }

  function renderSchoolHits(hits) {
    EL("wiz-school-hits").innerHTML = hits.map(function (r, i) {
      // [unitid, name, city, state]
      return '<div class="hit" data-school="' + i + '" tabindex="0">' + h(r[1]) +
             '<span class="meta"> &middot; ' + h(r[2]) + ", " + h(r[3]) + "</span></div>";
    }).join("");
    EL("wiz-school-hits").__hits = hits;
  }

  function pickSchool(r) {
    WIZ.campus = { unitid: String(r[0]), name: r[1], state: r[3], lms: "" };
    EL("wiz-school").value = r[1];
    EL("wiz-school-hits").innerHTML = "";
    EL("wiz-school-free").hidden = true;
    EL("wiz-school-picked").textContent = r[2] + ", " + r[3];
    WIZ.icsNote = "";
    // The timezone is a SUGGESTION from the state, and only into a field the student has not touched:
    // a typed value wins, and a state we do not know leaves the OS zone alone. Rust owns the table
    // (`scaffold::state_timezone`); the page only asks.
    invoke("timezone_for_state", { state: r[3] }).then(function (t) {
      if (t && t.ok && t.timezone && !WIZ.tzTouched) { WIZ.tz = t.timezone; EL("wiz-tz").value = t.timezone; }
    }).catch(function () {});
    renderWizard();
  }

  EL("wiz-school").addEventListener("input", function () {
    schoolHits(EL("wiz-school").value).then(function (hits) { if (hits) { renderSchoolHits(hits); } });
  });
  EL("wiz-school-hits").addEventListener("click", function (e) {
    var hit = e.target.closest("[data-school]");
    if (hit) { pickSchool(EL("wiz-school-hits").__hits[Number(hit.getAttribute("data-school"))]); }
  });
  // Keyboard-selectable: a typeahead you can only click is a typeahead that fails the person typing.
  EL("wiz-school-hits").addEventListener("keydown", function (e) {
    var hit = e.target.closest("[data-school]");
    if (hit && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      pickSchool(EL("wiz-school-hits").__hits[Number(hit.getAttribute("data-school"))]);
    }
  });
  EL("wiz-school").addEventListener("keydown", function (e) {
    if (e.key === "ArrowDown") {
      var first = EL("wiz-school-hits").querySelector("[data-school]");
      if (first) { e.preventDefault(); first.focus(); }
    }
  });
  // The bundled list is not all of them: a new campus, a satellite, somewhere abroad. A name and a
  // state is enough to make a vault, and that school simply has no curated feeds.
  EL("wiz-school-none").addEventListener("click", function () {
    EL("wiz-school-free").hidden = false;
    EL("wiz-school-hits").innerHTML = "";
  });
  EL("wiz-school-free").addEventListener("input", function () {
    WIZ.campus = { unitid: "", name: EL("wiz-school-name").value.trim(), state: EL("wiz-school-state").value.trim().toUpperCase(), lms: "" };
    EL("wiz-school-picked").textContent = "";
  });
  // The two-button fallback for a school whose LMS nothing established — shown by the sign-in handler
  // when `open_lms_window` says it does not know where to go.
  EL("wiz-lms-kind").addEventListener("click", function (e) {
    var b = e.target.closest("[data-lms]");
    if (b) { WIZ.campus.lms = b.getAttribute("data-lms"); EL("wiz-lms-kind").hidden = true; renderWizard(); }
  });
  // The slots panel, live: the summary on the finish panel is rendered from WIZ, so these are what
  // keep it true.
  ["wiz-tz", "wiz-slot1", "wiz-slot2"].forEach(function (id) {
    EL(id).addEventListener("input", function () { readSlotsPanel(); renderWizard(); });
  });
  // A suggestion must never overwrite an answer: once the student types a zone, picking a school
  // stops proposing one.
  EL("wiz-tz").addEventListener("input", function () { WIZ.tzTouched = true; });
  EL("wiz-autostart").addEventListener("change", function () { readSlotsPanel(); renderWizard(); });

  // The one seam the headless checks use (S13). Nothing in the app calls these; they exist so a
  // Playwright page can put the wizard, the picker or the settings overlay on screen without a
  // Tauri backend. `openSettings` is the console's own function, unchanged — with no backend its
  // two invokes reject and the rows keep their static markup, which is exactly the layout
  // `console-shots.py` is looking at; `settings-check.py` is the one that drives them with answers.
  window.KNOWLU_SHOTS = { startWizard: startWizard, renderPicker: renderPicker, openSettings: openSettings, openReport: openReport };

  invoke("launch_state", {}).then(function (l) {
    if (l && l.mode === "console") { bootConsole(); return; }
    if (l && l.mode === "wizard") { startWizard(l); return; }
    renderPicker(l || { profiles: [] });
  }).catch(function () { bootConsole(); });
})();
