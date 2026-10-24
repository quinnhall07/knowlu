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
      // Also dev-only: the grades strip's answer, so the shots can show each of its states. `?gs=`
      // names one of unavailable / notconnected / signedout / hidden; anything else is connected.
      if (fx && cmd === "grades_status") {
        var gs = new URLSearchParams(location.search).get("gs") || "";
        return Promise.resolve({ ok: true, available: gs !== "unavailable", reason: null, connected: gs !== "notconnected" && gs !== "unavailable", signed_out: gs === "signedout",
          host: "bb.example.edu", fetched_at: new Date(Date.now() - 7200000).toISOString(), hidden: gs === "hidden" });
      }
      return Promise.reject(new Error("no engine: __TAURI__ is absent"));
    }
    return window.__TAURI__.core.invoke(cmd, args || {});
  }

  // R-C1b-exec-9: the upgrade overlay (and, before F10, the wizard's subscribe panel) asks this same
  // question — is this account entitled right now — and each used to carry its own copy of the
  // status test. One copy, resolved `true` only on `ok` plus an active or trialing status, `false`
  // on anything else, a refusal or a dropped invoke included: nobody downstream of this has to
  // remember to `.catch`.
  // L8 S5 (spec 3.4): the same answer with the plan beside the status, `{ entitled, plan }`, and the
  // same fall-back — not entitled, no plan — on any failure. Every caller asks this one, so the plan
  // is known wherever the status is.
  function checkEntitlement() {
    return invoke("entitlement_now", {}).then(function (r) {
      var yes = !!(r && r.ok && (r.status === "active" || r.status === "trialing"));
      return { entitled: yes, plan: yes && r.plan ? String(r.plan) : "" };
    }).catch(function () { return { entitled: false, plan: "" }; });
  }
  function checkEntitled() {
    return checkEntitlement().then(function (r) { return r.entitled; });
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

  var pendingLaneRemovals = {};
  function renderTheDay(state) {
    var d = state.the_day;
    EL("dayopen").textContent = fmtH(d.open_hours) + " open";
    // P17: an accepted lane date carries its uid in `all_day_uids` (same index); a pending removal
    // is skipped while its Undo toast shows.
    var uids = d.all_day_uids || [];
    EL("allday").innerHTML = d.all_day.map(function (t, i) {
      var uid = uids[i];
      if (uid && pendingLaneRemovals[uid]) { return ""; }
      return "<div>(all day) " + h(t) + (uid ? ' <button class="b" type="button" data-lane-remove="' + h(uid) + '" data-title="' + h(t) + '">Remove from my day</button>' : "") + "</div>";
    }).join("");
    EL("theday").innerHTML = d.blocks.map(function (b) {
      var takes = b.takes.map(function (t) { return '<span class="sl"><span>' + h(t.title) + "</span><span>" + h(fmtH(t.hours)) + (t.of_hours - t.hours > 0.005 ? " of " + h(fmtH(t.of_hours)) : "") + "</span></span>"; }).join("");
      return '<div class="blk ' + h(b.kind) + '"><span class="t">' + h(b.start) + "–" + h(b.end) + '</span><span class="w">' + h(b.label) + '</span><span class="h">' + h(fmtH(b.hours)) + "</span>" + takes + "</div>";
    }).join("") + (d.empty_text ? '<div class="empty">' + h(d.empty_text) + "</div>" : "");
    EL("commitments").innerHTML = d.commitments.map(function (c) { return '<div class="commit"><span>' + h(c.name) + " · recurring</span><span>" + h(fmtH(c.hours)) + "</span></div>"; }).join("");
  }

  // Spec §6.3: an event card reads Accept and Decline; the verdicts sent are unchanged.
  function verdictLabels(kind) {
    if (kind === "event-accept" || kind === "event-check") { return { pri: "Accept", sec: "Decline" }; }
    return { pri: "Approve", sec: "Reject" };
  }

  // Gmail spec D15: a card from Gmail says so (its source_uid prefix, the same key the task chip
  // reads) beside the model's reason; never the sender, which the card does not carry.
  function sourceLabel(c) {
    return c.source_uid && String(c.source_uid).indexOf("gmail:") === 0 ? ' <span class="src">from Gmail</span>' : "";
  }

  // Email spec D21, §5.2 (T15.5): a mail card names its sender and the mail's day and quotes the evidence. All three are
  // the sender's own text, untrusted: each goes through h() and nowhere else, and none is ever sent as a usage event.
  function mailCardLine(c) {
    if (!c.from && !c.mail_date && !c.evidence) { return ""; }
    var who = [c.from ? h(c.from) : "", c.mail_date ? h(c.mail_date) : ""].filter(function (t) { return t; }).join(" · ");
    return (who ? '<div class="mail-from">' + who + "</div>" : "") + (c.evidence ? '<div class="mail-quote"><q>' + h(c.evidence) + "</q></div>" : "");
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
      var approve = c.kind === "commitment-ask" ? '<button class="b pri y" type="button" data-answer-in>Answer&hellip;</button>' : '<button class="b pri y" data-verdict="approved">' + verdictLabels(c.kind).pri + '</button>';
      deck.innerHTML += '<div class="card" data-id="' + h(c.id) + '" data-kind="approval"><button class="flag" data-flag="' + h(c.id) + '" title="agent-authored — flag it">&#9873;</button><div class="t">' + h(c.title) + '</div><div class="w">' + h(c.why || c.kind) + sourceLabel(c) + "</div>" + mailCardLine(c) +
        '<div class="nb"><input type="text" placeholder="Note (optional)"></div><div class="acts">' + approve + '<button class="b n" data-verdict="rejected">' + verdictLabels(c.kind).sec + '</button><button class="b" data-verdict="snoozed">&hellip;</button><input type="date" hidden value="' + h(tomorrow) + '"></div></div>';
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
    EL("cu").innerHTML = cu.length ? cu.map(function (e) { return '<div class="ln"><span class="k lp">' + h(e.when) + "</span><span>" + h(e.title) + (e.location ? " · " + h(e.location) : "") + (e.provenance ? ' <span class="k lp">' + h(e.provenance) + "</span>" : (e.accepted ? ' <span class="k lp">' + h("Accepted") + "</span>" : "")) + '</span><span class="rt">' + h(e.organizer) + "</span></div>"; }).join("") : '<div class="empty">' + h(state.texts.coming_up) + "</div>";
    renderNotShown();
  }

  // M2 T7: Not shown (N) under Coming up, read-only, from dropped_events (events the engine dropped, each with its reason).
  function renderNotShown() {
    var box = EL("cu-ns");
    return invoke("dropped_events", {}).then(function (r) {
      var list = (r && r.ok && r.dropped) || null;
      if (!r || !r.ok) { box.innerHTML = '<div class="empty">Not shown is unavailable right now.</div>'; return; }
      if (!list || !list.length) { box.innerHTML = ""; return; }
      box.innerHTML = '<details class="notshown"><summary>Not shown (' + list.length + ")</summary>" + list.map(function (d) {
        return '<div class="ln"><span class="k lp">' + h(d.date) + "</span><span>" + h(d.title) + '</span><span class="rt">' + h(d.reason) + "</span></div>";
      }).join("") + "</details>";
    }).catch(function () { box.innerHTML = '<div class="empty">Not shown is unavailable right now.</div>'; });
  }
  // M2 T7 end

  function renderGoodToKnow(state) {
    var g = state.good_to_know;
    EL("gtk-n").textContent = g.length ? g.length + " open" : "";
    EL("gtk").innerHTML = g.length ? g.map(function (i) { return '<div class="ln"><span class="k lp">' + h(i.kind) + "</span><span>" + h(i.title) + '</span><span class="rt">' + h(i.expires || "") + '</span><button class="b" data-close-info="' + h(i.id) + '">close</button></div>'; }).join("") : '<div class="empty">' + h(state.texts.info) + "</div>";
  }

  function renderClosed(state) {
    var c = state.closed_this_week;
    EL("closed-n").textContent = c.length;
    EL("closed").innerHTML = c.length ? c.map(function (x) { var k = x.mark === "me" ? "me" : x.mark === "agent" ? "ag" : "lp"; return '<div class="ln' + (x.undo && x.id ? " undo" : "") + '"><span class="k ' + k + '">' + h(x.mark === "me" ? "you" : x.mark) + "</span><span>" + h(x.title) + '</span><span class="rt">' + h(x.when.slice(11)) + "</span>" + (x.undo && x.id ? '<button class="b" type="button" data-undo-close="' + h(x.id) + '">Undo</button>' : "") + "</div>"; }).join("") : '<div class="empty">' + h(state.texts.closed) + "</div>";
  }

  /// O1 T7b: Undo on a close the engine marked `undo`. It is the student's own `set` back to active
  /// (the same `set_fields` the drawer uses); which rows may offer it is the engine's call.
  function undoClose(id, btn) {
    if (btn) { btn.disabled = true; }
    return invoke("set_fields", { view: stateView(), id: id, fields: { status: "active" } }).then(function (env) {
      var ok = applyEnvelope(env, function (msg) { if (btn) { btn.disabled = false; } showRefusal(null, "could not undo — " + msg, id); });
      if (ok) { showNote("put back on your list"); }
    }).catch(function (err) {
      if (btn) { btn.disabled = false; }
      showRefusal(null, "could not undo — " + reasonOf(err), id);
    });
  }

  // Plan 4a Task 8: the offer. Rendered only when the engine says there is one — the mid-run
  // gate is `updates::update_offer` in Rust, never a check here (R9).
  function renderUpdateOffer(u, quiet) {
    var el = EL("upd");
    current.version = u.version;
    // M8: version, last check, staged-or-not, error — and no release track is named, because
    // there is only one. (The word the settings test forbids in this file is the one M8 rules out.)
    // `u.held` is a bundle the RUN is holding back (R-P4a-24): downloaded, verified, and
    // deliberately not offered until the slot ends. Saying "up to date" there would be a lie about
    // a file already sitting on this machine — so it gets its own words, and the offer stays away.
    var when = localHM(u.last_check) ? ", checked " + localHM(u.last_check) : "";
    var what = u.staged ? " — " + u.staged + " ready"
      : u.held ? " — " + u.held + " staged; installs after the run"
      : u.last_error ? " — " + u.last_error.split("\n")[0]
      : " — up to date";
    // The last ACTION's outcome rides as its own suffix, never folded into `what` (fix round 1,
    // IMPORTANT 2): a tray refusal or a failed install has to stay legible next to "up to date",
    // which is a true statement about the CHECK and says nothing about the install that failed.
    var act = u.action_error ? " · last action: " + u.action_error.split("\n")[0] : "";
    current.updateText = "version " + u.version + what + when + act;
    // Mid-install the banner belongs to `installUpdate` ("installing…" or its failure); a check that
    // lands now must not put a fresh, pressable button over it.
    // A quiet (automatic) check also leaves a banner that is showing an install message alone: the
    // slot-running wait, a failure, "restart Knowlu" — hiding it would lose the offer until a restart.
    if (UPD.installing || (quiet && UPD.note)) { return; }
    UPD.note = false;
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
  // The updater's page-side state: an install is in flight (`installing`), was ever pressed
  // (`attempted`, which ends the automatic retries), the banner shows an install message (`note`),
  // and how many times a check that lost the single flight has asked again, on one shared `timer`.
  var UPD = { installing: false, attempted: false, note: false, retries: 0, timer: null };
  var ALREADY_CHECKING = "a check is already running";
  /// A local HH:MM from an ISO stamp (the engine and the updater write UTC, `...Z`); "" if unparseable.
  function localHM(iso) {
    var d = new Date(iso);
    if (!iso || isNaN(d.getTime())) { return ""; }
    return ("0" + d.getHours()).slice(-2) + ":" + ("0" + d.getMinutes()).slice(-2);
  }
  /// A calm toast: what happened, when nothing went wrong. Errors keep `showRefusal`.
  function showNote(message) {
    var n = document.createElement("div"); n.className = "notetoast"; n.textContent = message;
    document.body.appendChild(n);
    setTimeout(function () { n.remove(); }, 6000);
  }
  function reasonOf(err) { return String((err && err.message) || err || "unknown").split("\n")[0]; }
  function setUpdateState(text) { var el = EL("set-update-state"); if (el) { el.textContent = text; } }
  /// The Updates row's line for one finished check — every outcome says something. Never a decision:
  /// `u` is the command's own answer (`staged` is already gated on a running slot in Rust).
  function updateOutcome(u, retrying) {
    if (u.error) { return u.error === ALREADY_CHECKING ? (retrying ? "a check is already running — the result will show here" : "a check is already running — try again shortly") : "could not check — " + String(u.error).split("\n")[0]; }
    if (u.staged) { return u.staged + " is available"; }
    if (u.held) { return u.held + " is staged; installs after the run"; }
    return "up to date (checked " + (localHM(u.last_check) || localHM(new Date().toISOString())) + ")";
  }
  /// `manual` is the Updates row's button: it says "checking…" and then the outcome. The quiet form
  /// (launch, and the retry below) only fills the row if the panel is open. Either way a staged
  /// update raises the same banner (`renderUpdateOffer`). The startup check and the housekeeping
  /// thread share one flight, and the loser used to say nothing and never ask again — the banner then
  /// waited for a restart — so it asks again in 10 s, up to six times, on ONE timer. Every check
  /// clears the server's action error (`check_for_updates`), which background work must not do to an
  /// install's outcome: so no retry once an install was pressed or while the banner shows a message.
  function checkForUpdates(manual) {
    var btn = EL("set-update-check");
    clearTimeout(UPD.timer);
    if (manual === true) { UPD.retries = 0; btn.disabled = true; setUpdateState("checking…"); }
    return invoke("check_for_updates", {}).then(function (u) {
      renderUpdateOffer(u, manual !== true);
      // Whether a retry will follow decides what the row may promise: after an install attempt none does.
      var retry = u.error === ALREADY_CHECKING && UPD.retries < 6 && !UPD.attempted && !UPD.note;
      if (manual === true) { setUpdateState(updateOutcome(u, retry)); }
      else if (!EL("settings").hidden) { setUpdateState(current.updateText); }
      if (retry) {
        UPD.retries++;
        clearTimeout(UPD.timer);
        UPD.timer = setTimeout(function () { if (!UPD.attempted && !UPD.note && !UPD.installing) { checkForUpdates(); } }, 10000);
      }
    }).catch(function (err) {
      if (manual === true) { setUpdateState("could not check — " + reasonOf(err)); }
    }).then(function () { if (manual === true) { btn.disabled = false; } });
  }
  /// *Restart to update*: disabled and "installing…" on the first press. A slot running (it often is,
  /// just after launch) is not a failure: the banner says it is waiting for the current sync to finish
  /// and the install is retried every 5 s for up to 5 minutes. Any other refusal stays ON the banner
  /// with a way to try again, not only in a six-second toast. On Windows a good install exits the
  /// process; where it does not, the banner says to restart.
  function installUpdate(btn) {
    if (UPD.installing) { return; }
    var el = EL("upd"), waits = 0;
    function say(html) { el.innerHTML = html; el.hidden = false; UPD.note = true; }
    function fail(msg) {
      UPD.installing = false;
      say('<span class="amber">update failed — ' + h(msg) + '</span> <button class="b pri y" type="button" data-install>Try again</button>');
      showRefusal(null, "update failed — " + msg);
    }
    function attempt() {
      invoke("install_update", {}).then(function (r) {
        if (r.ok) {
          setTimeout(function () { UPD.installing = false; say('<span class="amber">restart Knowlu to finish the update</span>'); }, 15000);
          return;
        }
        var msg = reasonOf(r.error || "refused");
        if (msg.indexOf("a slot is running") === 0 && waits < 60) {
          if (waits === 0) { say('<span class="amber">waiting for the current sync to finish — Knowlu will install the update when it ends</span>'); }
          waits++;
          setTimeout(attempt, 5000);
          return;
        }
        fail(msg);
      }).catch(function (err) { fail(reasonOf(err)); });
    }
    UPD.attempted = true; UPD.installing = true; UPD.note = false;
    clearTimeout(UPD.timer);
    btn.disabled = true; btn.textContent = "installing…";
    attempt();
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
  // *Sync now* can take ~30 s and a poll repaint rebuilds the button while it runs, so "busy" is a flag
  // `renderSyncLine` reads as well as a state the button is put into.
  var syncing = false;
  function setSyncBusy(on) {
    syncing = on;
    Array.prototype.forEach.call(document.querySelectorAll("[data-sync]"), function (b) {
      b.disabled = on || RECONSENT_OWED; b.textContent = on ? "syncing…" : "sync now";
    });
  }
  /// Every outcome says something: the time on a clean sync, the sync line's own words on a skip, the
  /// refusal on an error, and a rejected call its reason — never an empty catch.
  function runSync() {
    if (syncing) { return; }
    setSyncBusy(true);
    return invoke("sync", { view: stateView() }).then(function (env) {
      setSyncBusy(false);
      var ok = applyEnvelope(env, syncRefusal);
      ev("sync_run", null, null, null);
      if (!ok) { return; }
      var s = (env.state && env.state.topline && env.state.topline.sync) || {};
      if (s.skipped) { showNote(syncSays(s.skipped, "calm", "sync skipped — ")[1]); }
      else { showNote("synced " + (localHM(s.at) || localHM(new Date().toISOString()))); }
    }).catch(function (err) {
      setSyncBusy(false);
      showRefusal(null, "sync could not start — " + reasonOf(err));
    });
  }
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
    // `at` is stamped on every run, so a skip or an error says "tried", never "synced".
    if (s.at && localHM(s.at)) { bits.push('<span class="calm">' + (s.last_error || s.skipped ? "tried " : "synced ") + localHM(s.at) + "</span>"); }
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
    EL("syncline").innerHTML = bits.join(" · ") + ' <button class="b" type="button" data-sync' + owedAttrs() + (syncing ? " disabled" : "") + '>' + (syncing ? "syncing…" : "sync now") + '</button> <button class="b" type="button" data-backup>back up now</button>';
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
          '<div class="why">' + h(c.why) + sourceLabel(c) + "</div>" + mailCardLine(c) + changes + "</div>" +
          '<div class="acts"><input type="text" class="dnote" placeholder="Note (optional)" value="' + h(notes[c.id] || "") + '">' +
          '<button class="b pri y" data-verdict="approved">' + verdictLabels(c.kind).pri + '</button><button class="b n" data-verdict="rejected">' + verdictLabels(c.kind).sec + '</button>' +
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
      // L7 (spec 3.6): sources in trouble come first. The engine's words, shown as given; the row has no
      // Resolve, flag or Close, because the heads-up clears itself when the source is well again.
      var srcs = (p.sources || []).map(function (s) {
        return '<div class="row iss src" data-id="' + h(s.id) + '" data-kind="info">' +
          '<div class="ttl"><span class="a">' + h(s.title) + '</span>' +
          '<span class="meta">' + h(s.since ? "since " + s.since.slice(0, 10) : "") + '</span>' +
          '<div class="why">' + h(s.text) + "</div></div></div>";
      }).join("");
      host.innerHTML = srcs + p.rows.map(function (r) {
        var chips = (r.categories || []).map(function (c) { return '<span class="chip on">' + h(c) + "</span>"; }).join("");
        var when = r.opened_at ? "raised " + r.opened_at.slice(0, 10) : "";
        return '<div class="row iss" data-id="' + h(r.id) + '" data-kind="issue">' +
          '<div class="ttl"><span class="a">' + h(r.title) + '</span><span class="meta">' + h(when) + "</span><div class=\"chips\">" + chips + "</div>" +
          (r.target ? '<button class="lnk" data-open-target="' + h(r.target) + '">open the object</button>' : "") + "</div>" +
          '<div class="acts"><input type="text" class="dnote" placeholder="Resolution"><button class="b" data-resolve="' + h(r.id) + '">Resolve</button></div></div>';
      }).join("") + (p.rows.length || srcs ? "" : '<div class="empty">' + h(p.empty_text) + "</div>");
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
      registrarLabel = r.registrar_label || null; renderRegistrar(w);
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

  // ---- Phase 3 (spec D1, D6, §2): Get my class times from myBama. The page opens the school's
  // sign-in window and, once the student says they are signed in, asks the app to read the
  // schedule. The page never sees a cookie or the fetched bytes: `capture_registrar` keeps both.
  var registrarLabel = null;   // your_week's registrar_label; null where the school has none
  // B5, m3 and m4: a fetch that reached the engine and failed shows this sentence and nothing the
  // engine said. The school window is gone by then (R4-d), so a retry starts from a fresh sign-in.
  var REG_FAILED = "Knowlu couldn't use what the school sent, so nothing was changed. The sign-in window is closed; to try again, press the button and sign in again.";

  // R5-a: a row the registrar proposed.
  function fromRegistrar(p) { return p.source_uid.indexOf("registrar:") === 0; }

  function registrarControls(host) {
    return { open: host.querySelector("[data-reg-open]"), done: host.querySelector("[data-reg-done]"),
      cancel: host.querySelector("[data-reg-cancel]"), say: host.querySelector("[data-reg-say]") };
  }

  function registrarIdle(host, message) {
    var c = registrarControls(host);
    c.open.hidden = false; c.done.hidden = true; c.cancel.hidden = true; c.done.disabled = false;
    c.say.textContent = message || "";
  }

  // `after(env)` runs once a fetch has reached the engine: the confirm screen re-reads its rows;
  // Schedule repaints (paint already ran its renderer when a state came back). `busy(on)`, if
  // given, holds the host's screen from I'm signed in until the fetch and `after` settle, on every
  // path (R5 review I1: a Finish mid-fetch wrote the rows from before the reload).
  function bindRegistrar(host, after, busy) {
    var setBusy = busy || function () {};
    host.addEventListener("click", function (e) {
      e.stopPropagation();
      var c = registrarControls(host);
      if (e.target.closest("[data-reg-open]")) {
        invoke("open_registrar_window", {}).then(function (r) {
          if (!r || !r.ok) { registrarIdle(host, r && r.error); return; }
          c.open.hidden = true; c.done.hidden = false; c.cancel.hidden = false;
          c.say.textContent = "Sign in to " + registrarLabel + " in the window Knowlu opened, then press I'm signed in.";
        }).catch(function () { registrarIdle(host, "Knowlu couldn't open " + registrarLabel + "; try again."); });
      } else if (e.target.closest("[data-reg-done]")) {
        c.done.disabled = true; c.cancel.hidden = true; setBusy(true);
        c.say.textContent = "Reading your class schedule…";
        invoke("capture_registrar", { view: stateView() }).then(function (env) {
          c.done.disabled = false;
          if (!env.ok) {
            setBusy(false);
            if (env.closed) { registrarIdle(host, REG_FAILED); } else { c.cancel.hidden = false; c.say.textContent = env.error; }
            return;
          }
          if (env.state) { current.pendingOrder = null; paint(env.state, true); }
          var r = env.result || {};
          registrarIdle(host, "Found " + r.rows + (r.rows === 1 ? " class" : " classes") + " in " + registrarLabel + ": " + r.confirmed + " confirmed, " + r.proposed + " to check.");
          return Promise.resolve(after(env)).then(function () { setBusy(false); }, function () { setBusy(false); });
        }).catch(function () { setBusy(false); registrarIdle(host, "Knowlu couldn't read " + registrarLabel + "; try again."); });
      } else if (e.target.closest("[data-reg-cancel]")) {
        invoke("close_registrar_window", {}).catch(function () {});
        registrarIdle(host, "");
      }
    });
  }

  // R5-c: Schedule's registrar section — the label by term, the primary style only when a term the
  // file does not hold has begun, and the registrar's proposals with Add.
  function renderRegistrar(w) {
    var sec = EL("sched-reg-sec");
    sec.hidden = !registrarLabel;
    if (!registrarLabel) { return; }
    var reg = w.registrar, open = EL("sched-reg").querySelector("[data-reg-open]");
    open.textContent = (reg && reg.held && reg.held.length ? "Refresh from " : "Get my class times from ") + registrarLabel;
    open.classList.toggle("pri", !!(reg && reg.refresh));
    open.classList.toggle("y", !!(reg && reg.refresh));
    EL("sched-reg-rows").innerHTML = (w.registrar_proposals || []).map(function (p) {
      return '<div class="row sched reg-row"><div class="ttl"><span class="a">' + h(p.title) + '</span><span class="meta">' + h(p.when || "") +
        ' <span class="src">from ' + h(registrarLabel) + '</span></span></div><div class="acts"><button class="b" type="button" data-reg-add="' +
        h(p.source_uid) + '">Add</button></div></div>';
    }).join("");
    var list = registrarChangesHtml(w.registrar_changes, w.commitments), box = EL("sched-reg-changes");
    box.hidden = !list; box.innerHTML = list;
  }

  // B5 (spec A2 item 13): the "From myBama" lists that follow a fetch and a restart, read from the
  // overview's `registrar_changes`. The page names a note by its title (the block holds its path),
  // words an unread row itself, and shows each value escaped; it works out nothing.
  var REG_FIELDS = { meets: "meeting times", where: "room", until: "last day", from: "first day" };

  function regMeets(list) {
    return list.map(function (m) {
      return m.days.map(function (d) { return d.charAt(0).toUpperCase() + d.slice(1, 3); }).join("/") + " " + m.start + "–" + m.end;
    }).join(", ");
  }

  function regValue(v) { return v == null || v === "" ? "nothing" : Array.isArray(v) ? regMeets(v) : v; }

  // Returns markup: the title is escaped here, once.
  function regTitle(note, key, commitments) {
    var c = (commitments || []).filter(function (x) { return x.path === note; })[0];
    return c ? h(c.title) : (key ? "Class " + h(String(key).split("-").pop()) : "A class");
  }

  function registrarChangesHtml(ch, commitments) {
    if (!ch || !registrarLabel) { return ""; }
    var field = function (f) { return REG_FIELDS[f] || f; };
    var CARDS = { filed: "Knowlu has asked you in Decisions which to keep.", waiting: "Knowlu will ask you in Decisions which to keep.", listed: "Listed here only." };
    var parts = [
      // `e["from"]`: the entry's old value, not the mail sender the deck scan looks for.
      [" — changed", ch.applied, function (e) {
        return regTitle(e.note, null, commitments) + ": " + h(field(e.field)) + " " + h(regValue(e["from"])) + " → " + h(regValue(e.to));
      }],
      [" — ended", ch.ended, function (e) {
        return regTitle(e.note, null, commitments) + " ended on " + h(e.until) + (e.kept_yours
          ? ' <span class="reg-card">You had set a different last day, so Knowlu kept yours.</span>' : "");
      }],
      [" — kept yours", ch.kept, function (e) {
        return regTitle(e.note, null, commitments) + ": your " + h(field(e.field)) + " is " + h(regValue(e.student)) + "; " + h(registrarLabel) +
          " has " + h(regValue(e.registrar)) + '. <span class="reg-card">' + h(CARDS[e.card] || CARDS.listed) + "</span>";
      }],
      [" — status", ch.status, function (e) {
        return regTitle(e.note, e.key, commitments) + ": " + h(registrarLabel) + " lists it as " + h(e.status) + "; nothing changed.";
      }],
      [" — not read", ch.unread, function (e) {
        var what = String(e.reason || "").indexOf("no class time") >= 0 ? "no class time" : "a meeting Knowlu can't read";
        return regTitle(e.note, e.key, commitments) + ": " + h(registrarLabel) + " lists it with " + h(what) + "; nothing changed.";
      }]];
    return parts.filter(function (p) { return (p[1] || []).length; }).map(function (p) {
      return '<p class="hint">From ' + h(registrarLabel) + p[0] + "</p><ul>" + p[1].map(function (e) { return "<li>" + p[2](e) + "</li>"; }).join("") + "</ul>";
    }).join("");
  }

  // R5-b: after a fetch the confirm screen reads its rows again, and the answers and levels already
  // given are put back by key.
  function reloadSetupRows() {
    var kept = {};
    document.querySelectorAll("#week-setup .wsrow[data-key]").forEach(function (r) {
      kept[r.getAttribute("data-key")] = { answer: r.getAttribute("data-answer"), level: r.getAttribute("data-level") };
    });
    return invoke("commitment_proposals", {}).then(function (r) {
      paintSetupRows((r && r.proposals) || [], (r && r.uncovered_courses) || []);
      document.querySelectorAll("#week-setup .wsrow[data-key]").forEach(function (row) {
        var k = kept[row.getAttribute("data-key")]; if (!k) { return; }
        row.setAttribute("data-answer", k.answer); row.setAttribute("data-level", k.level);
        row.querySelectorAll("[data-answer-set]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.getAttribute("data-answer-set") === k.answer)); });
        row.querySelectorAll("[data-level-set]").forEach(function (b) { b.setAttribute("aria-pressed", String(b.getAttribute("data-level-set") === k.level)); });
      });
      return invoke("your_week", {}).then(function (w) {
        paintRegistrarAdded(w && w.ok && w.week ? w.week.commitments : [], w && w.ok && w.week ? w.week.registrar_changes : null);
      });
    }).catch(function () {});
  }

  // Final review I3: the confirm screen lists what myBama confirmed (spec §2 step 4) — your_week's
  // confirmed rows keyed `registrar:`, as the engine built them: title, then days and times.
  function paintRegistrarAdded(commitments, changes) {
    var added = (commitments || []).filter(function (c) { return c.source_uid && c.source_uid.indexOf("registrar:") === 0; });
    var host = EL("ws-reg-added"), list = registrarChangesHtml(changes, commitments);
    host.hidden = (!added.length && !list) || !registrarLabel;
    host.innerHTML = host.hidden ? "" : (added.length ? '<p class="hint">From ' + h(registrarLabel) + " — added</p><ul>" + added.map(function (c) {
      return '<li><span class="a">' + h(c.title) + '</span> <span class="meta">' + h(c.when || "") + "</span></li>";
    }).join("") + "</ul>" : "") + list;
  }

  // ---- Phase 2 (spec §2, D1, D3): the confirm screen, over the first-run view. `your_week` says
  // `setup` while the vault is on its first day and has no planning-day note. Not now hides the
  // screen for this console session; Finish writes the planning-day note, so it never returns.
  var weekSetup = { dismissed: false, open: false, dirty: false, reading: false, fetching: false, finishing: false };

  // R5 re-review: the one answer to "may Finish be pressed?" — nothing else is in flight: not the
  // first calendar read, not a myBama fetch, not a Finish. Every path that re-enables Finish asks.
  function finishMayRun() {
    return !weekSetup.reading && !weekSetup.fetching && !weekSetup.finishing;
  }

  // A Finish in flight holds the myBama controls too, so no capture starts under it.
  function setupFinishing(on) {
    weekSetup.finishing = on;
    EL("ws-reg").querySelectorAll("[data-reg-open], [data-reg-done]").forEach(function (b) { b.disabled = on; });
    EL("ws-finish").disabled = !finishMayRun();
    EL("ws-later").disabled = !finishMayRun();   // final review m1: Not now waits as Finish does
  }

  // R5 review I1: a myBama fetch holds Finish and Not now until it and the reload settle. Finish
  // stays held past it while the first calendar read is still out (phase 2's final review I1).
  function setupBusy(on) {
    weekSetup.fetching = on;
    EL("ws-finish").disabled = !finishMayRun();
    EL("ws-later").disabled = !finishMayRun();
  }

  function checkWeekSetup() {
    if (weekSetup.dismissed || weekSetup.open) { return; }
    invoke("your_week", {}).then(function (r) {
      if (r) { registrarLabel = r.registrar_label || null; }
      if (r && r.ok && r.week && r.week.setup && !weekSetup.dismissed) { openWeekSetup(r.week); }
    }).catch(function () {});
  }

  // One row: title, the §5.2 label, `where` in small type; Mine / Not mine; the level control,
  // shown by CSS only on a Mine row. Nothing typed.
  function setupRow(p) {
    var preset = { class: 1, lab: 1, work: 1 }[p.kind] ? "mine" : "";
    return '<div class="wsrow" data-key="' + h(p.source_uid) + '" data-answer="' + preset + '" data-level="' + h(p.level) + '">' +
      '<div class="ttl"><span class="a">' + h(p.title) + '</span><span class="meta">' + h(p.when || "") + "</span>" +
      (fromRegistrar(p) && registrarLabel ? ' <span class="src">from ' + h(registrarLabel) + "</span>" : "") +
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
    EL("ws-classes").hidden = !classes.length && !uncovered.length && !registrarLabel;
    EL("ws-reg").hidden = !registrarLabel;
    if (registrarLabel) { EL("ws-reg").querySelector("[data-reg-open]").textContent = "Get my class times from " + registrarLabel; }
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
    EL("ws-finish").disabled = true; weekSetup.reading = true;
    EL("ws-later").disabled = true;   // final review m1
    var ready = function () { weekSetup.reading = false; EL("ws-later").disabled = !finishMayRun(); if (finishMayRun()) { EL("ws-finish").disabled = false; } };
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

  // Final review m2: the school's window goes with the screen (its controls do), wiped as on Cancel.
  function closeWeekSetup() {
    weekSetup.open = false; EL("week-setup").hidden = true;
    invoke("close_registrar_window", {}).catch(function () {});
  }

  function finishWeekSetup() {
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
    setupFinishing(true);
    confirmWeek(payload).then(function (env) {
      setupFinishing(false);
      if (!env.ok) { showWindowError(EL("ws-window"), env.error); EL("ws-status").textContent = env.error; return; }
      closeWeekSetup();
      // Final review M4: a row --confirm skipped (no longer a current proposal) comes back as a
      // question; say so on the notice line until the next paint, rather than close in silence.
      var skipped = (env.result && env.result.warnings) || [];
      if (skipped.length) {
        EL("delta").textContent = skipped.length + (skipped.length === 1 ? " row" : " rows") + " will come back as questions: " + skipped.join("; ");
      }
    }).catch(function () { setupFinishing(false); });
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
    if (e.target.closest("#ws-later")) { if (finishMayRun()) { weekSetup.dismissed = true; closeWeekSetup(); } return; }
    var fin = e.target.closest("#ws-finish");
    if (fin) { finishWeekSetup(); }
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

  // ---- M1 grades (grades spec §2 and §9). The strip on Today, one ring per course, and the read-only
  // breakdown in the existing drawer. Nothing is computed here: every number is `state.grades`'s, and
  // which of the states shows is `grades_status`'s word alone. The console has no address entry and
  // never decides whether a school offers grades. A signed-out session is a named state, in the
  // page's own quiet voice: never amber, never an error.
  var GRADES = { status: null, list: [], note: "" };
  var GRADE_STATUS_WORDS = { "graded": "graded", "needs-grading": "needs grading", "in-progress": "in progress", "not-submitted": "not submitted", "exempt": "exempt" };

  function renderGrades(state) {
    GRADES.list = (state && state.grades) || [];
    return gradesStatus().then(refreshGradesDrawer);
  }
  function gradesStatus() {
    return invoke("grades_status", {}).then(function (s) { GRADES.status = s; drawGrades(); }).catch(function () { GRADES.status = null; drawGrades(); });
  }
  function gradesUnavailableHtml() {
    return '<div class="gs-line"><span>Grades from Blackboard are not available at your school yet</span></div>';
  }
  function ringSvg(c) {
    var fam = c.family ? String(c.family).toLowerCase() : "";
    var ring = '<svg class="ring" viewBox="0 0 36 36" aria-hidden="true"><circle class="rt" cx="18" cy="18" r="15.9155" fill="none"></circle>';
    if (c.pct != null) {
      // r = 100 / 2π, so the circle's length is 100 and the dash is the percentage itself.
      ring += '<circle class="rf' + (/^[abcdf]$/.test(fam) ? " grade-" + fam : "") + '" cx="18" cy="18" r="15.9155" fill="none" stroke-dasharray="' + Math.max(0, Math.min(100, c.pct)) + ' 100" transform="rotate(-90 18 18)"></circle>';
    }
    return ring + "</svg>";
  }
  function drawGrades() {
    var el = EL("grades"), s = GRADES.status, html;
    if (!el) { return; }
    // The settings panel's Hide grades box shows the student's own flag, whatever the strip shows.
    if (s && s.ok !== false) { EL("set-grades-hide-in").checked = !!s.hidden; }
    if (!s || s.ok === false || s.hidden) { el.hidden = true; el.innerHTML = ""; return; }
    html = '<div class="sec-hd"><h2>Grades</h2></div>';
    if (s.available === false) {
      html += gradesUnavailableHtml();
    } else {
      if (GRADES.list.length) {
        html += '<div class="gs-rings">' + GRADES.list.map(function (c) {
          return '<button class="gring" type="button" data-grades-course="' + h(c.course) + '" title="' + h(c.title) + '">' + ringSvg(c)
            + '<span class="gt"><b>' + h(c.title) + "</b><span>" + (c.pct == null ? "no grades yet" : h(c.pct) + "% " + h(c.letter || "")) + "</span></span></button>";
        }).join("") + "</div>";
      }
      if (!s.connected) {
        html += '<div class="gs-line"><span>Connect Blackboard to see your grades</span><button class="b pri y" type="button" data-grades-connect' + owedAttrs() + '>Connect Blackboard</button></div>';
      } else if (s.signed_out) {
        html += '<div class="gs-line"><span>Sign in to Blackboard again to update grades</span><button class="b pri y" type="button" data-grades-connect' + owedAttrs() + '>Sign in again</button></div>';
      } else if (!GRADES.list.length) {
        html += '<div class="gs-line"><span>Blackboard is connected. Your grades appear after the next refresh.</span><button class="b" type="button" data-grades-refresh' + owedAttrs() + '>Refresh</button></div>';
      }
    }
    if (RECONSENT_OWED && /data-grades-(connect|refresh)/.test(html)) { html += '<p class="hint gs-note">' + h(OWED_LINE) + "</p>"; }
    if (GRADES.note) { html += '<p class="hint gs-note">' + h(GRADES.note) + "</p>"; }
    el.innerHTML = html; el.hidden = false;
  }
  function agoText(iso) {
    var t = Date.parse(iso), m;
    if (isNaN(t)) { return ""; }
    m = Math.floor((Date.now() - t) / 60000);
    return m < 1 ? "just now" : m < 60 ? m + " min ago" : m < 1440 ? Math.floor(m / 60) + " h ago" : Math.floor(m / 1440) + " d ago";
  }
  function gradesFooterHtml(c) {
    var pts = c.earned + " / " + c.possible, src;
    if (c.basis === "overall") { src = "Overall Grade" + (c.possible > 0 ? " · points so far " + pts : ""); }
    else if (c.basis === "points") { src = "points so far · " + pts; }
    else { src = "nothing graded yet"; }
    return '<div class="gd-foot"><span>From Blackboard · ' + h(src) + (c.fetched_at ? " · updated " + h(agoText(c.fetched_at)) : "")
      + '</span><button class="b" type="button" data-grades-refresh' + owedAttrs() + '>Refresh</button></div><p class="hint gs-note" id="gd-note">' + h(GRADES.note) + "</p>";
  }
  function gradeRowHtml(e) {
    var score = (e.score == null ? "–" : e.score) + " / " + (e.possible == null ? "–" : e.possible);
    return '<div class="grw"><span class="gn">' + h(e.title) + (e.counts === false ? ' <small>not counted</small>' : "") + "</span>"
      + '<span class="gsc">' + h(score) + '</span><span class="gst st-' + h(e.status) + '">' + h(GRADE_STATUS_WORDS[e.status] || e.status) + "</span>"
      + (e.due ? '<span class="gdue">' + h(String(e.due).slice(0, 10)) + "</span>" : "") + "</div>";
  }
  // Read-only: the breakdown never edits, creates or deletes a note.
  function openGradesDrawer(slug) {
    dropSavedToast();
    var d = EL("drawer"), c = null, groups = [], html;
    GRADES.list.forEach(function (x) { if (x.course === slug) { c = x; } });
    if (!c) { return; }
    (c.entries || []).forEach(function (e) {
      var k = e.category || "", g = null;
      groups.forEach(function (x) { if (x.cat === k) { g = x; } });
      if (!g) { g = { cat: k, rows: [] }; groups.push(g); }
      g.rows.push(e);
    });
    html = '<button class="close">&times;</button><h2>' + h(c.title) + "</h2>"
      + '<p class="hint">' + (c.pct == null ? "no grades yet" : h(c.pct) + "% " + h(c.letter || "")) + " · " + h(c.graded) + " graded · " + h(c.pending) + " waiting</p>";
    groups.forEach(function (g) {
      html += (g.cat ? '<h3 class="gcat">' + h(g.cat) + "</h3>" : "") + g.rows.map(gradeRowHtml).join("");
    });
    d.innerHTML = html + gradesFooterHtml(c);
    d.hidden = false; d.removeAttribute("data-id"); d.setAttribute("data-kind", "grades"); d.setAttribute("data-grades-open", c.course);
    d.querySelector(".close").addEventListener("click", function () { dropSavedToast(); d.hidden = true; d.removeAttribute("data-grades-open"); });
  }
  // M2 9.1: a background repaint never closes, rebuilds or refills an open body editor.
  function editorOpen() { return !!document.querySelector("#drawer [data-open-editor]"); }
  function refreshGradesDrawer() {
    var d = EL("drawer"), open = d && !d.hidden && d.getAttribute("data-kind") === "grades" ? d.getAttribute("data-grades-open") : null;
    if (open && !editorOpen()) { openGradesDrawer(open); }
  }
  function gradesSay(text) { GRADES.note = text || ""; drawGrades(); refreshGradesDrawer(); }
  // A capture's answer is a named outcome, never an error line; the strip re-reads its own state after
  // every press, and the day repaints from the new notes.
  function gradesAct(cmd, button) {
    if (button) { button.disabled = true; }
    gradesSay("");
    return invoke(cmd, {}).then(function (r) {
      gradesSay(r && r.ok === false ? String(r.error || "") : "");
      return gradesStatus().then(poll);
    }).catch(function () { if (button) { button.disabled = false; } });
  }
  EL("grades").addEventListener("click", function (e) {
    var ring = e.target.closest("[data-grades-course]"); if (ring) { openGradesDrawer(ring.getAttribute("data-grades-course")); return; }
    var conn = e.target.closest("[data-grades-connect]"); if (conn) { gradesAct("grades_connect", conn); return; }
    var ref = e.target.closest("[data-grades-refresh]"); if (ref) { gradesAct("grades_refresh", ref); }
  });
  EL("drawer").addEventListener("click", function (e) {
    var ref = e.target.closest("[data-grades-refresh]"); if (ref) { gradesAct("grades_refresh", ref); }
  });
  EL("set-grades-forget").addEventListener("click", function () {
    invoke("grades_forget", {}).then(function (r) {
      EL("set-grades-note").textContent = r && r.ok === false ? String(r.error || "") : "forgotten";
      return gradesStatus();
    }).catch(function () {});
  });
  // Hide grades (spec §2, §9): a settings row like Start with Windows, through set_settings; the
  // flag lives in grades.json, and the strip follows grades_status's `hidden`.
  EL("set-grades-hide-in").addEventListener("change", function () {
    invoke("set_settings", { patch: { grades_hidden: EL("set-grades-hide-in").checked } }).then(function (r) {
      EL("set-grades-note").textContent = r && r.ok === false ? String(r.error || "") : "";
      return gradesStatus();
    }).catch(function () {});
  });
  // ---- end M1 grades

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
    if (current.view === "today") { renderVerdict(state); renderMeter(state); renderMoved(state); renderGrades(state); }
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
    dropSavedToast();
    invoke("note", { id: id }).then(function (env) {
      var d = EL("drawer");
      if (!env.ok) { d.innerHTML = '<button class="close">&times;</button><h2>Not found</h2><p>' + h(env.error) + "</p>"; d.hidden = false; d.removeAttribute("data-id"); d.removeAttribute("data-grades-open"); return; }
      var n = env.note, fm = n.frontmatter || {}, dl = "", title = fm.title || n.slug;
      Object.keys(fm).sort().forEach(function (k) {
        var val = typeof fm[k] === "object" ? JSON.stringify(fm[k]) : fm[k];
        var attrs = EDITABLE.indexOf(k) !== -1 ? ' data-field="' + h(k) + '"' : (DRAWER_RO[k] ? ' class="ro" readonly' : "");
        dl += "<dt>" + h(k) + "</dt><dd" + attrs + ">" + h(val) + "</dd>";
      });
      var hist = (n.history || []).map(function (r) { return "<div>" + h(r.ts.slice(0, 16).replace("T", " ")) + " · " + h(r.text) + "</div>"; }).join("") || "<div>no journal history</div>";
      clearStaleEditing(d);
      d.innerHTML = '<button class="close">&times;</button><h2>' + h(title) + '</h2><dl data-id="' + h(id) + '" data-kind="' + h(n.folder) + '">' + dl + "</dl>" + bodyViewHtml(n) +
        '<button class="b crit" data-del="' + h(id) + '" data-title="' + h(title) + '">Delete&hellip;</button>' +
        '<div class="hist"><b>history</b>' + hist + "</div>";
      // The note's own kind (its folder — tasks/approvals/issues/info/courses) rides on the
      // outer panel too, since it also carries data-id and is what the observer actually sees
      // fill the viewport.
      d.hidden = false; d.setAttribute("data-id", id); d.setAttribute("data-kind", n.folder || ""); d.removeAttribute("data-grades-open");
      d.querySelector(".close").addEventListener("click", function () { dropSavedToast(); d.hidden = true; });
      var be = d.querySelector("[data-body-edit]");
      if (be) { be.addEventListener("click", function () { openBodyEditor(d, id, n.body || ""); }); }
    });
  }

  // M2 9.1: only a note in tasks/ or courses/ has an editable body. The text reaches the DOM as
  // escaped text (<pre>) or a textarea's value, never as markup, and never reaches a ui_event.
  var BODY_FOLDERS = { tasks: 1, courses: 1 };
  function bodyViewHtml(n) {
    var pre = n.body ? "<pre>" + h(n.body) + "</pre>" : "";
    if (!BODY_FOLDERS[n.folder]) { return pre; }
    return '<div class="bodyed">' + pre + '<button class="b" data-body-edit>' + (n.body ? "Edit" : "Add notes") + "</button></div>";
  }
  function openBodyEditor(d, id, expected) {
    var host = d.querySelector(".bodyed"); if (!host) { return; }
    host.setAttribute("data-open-editor", "body");
    host.innerHTML = '<textarea class="bodyta" rows="10"></textarea><div class="bodybtns"><button class="b" data-body-save>Save</button> <button class="b" data-body-cancel>Cancel</button></div>';
    var ta = host.querySelector("textarea"), sv = host.querySelector("[data-body-save]"), cn = host.querySelector("[data-body-cancel]");
    ta.value = expected; ta.focus();
    ev("edit_started", id, "body");
    function lock(on) { ta.disabled = on; sv.disabled = on; cn.disabled = on; }
    function conflictState() {
      if (host.querySelector("[data-body-conflict]")) { return; }
      var c = document.createElement("div"); c.setAttribute("data-body-conflict", ""); c.className = "bodyconf";
      c.innerHTML = "<p>This note changed since you opened it. Your text is still here. Copy it, then reload to see the new version.</p>" +
        '<button class="b" data-body-copy>Copy</button> <button class="b" data-body-reload>Reload</button>';
      host.appendChild(c);
      c.querySelector("[data-body-copy]").addEventListener("click", function () { invoke("copy_text", { text: ta.value }).catch(function () {}); });
      c.querySelector("[data-body-reload]").addEventListener("click", function () {
        if (ta.value !== expected && !confirm("Reload and discard your text?")) { return; }
        ev("edit_cancelled", id, "body"); openDrawer(id);
      });
    }
    function save() {
      lock(true);
      var before = expected;
      invoke("set_body", { view: stateView(), id: id, expected: expected, body: ta.value }).then(function (env) {
        if (env.conflict) { applyEnvelope(env, function () {}); lock(false); conflictState(); return; }
        if (applyEnvelope(env, function (m) { lock(false); showRefusal(null, m, id, "body"); })) { ev("edit_committed", id, "body"); openDrawer(id); offerBodyUndo(id, before); }
      }).catch(function (e) { lock(false); showRefusal(null, "refused: " + e, id, "body"); });
    }
    function cancel() { ev("edit_cancelled", id, "body"); openDrawer(id); }
    sv.addEventListener("click", save); cn.addEventListener("click", cancel);
    ta.addEventListener("keydown", function (e) {
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); save(); }
      else if (e.key === "Escape") { e.preventDefault(); cancel(); }
    });
  }

  // M2 D8: Saved - Undo for 10 seconds. Undo is a second set_body whose `expected` is the body of
  // the re-read note (never the textarea) and whose body is the text loaded before the edit.
  // The toast belongs to the drawer it was raised from: closing it or opening another note drops it.
  function dropSavedToast() {
    Array.prototype.forEach.call(document.querySelectorAll(".savedtoast"), function (x) { x.remove(); });
  }
  function offerBodyUndo(id, before) {
    invoke("note", { id: id }).then(function (env) {
      if (!env.ok || !env.note) { return; }
      var dr = EL("drawer");
      if (!dr || dr.hidden || dr.getAttribute("data-id") !== id) { return; }
      dropSavedToast();
      var t = document.createElement("div"); t.className = "savedtoast";
      t.innerHTML = '<span>Saved</span> &middot; <button class="b" data-body-undo>Undo</button>';
      document.body.appendChild(t);
      var gone = setTimeout(function () { t.remove(); }, 10000);
      t.querySelector("[data-body-undo]").addEventListener("click", function () {
        clearTimeout(gone); t.querySelector("[data-body-undo]").disabled = true;
        invoke("set_body", { view: stateView(), id: id, expected: env.note.body || "", body: before }).then(function (r) {
          if (applyEnvelope(r, function (m) {
            showRefusal(null, m, id, "body");
            t.innerHTML = '<button class="b" data-body-prev>Copy previous text</button>';
            t.querySelector("[data-body-prev]").addEventListener("click", function () { invoke("copy_text", { text: before }).catch(function () {}); });
            setTimeout(function () { t.remove(); }, 10000);
          })) { t.remove(); ev("edit_committed", id, "body"); var d = EL("drawer"); if (d && d.getAttribute("data-id") === id && !editorOpen()) { openDrawer(id); } }
        }).catch(function () { t.remove(); });
      });
    }).catch(function () {});
  }

  // P17, PQ6 (b): "Remove from my day" acts at once in the UI; the engine call waits for the
  // 10-second Removed - Undo toast to close. Undo cancels it, so nothing is sent and the date returns.
  function removeFromMyDay(uid, title) {
    if (pendingLaneRemovals[uid]) { return; }
    pendingLaneRemovals[uid] = true;
    if (current.state) { renderTheDay(current.state); }
    var t = document.createElement("div"); t.className = "lanetoast";
    t.style.bottom = "calc(var(--s4) + " + (document.querySelectorAll(".lanetoast").length * 44) + "px)";
    t.innerHTML = '<span>Removed</span> &middot; <button class="b" data-lane-undo>Undo</button>';
    document.body.appendChild(t);
    var timer = setTimeout(function () {
      t.remove();
      invoke("remove_lane_date", { view: stateView(), uid: uid }).then(function (env) {
        delete pendingLaneRemovals[uid];
        if (current.state) { renderTheDay(current.state); }
        applyEnvelope(env, function (m) { showRefusal(null, m); });
      }).catch(function () { delete pendingLaneRemovals[uid]; if (current.state) { renderTheDay(current.state); } });
    }, 10000);
    t.querySelector("[data-lane-undo]").addEventListener("click", function () {
      clearTimeout(timer); t.remove();
      delete pendingLaneRemovals[uid];
      if (current.state) { renderTheDay(current.state); }
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
    invoke("delete_note", { view: stateView(), id: id }).then(function (env) { if (applyEnvelope(env)) { dropSavedToast(); EL("drawer").hidden = true; } });
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
    var rml = e.target.closest("[data-lane-remove]"); if (rml) { removeFromMyDay(rml.getAttribute("data-lane-remove"), rml.getAttribute("data-title")); return; }
    var del = e.target.closest("[data-del]"); if (del) { confirmDelete(del.getAttribute("data-del"), del.getAttribute("data-title")); return; }
    // [data-flag]/[data-close-info] route before the .row[data-id] drawer branch below — a flag
    // or close click must never also open the drawer underneath it.
    var flagBtn = e.target.closest("[data-flag]"); if (flagBtn) { openFlag(flagBtn, flagBtn.getAttribute("data-flag")); return; }
    var undoBtn = e.target.closest("[data-undo-close]"); if (undoBtn) { undoClose(undoBtn.getAttribute("data-undo-close"), undoBtn); return; }
    var closeBtn = e.target.closest("[data-close-info]"); if (closeBtn) { closeInfoItem(closeBtn.getAttribute("data-close-info")); return; }
    // Task 15: manual sync/backup from the topline — both return the fresh state like every
    // other mutating command (spec's envelope shape), so applyEnvelope repaints it the same way.
    var syncBtn = e.target.closest("[data-sync]"); if (syncBtn) { runSync(); return; }
    var backupBtn = e.target.closest("[data-backup]"); if (backupBtn) { invoke("backup_now", { view: stateView() }).then(applyEnvelope).catch(function () {}); return; }
    var gear = e.target.closest("[data-settings]"); if (gear) { openSettings(); return; }
    // Plan 4a Task 8: *Restart to update*. A slot that started between the offer and the click is
    // waited out and the install retried; any other refusal comes back in the envelope and is shown
    // on the banner with Try again (`installUpdate`).
    var ins = e.target.closest("[data-install]");
    if (ins) { installUpdate(ins); return; }
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
  bindRegistrar(EL("ws-reg"), reloadSetupRows, setupBusy);
  bindRegistrar(EL("sched-reg"), function (env) { if (!env.state) { renderScheduleView(); } });
  // R5-c: Add confirms a registrar row at hard, as the office-hours Add confirms at optional.
  EL("sched-reg-rows").addEventListener("click", function (e) {
    e.stopPropagation();
    var b = e.target.closest("[data-reg-add]"); if (!b) { return; }
    b.disabled = true;
    confirmWeek({ mine: [{ source_uid: b.getAttribute("data-reg-add"), level: "hard" }] }).then(function (env) {
      if (!env.ok) { b.disabled = false; showRefusal(null, "refused: " + env.error); }
      if (!env.state) { return renderScheduleView(); }
    }).catch(function () { b.disabled = false; });
  });
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
  function renderAccountRow(a) {
    // L8 S5 (spec 3.4): a comped account reads the Settings tester line, never a status or a plan, and
    // has no subscription to manage.
    var tester = !!(a && a.ok && !a.needs_account && a.plan === "tester");
    var until = tester && a.current_period_end ? String(a.current_period_end).slice(0, 10) : "";
    EL("set-portal").hidden = tester;
    EL("set-account-state").textContent = (!a || !a.ok) ? "" : (a.needs_account
      ? "not attached to an account yet"
      : tester ? (a.email ? a.email + " · " : "") + "Free tester access" + (until ? " through " + until : "") + ". Nothing to pay."
      : [a.email || "", a.status || "", a.plan || ""].filter(function (x) { return x; }).join(" · "));
  }
  // Gmail connect T9a (spec §4.4, D1, D10, D12): the Google row. `SET` is the settings panel's own
  // state; the row keeps no cache, it asks `google_status` every time Settings opens. `googleSeq` is
  // bumped on close and on every new load, so a stale reply never repaints the row. The buttons are
  // wired in T9b (Connect, Reconnect) and T9c (Disconnect).
  var SET = { googleSeq: 0, google: null, googleNote: "", googlePolling: false, googleWaitedOut: false, confirming: false, leaving: false };
  var GOOGLE_TESTING = "While Google reviews Knowlu, this works only for invited testers, and the connection needs renewing about once a week.";
  // T9c (D6, D14): Disconnect is two steps, like Delete my data. Step 1 shows this and step 2.
  var GOOGLE_DISCONNECT_CONFIRM = "This disconnects Google Calendar too — Google keeps them as one permission. Knowlu stops reading both and deletes the proposals from your mail it had not delivered yet. What it already added stays in your vault, and your calendar stops updating.";
  var GOOGLE_DISCLOSURE = "Google also tells Knowlu which Google account you connected, so this row can show it.";
  function googleSentence(g) {
    if (!g) { return "Checking Google…"; }
    if (!g.ok) { return g.error || "Knowlu could not reach Google just now."; }
    var who = g.email ? " (as " + g.email + ")" : "";
    if (g.state === "revoked") { return "Google stopped answering for Knowlu. While Google reviews Knowlu, connections expire after seven days."; }
    if (g.state === "none") { return "Gmail is not connected. Knowlu can read your inbox for things you have to do and propose each one for you to approve."; }
    // L7 (spec 3.7; plan Q2 and Q3, 2026-10-19): the service found no mail for a week.
    if (g.state === "quiet" && g.gmail) { return "Gmail is connected" + who + ", but Knowlu has found no mail in it for a week. If this is the wrong account, disconnect it and connect the right one."; }
    if (g.gmail) { return "Gmail is connected" + who + ", read-only. Knowlu proposes what it finds; nothing is added without you."; }
    return "Google Calendar is connected" + who + ". Gmail is not.";
  }
  function renderGoogleRow() {
    var g = SET.google, busy = SET.googlePolling;
    var known = !!(g && g.ok), none = known && g.state === "none", revoked = known && g.state === "revoked";
    var live = known && !none && !revoked;
    EL("set-google-state").textContent = busy ? GOOGLE_POLLING : (SET.googleWaitedOut ? GOOGLE_NO_ANSWER + " " : "") + googleSentence(g);
    EL("set-google-check").hidden = !SET.googleWaitedOut;
    EL("set-google-note").textContent = (SET.confirming ? GOOGLE_DISCONNECT_CONFIRM + " " : "") + (SET.googleNote ? SET.googleNote + " " : "") + GOOGLE_TESTING + " " + GOOGLE_DISCLOSURE;
    EL("set-google-connect").hidden = !(none || (live && !g.gmail));
    EL("set-google-reconnect").hidden = !revoked;
    EL("set-google-disconnect-1").hidden = !(revoked || live);
    EL("set-google-retry").hidden = !(g && !g.ok);
    EL("set-google-disconnect-2").hidden = !(SET.confirming && (revoked || live));
    ["set-google-connect", "set-google-reconnect", "set-google-disconnect-1", "set-google-disconnect-2", "set-google-retry", "set-google-check"].forEach(function (id) {
      // Connect and Reconnect stay live during the ten-minute poll: pressing one again mints a new link.
      var retry = id === "set-google-connect" || id === "set-google-reconnect";
      EL(id).disabled = SET.leaving || (busy && !retry);
    });
    lockGoogleStarts();
  }
  /** While the privacy update is owed, Connect and Reconnect start nothing; Disconnect is untouched. */
  function lockGoogleStarts() {
    if (!RECONSENT_OWED) { return; }
    EL("set-google-connect").disabled = true; EL("set-google-reconnect").disabled = true;
  }
  function loadGoogleRow() {
    var seq = ++SET.googleSeq;
    SET.google = null; SET.googleNote = ""; SET.googlePolling = false; SET.googleWaitedOut = false; SET.confirming = false;
    EL("set-google").hidden = false;
    renderGoogleRow();
    return invoke("google_status", {}).then(function (g) {
      if (seq !== SET.googleSeq) { return; }
      SET.google = g || { ok: false, error: UNREACHABLE };
      renderGoogleRow();
    }).catch(function () {
      if (seq !== SET.googleSeq) { return; }
      SET.google = { ok: false, error: UNREACHABLE };
      renderGoogleRow();
    });
  }
  // T9b: Connect ("gmail") and Reconnect ("reconnect"). The browser opens from Rust; the page only
  // polls google_status until the consent link's own lifetime is up (the server's nonce dies ten
  // minutes after minting), starting at 3 s and backing off to 10 s. Then the row says Google was
  // not heard from and offers "Check again", which re-reads the status and never mints a new link.
  // `mySeq` is taken on every click and Settings close bumps `SET.googleSeq`, so a stale poll never
  // repaints the row (as `SET.googleSeq` does).
  var GOOGLE_POLLING = "Waiting for Google… finish in your browser.";
  var GOOGLE_LINK_LIFETIME_MS = 600000;
  var GOOGLE_NO_ANSWER = "Didn’t hear back from Google.";
  var GOOGLE_TIMEOUT = "Google did not finish connecting. If Google said Knowlu is not verified, this Google account is not on the tester list yet.";
  function googleDone(g, scope) { return !!(g && g.ok && (scope === "reconnect" ? g.state === "active" : g.gmail === true)); }
  async function connectGoogle(scope) {
    var mySeq = ++SET.googleSeq;
    SET.googlePolling = true; SET.googleWaitedOut = false; SET.googleScope = scope; SET.googleNote = ""; SET.confirming = false;
    renderGoogleRow();
    var started = await invoke("google_connect", { scope: scope }).catch(function () { return { ok: false, error: UNREACHABLE }; });
    if (SET.googleSeq !== mySeq) { return; }
    if (!started || !started.ok) {
      SET.googlePolling = false; SET.googleNote = (started && started.error) || UNREACHABLE;
      renderGoogleRow();
      return;
    }
    var deadline = Date.now() + GOOGLE_LINK_LIFETIME_MS, delay = 3000;
    while (Date.now() < deadline) {
      await new Promise(function (r) { setTimeout(r, delay); });
      if (SET.googleSeq !== mySeq) { return; }
      delay = Math.min(Math.round(delay * 1.25), 10000);
      var g = await invoke("google_status", {}).catch(function () { return null; });
      if (SET.googleSeq !== mySeq) { return; }
      if (googleDone(g, scope)) {
        SET.google = g; SET.googlePolling = false; SET.googleNote = "";
        renderGoogleRow();
        return;
      }
    }
    var fresh = await invoke("google_status", {}).catch(function () { return null; });
    if (SET.googleSeq !== mySeq) { return; }
    SET.google = fresh || { ok: false, error: UNREACHABLE };
    SET.googlePolling = false; SET.googleNote = GOOGLE_TIMEOUT;
    SET.googleWaitedOut = !googleDone(SET.google, scope);
    renderGoogleRow();
  }
  // "Check again": one status read, no google_connect, so no new link is minted.
  async function checkGoogleAgain() {
    var mySeq = ++SET.googleSeq;
    SET.googlePolling = true;
    renderGoogleRow();
    var g = await invoke("google_status", {}).catch(function () { return null; });
    if (SET.googleSeq !== mySeq) { return; }
    SET.googlePolling = false;
    if (g) { SET.google = g; }
    if (googleDone(g, SET.googleScope)) { SET.googleWaitedOut = false; SET.googleNote = ""; }
    renderGoogleRow();
  }
  // T9c: step 2. On ok: false the row keeps its previous state and shows the error under it; on ok
  // the row is asked afresh. A close or a new click bumps the token, so a late reply repaints nothing.
  async function disconnectGoogle() {
    var mySeq = ++SET.googleSeq;
    SET.leaving = true;
    renderGoogleRow();
    var r = await invoke("google_disconnect", {}).catch(function () { return { ok: false, error: UNREACHABLE }; });
    if (SET.googleSeq !== mySeq) { return; }
    SET.leaving = false;
    if (r && r.ok) { loadGoogleRow(); return; }
    SET.confirming = false; SET.googleNote = (r && r.error) || UNREACHABLE;
    renderGoogleRow();
  }
  // Signed-in students only: `account_status` says `needs_account` (or nothing) and the row is hidden.
  function gateGoogleRow(a) {
    // checkAccount also runs at launch; the row asks google_status only while Settings is open.
    if (EL("settings").hidden) { return; }
    if (a && a.ok && !a.needs_account) { loadGoogleRow(); return; }
    SET.googleSeq += 1; EL("set-google").hidden = true;
  }
  function closeSettings() { SET.googleSeq += 1; SET.googlePolling = false; SET.googleWaitedOut = false; SET.confirming = false; SET.leaving = false; EL("settings").hidden = true; maybeReconsent(); }
  // The account row and the upgrade overlay's gate, from ONE reply — `account_status` answers from
  // this machine only (no network call), and asking twice for the same three fields is two answers
  // that can disagree. A failed call is treated as "not reachable", not as "no account": the overlay
  // stays down and the console is exactly as usable as it was before C1. Best effort in both
  // directions — the vault-less shell does not register the command, and neither does a build made
  // before Task 18.
  function checkAccount() {
    return invoke("account_status", {}).then(function (s) {
      renderAccountRow(s);
      gateGoogleRow(s);
      maybeUpgrade(s);
      return askPrivacy(s);
    }).catch(function () { renderAccountRow(null); gateGoogleRow(null); EL("upgrade").hidden = true; });
  }
  // M2 T5b.1: Settings, "How you like to work". PREFS.loaded is the text `profile` last returned,
  // the `expected` of a save. Events carry the object kind only, never the text (D15).
  var PREFS = { loaded: "", started: false };
  function prefsDirty() { return EL("set-prefs-ta").value !== PREFS.loaded; }
  function prefsCount() {
    var n = Array.from(EL("set-prefs-ta").value).length, c = EL("set-prefs-count");
    c.textContent = n + " / 600";
    c.classList.toggle("over", n > 600);
  }
  function prefsFill(p) {
    PREFS.loaded = p.preferences || ""; PREFS.started = false;
    EL("set-prefs-ta").value = PREFS.loaded; prefsCount();
    var c = document.querySelector("[data-prefs-conflict]"); if (c) { c.remove(); }
  }
  function loadPrefs() {
    if (prefsDirty()) { return Promise.resolve(); }
    return invoke("profile", {}).then(function (env) {
      if (env.ok && env.profile && !prefsDirty()) { prefsFill(env.profile); }
    }).catch(function () {});
  }
  function prefsLock(on) { EL("set-prefs-ta").disabled = on; EL("set-prefs-save").disabled = on; EL("set-prefs-cancel").disabled = on; }
  function prefsConflict() {
    if (document.querySelector("[data-prefs-conflict]")) { return; }
    var c = document.createElement("div"); c.setAttribute("data-prefs-conflict", ""); c.className = "prefsconf";
    c.innerHTML = "<p>This changed since you opened it. Your text is still here. Copy it, then reload to see the new version.</p>" +
      '<button class="b" data-prefs-copy>Copy</button> <button class="b" data-prefs-reload>Reload</button>';
    EL("set-prefs").appendChild(c);
    c.querySelector("[data-prefs-copy]").addEventListener("click", function () { invoke("copy_text", { text: EL("set-prefs-ta").value }).catch(function () {}); });
    c.querySelector("[data-prefs-reload]").addEventListener("click", function () {
      if (prefsDirty() && !confirm("Reload and discard your text?")) { return; }
      ev("edit_cancelled", null, "preferences");
      invoke("profile", {}).then(function (env) { if (env.ok && env.profile) { prefsFill(env.profile); } }).catch(function () {});
    });
  }
  function savePrefs() {
    var before = PREFS.loaded;
    prefsLock(true);
    invoke("set_preferences", { view: stateView(), expected: PREFS.loaded, text: EL("set-prefs-ta").value }).then(function (env) {
      if (env.conflict) { applyEnvelope(env, function () {}); prefsLock(false); prefsConflict(); return; }
      if (!applyEnvelope(env, function (m) { prefsLock(false); showRefusal(null, m, null, "preferences"); })) { return; }
      ev("edit_committed", null, "preferences");
      return invoke("profile", {}).then(function (p) {
        prefsLock(false);
        if (!p.ok || !p.profile) { return; }
        prefsFill(p.profile); offerPrefsUndo(before, PREFS.loaded);
      });
    }).catch(function (e) { prefsLock(false); showRefusal(null, "refused: " + e, null, "preferences"); });
  }
  function offerPrefsUndo(before, after) {
    Array.prototype.forEach.call(document.querySelectorAll(".settoast"), function (x) { x.remove(); });
    var t = document.createElement("div"); t.className = "settoast";
    t.innerHTML = '<span>Saved</span> &middot; <button class="b" data-prefs-undo>Undo</button>';
    document.body.appendChild(t);
    var gone = setTimeout(function () { t.remove(); }, 10000);
    t.querySelector("[data-prefs-undo]").addEventListener("click", function () {
      clearTimeout(gone); t.querySelector("[data-prefs-undo]").disabled = true;
      invoke("set_preferences", { view: stateView(), expected: after, text: before }).then(function (r) {
        if (applyEnvelope(r, function (m) {
          showRefusal(null, m, null, "preferences");
          t.innerHTML = '<button class="b" data-prefs-prev>Copy previous text</button>';
          t.querySelector("[data-prefs-prev]").addEventListener("click", function () { invoke("copy_text", { text: before }).catch(function () {}); });
          setTimeout(function () { t.remove(); }, 10000);
        })) {
          t.remove(); ev("edit_committed", null, "preferences");
          invoke("profile", {}).then(function (p) { if (p.ok && p.profile) { prefsFill(p.profile); } }).catch(function () {});
        }
      }).catch(function () { t.remove(); });
    });
  }
  EL("set-prefs-ta").addEventListener("input", function () {
    var first = !PREFS.started;
    PREFS.started = true;
    if (first) { ev("edit_started", null, "preferences"); }
    prefsCount();
  });
  EL("set-prefs-save").addEventListener("click", savePrefs);
  EL("set-prefs-cancel").addEventListener("click", function () {
    if (prefsDirty()) { ev("edit_cancelled", null, "preferences"); }
    EL("set-prefs-ta").value = PREFS.loaded; PREFS.started = false; prefsCount();
  });
  EL("set-prefs-ta").addEventListener("keydown", function (e) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); savePrefs(); }
  });
  // M2 T5b.1 end
  // M2 T5b.2: Settings, "Campus events". INT.loaded is the four lists `profile` last returned; undo
  // sends the ones held before the save. Events carry the object kind only, never an item (D15).
  var INT_KEYS = ["strong", "mild", "never", "clubs"];
  var INT = { loaded: null, started: false, editable: true };
  function intRead() {
    var out = {};
    INT_KEYS.forEach(function (k) {
      out[k] = EL("set-int-" + k).value.split("\n").map(function (s) { return s.trim(); }).filter(function (s) { return s; });
    });
    return out;
  }
  function intDirty() { return !!INT.loaded && JSON.stringify(intRead()) !== JSON.stringify(INT.loaded); }
  function intLock(on) {
    INT_KEYS.forEach(function (k) { EL("set-int-" + k).disabled = on; });
    EL("set-int-save").disabled = on || !INT.editable; EL("set-int-cancel").disabled = on;
  }
  function intFill(p) {
    INT.loaded = {}; INT.started = false; INT.editable = p.interests_editable !== false;
    INT_KEYS.forEach(function (k) {
      INT.loaded[k] = (p[k] || []).slice();
      var ta = EL("set-int-" + k); ta.value = INT.loaded[k].join("\n"); ta.readOnly = !INT.editable;
    });
    EL("set-int-save").disabled = !INT.editable; EL("set-int-cancel").disabled = !INT.editable;
    var n = EL("set-int-reason"), r = "Knowlu can only edit a list written on one line. Yours continues onto following lines, so it is shown here to read only.";
    if (!INT.editable && !n) {
      n = document.createElement("p"); n.id = "set-int-reason"; n.className = "meta set-prefs-copy"; n.textContent = r;
      EL("set-int-note").parentNode.insertBefore(n, EL("set-int-note"));
    }
    if (INT.editable && n) { n.remove(); }
  }
  function loadInterests() {
    if (intDirty()) { return Promise.resolve(); }
    return invoke("profile", {}).then(function (env) {
      if (env.ok && env.profile && !intDirty()) { intFill(env.profile); }
    }).catch(function () {});
  }
  function saveInterests() {
    var held = INT.loaded;
    var sent = intRead();
    intLock(true);
    invoke("set_interests", { view: stateView(), strong: sent.strong, mild: sent.mild, never: sent.never, clubs: sent.clubs }).then(function (env) {
      if (!applyEnvelope(env, function (m) { intLock(false); showRefusal(null, m, null, "interests"); })) { return; }
      ev("edit_committed", null, "interests");
      return invoke("profile", {}).then(function (p) {
        intLock(false);
        if (!p.ok || !p.profile) { return; }
        intFill(p.profile); offerIntUndo(held);
      });
    }).catch(function (e) { intLock(false); showRefusal(null, "refused: " + e, null, "interests"); });
  }
  function offerIntUndo(before) {
    Array.prototype.forEach.call(document.querySelectorAll(".settoast"), function (x) { x.remove(); });
    var t = document.createElement("div"); t.className = "settoast";
    t.innerHTML = '<span>Saved</span> &middot; <button class="b" data-int-undo>Undo</button>';
    document.body.appendChild(t);
    var gone = setTimeout(function () { t.remove(); }, 10000);
    t.querySelector("[data-int-undo]").addEventListener("click", function () {
      clearTimeout(gone); t.querySelector("[data-int-undo]").disabled = true;
      invoke("set_interests", { view: stateView(), strong: before.strong, mild: before.mild, never: before.never, clubs: before.clubs }).then(function (r) {
        if (applyEnvelope(r, function (m) { t.remove(); showRefusal(null, m, null, "interests"); })) {
          t.remove(); ev("edit_committed", null, "interests");
          invoke("profile", {}).then(function (p) { if (p.ok && p.profile) { intFill(p.profile); } }).catch(function () {});
        }
      }).catch(function () { t.remove(); });
    });
  }
  INT_KEYS.forEach(function (k) {
    EL("set-int-" + k).addEventListener("input", function () {
      var first = !INT.started;
      INT.started = true;
      if (first) { ev("edit_started", null, "interests"); }
    });
  });
  EL("set-int-save").addEventListener("click", saveInterests);
  EL("set-int-cancel").addEventListener("click", function () {
    if (intDirty()) { ev("edit_cancelled", null, "interests"); }
    if (INT.loaded) { intFill({ strong: INT.loaded.strong, mild: INT.loaded.mild, never: INT.loaded.never, clubs: INT.loaded.clubs, interests_editable: INT.editable }); }
  });
  // M2 T5b.2 end
  // ---- mail progress view (email spec §4.5, D6, BW). Every string here is a fixed sentence of the engine's
  // or this page's: no mailbox text reaches it, and it reports nothing to the app log. ----
  var MP = { timer: null, last: null };
  var MP_STEP = { sign_in: "Sign in", step_aside: "Step aside", forwarding: "Open forwarding", add_address: "Add Knowlu’s address",
    confirm_address: "Confirm the address", turn_on: "Turn forwarding on", canary: "Test message", backfill: "Bring in recent mail", finish: "Finish" };
  var MP_STATE = { waiting: "waiting", working: "working", done: "done", needs_you: "needs you", done_by_hand: "done by hand", skipped: "skipped", failed: "failed" };
  var MP_REASON = { guided: "Do this one yourself, then say you have.", forwarding_elsewhere: "This mailbox already forwards somewhere else.",
    not_found: "Knowlu could not find the setting on the page.", script_failed: "That step did not work.",
    blocked: "The test message did not arrive. Your school may be blocking forwarding." };
  // A step's own words for a reason, read before MP_REASON: sign-in's not_found is the mailbox address the
  // page never showed, not a setting.
  var MP_STEP_REASON = { sign_in: { not_found: "Knowlu could not read your mailbox’s address from the page. Start setup again.",
    script_failed: "Signing in did not finish. Start setup again.",
    already_connected: "This mailbox is already set up in Knowlu. Open Settings, then Email, to resume it or turn it off there.",
    refused: "Knowlu’s service did not accept this mailbox, for the reason given below." } };
  var MP_MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  function mpDate(iso) {
    var m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(iso || ""));
    return m ? Number(m[3]) + " " + MP_MONTHS[Number(m[2]) - 1] : String(iso || "");
  }
  function mpLeft(secs) {
    if (secs == null) { return ""; }
    var m = Math.max(1, Math.round(secs / 60));
    return m >= 60 ? " · about " + Math.round(m / 60) + " h left" : " · about " + m + " min left";
  }
  function mpBtn(label, choice, extra) { return '<button' + (choice === "stop" ? "" : owedAttrs()) + ' class="b' + (extra || "") + '" data-mp-choice="' + h(choice) + '">' + h(label) + "</button>"; }
  // A guided step's directions (spec §4.4): the provider's own path to the setting, four lines at most, keyed on
  // the provider and the step, never on a school. The address comes from mail_status by the setup's opaque id.
  var MP_PATH = {
    m365: ["Open Outlook on the web and choose Settings.", "Go to Mail, then Forwarding.", "Turn forwarding on and paste the address above.", "Save, then come back here."],
    google: ["Open Gmail and choose Settings, then See all settings.", "Open Forwarding and POP/IMAP.", "Choose Add a forwarding address and paste the address above.", "Confirm it, turn forwarding on, then come back here."],
    other: ["Open your mail provider’s settings.", "Find Forwarding (sometimes under Mail or Rules).", "Forward a copy of every message to the address above.", "Save, then come back here."]
  };
  var MP_PATH_CONFIRM = ["Open the confirmation message your provider sent.", "Follow its link or enter its code.", "Then come back here."];
  var MPA = { id: null, address: null, busy: false };
  function mpGuided(p) {
    return p.rows.some(function (x) { return x.state === "needs_you" || x.state === "failed" || (x.guided && x.state === "working"); });
  }
  function mpAddressFor(p) {
    var want = p.address_id;
    if (!want || MPA.busy || MPA.id === want) { return; }
    MPA.busy = true;
    invoke("mail_status", {}).then(function (s) {
      var m = ((s && s.mailboxes) || []).filter(function (x) { return x["address_id"] === want; })[0];
      MPA.id = want; MPA.address = m ? m.address || null : null; MPA.busy = false;
      if (MP.last) { renderMailProgress({ progress: MP.last }); }
    }).catch(function () { MPA.busy = false; });
  }
  function mpGuide(p) {
    if (p.status === "ended" || !mpGuided(p)) { return ""; }
    var row = p.rows.filter(function (x) { return x.state === "needs_you" || x.state === "failed" || x.guided; })[0] || {};
    // The held Microsoft 365 backfill: forwarding is already set and verified, so no address and no path.
    if (row.step === "backfill" && p.provider === "m365") {
      return '<div class="mp-guide"><p>' + h("Knowlu can’t bring in Outlook’s older mail yet. New mail is already arriving — press I’ve done this to finish.") + "</p></div>";
    }
    mpAddressFor(p);
    var lines = row.step === "confirm_address" ? MP_PATH_CONFIRM : (MP_PATH[p.provider] || MP_PATH.other);
    var addr = MPA.id === p.address_id && MPA.address ? '<div class="set-row"><code class="mp-addr">' + h(MPA.address) + "</code>" +
      '<button class="b" data-mp-copy>Copy</button></div>' : "";
    return '<div class="mp-guide">' + addr + "<ol>" + lines.map(function (l) { return "<li>" + h(l) + "</li>"; }).join("") + "</ol></div>";
  }
  function renderMailProgress(r) {
    var p = r && r.progress; MP.last = p || null;
    if (!p) {
      EL("mp-sentence").textContent = "No email setup is running.";
      EL("mp-rows").innerHTML = ""; EL("mp-backfill").textContent = ""; EL("mp-ask").innerHTML = ""; EL("mp-end").textContent = "";
      EL("mp-stop").hidden = true;
      return;
    }
    EL("mp-sentence").textContent = p.sentence || "";
    var html = "";
    p.rows.forEach(function (row) {
      var said = row.reason && ((MP_STEP_REASON[row.step] || {})[row.reason] || MP_REASON[row.reason]);
      var why = said ? '<span class="mp-why">' + h(said) + "</span>" : "";
      html += '<div class="mp-row mp-' + h(row.state) + '"><span class="mp-step">' + h(MP_STEP[row.step] || row.step) +
        '</span><span class="mp-state">' + h(MP_STATE[row.state] || row.state) + "</span>" + why + "</div>";
    });
    EL("mp-rows").innerHTML = html;
    var b = p.backfill;
    EL("mp-backfill").textContent = b ? "Backfill: mail from the last 120 days, since " + mpDate(b.since) + " — " + b.handled + " of " + b.expected +
      mpLeft(b.eta_secs) + (b.paused ? " (paused)" : "") : "";
    var ask = mpGuide(p);
    if (p.ask === "school_address") {
      ask = '<div class="set-row"><input type="text" id="mp-addr" placeholder="Your school email address">' + mpBtn("Continue", "school_address", " pri y") + "</div>";
    } else if (p.ask === "forwarding_elsewhere") {
      ask = '<div class="set-row">' + mpBtn("Forward to Knowlu instead", "replace") + mpBtn("Keep it as it is", "stop") + "</div>";
    } else if (p.status !== "ended" && p.rows.some(function (x) { return x.state === "needs_you" || x.state === "failed"; })) {
      ask += '<div class="set-row">' + mpBtn("I’ve done this", "done", " pri y") + (p.blocked ? mpBtn("Try again", "retry") : "") + "</div>";
    }
    if (p.blocked && (p.offers || []).indexOf("personal_gmail") >= 0) { ask += '<div class="set-row"><button class="b" id="mp-gmail"' + owedAttrs() + '>Connect personal Gmail instead</button></div>'; }
    EL("mp-ask").innerHTML = ask;
    var end = "";
    if (p.status === "ended") {
      end = p.line && p.line !== p.sentence ? p.line : "";
      // The row already says it; but a setup that kept its address says so only here (runner.rs KEPT_*_LINE).
      var kept = p.kept === true;
      if (!kept && p.rows.some(function (x) { return x.reason === "already_connected"; })) { end = ""; }
      if (p.done && p.done.length) { end += " Done: " + p.done.map(function (s) { return MP_STEP[s] || s; }).join(", ") + "."; }
      if (p.left && p.left.length) { end += " Not done: " + p.left.map(function (s) { return MP_STEP[s] || s; }).join(", ") + "."; }
      if (p.not_brought_in_before) { end += " Mail before " + mpDate(p.not_brought_in_before) + " was not brought in."; }
    }
    EL("mp-end").textContent = end.trim();
    EL("mp-stop").hidden = p.status === "ended";
  }
  function pollMailProgress() {
    return invoke("mail_setup_progress", {}).then(function (r) {
      renderMailProgress(r);
      if (!r || !r.active) { clearInterval(MP.timer); MP.timer = null; }
      return r;
    }).catch(function () {});
  }
  function openMailProgress() {
    // "From your email" sits above this panel in page order; opened from Settings it could cover it.
    EL("mail-applied").hidden = true;
    EL("mail-progress").hidden = false; maybeReconsent();
    if (!MP.timer) { MP.timer = setInterval(pollMailProgress, 1500); }
    return pollMailProgress();
  }
  function mailLeave() {
    EL("mail-progress").hidden = true; maybeReconsent();
    clearInterval(MP.timer); MP.timer = null;
  }
  function mailFirstOpen() {
    if (RECONSENT_OWED) { return Promise.resolve(); }
    return invoke("mail_status", {}).then(function (s) {
      if (s && s.offer_pending && !s.setup_running) {
        return invoke("mail_setup_start", { provider: "school" }).then(function (r) { if (r && r.ok) { return openMailProgress(); } });
      }
    }).catch(function () {});
  }
  function mailSettingsRow() {
    return invoke("mail_setup_progress", {}).then(function (r) {
      var p = r && r.progress;
      EL("set-email-state").textContent = !p ? "Not connected here." : p.status === "ended" ? (p.line || "Setup ended.") : (p.sentence || "Setting up.");
      EL("set-email-open").hidden = !p;
      EL("set-email-start").hidden = !!(r && r.active);
      return mailBoxesRow();
    }).catch(function () {});
  }
  EL("mail-progress").addEventListener("click", function (e) {
    if (e.target.closest("#mp-leave")) { mailLeave(); return; }
    if (e.target.closest("#mp-stop")) { invoke("mail_setup_stop", {}).then(pollMailProgress).catch(function () {}); return; }
    if (e.target.closest("#mp-gmail")) { invoke("mail_setup_start", { provider: "gmail" }).then(function (r) { if (r && r.ok) { openMailProgress(); } }).catch(function () {}); return; }
    if (e.target.closest("[data-mp-copy]")) { if (MPA.address && navigator.clipboard) { navigator.clipboard.writeText(MPA.address).catch(function () {}); } return; }
    var c = e.target.closest("[data-mp-choice]"); if (!c) { return; }
    var k = c.getAttribute("data-mp-choice"), a;
    if (k === "school_address") { a = invoke("mail_setup_answer", { choice: { school_address: EL("mp-addr").value.trim() } }); }
    else if (k === "done") { a = invoke("mail_setup_answer", { choice: "done" }); }
    else if (k === "retry") { a = invoke("mail_setup_answer", { choice: "retry" }); }
    else if (k === "replace") { a = invoke("mail_setup_answer", { choice: "replace" }); }
    else { a = invoke("mail_setup_answer", { choice: "stop" }); }
    a.then(pollMailProgress).catch(function () {});
  });
  EL("set-email-open").addEventListener("click", openMailProgress);
  EL("set-email-start").addEventListener("click", function () {
    invoke("mail_setup_start", { provider: "school" }).then(function (r) { if (r && r.ok) { openMailProgress(); } }).catch(function () {});
  });
  // ---- end mail progress view ----
  // ---- mail settings (email spec §4.12, §4.13, D15, T15.2). A mailbox row is the service's status joined to what
  // this device keeps. Everything the rows show reaches the DOM through h(); nothing here reports to the app log. ----
  var SETM = { s: null, confirming: null, notes: [], busy: false };
  var EM_STATE = { pending: "Setting up.", confirmed: "Confirmed; waiting for the first message.", active: "Forwarding is working.",
    quiet: "No mail has come through for a while.", blocked: "The test message did not arrive. Your school may be blocking forwarding.",
    off: "Forwarding is off or goes to another address.", paused_send: "Forwarding paused — Microsoft refused a send.",
    lapsed: "Paused because your subscription lapsed.", revoked: "Disconnected." };
  var EM_PROVIDER = { m365: "School email", google: "Google mail", other: "School email" };
  var EM_TURN_OFF_CONFIRM = "Knowlu turns off forwarding in this mailbox, stops reading it and deletes what it holds to serve it. What it already added stays in your plan.";
  var EM_FORWARD_SENTENCE = "If Knowlu cannot turn forwarding off, your mail keeps going to Knowlu's address and is deleted unread; you can turn it off in your mail settings.";
  var EM_STILL = "Knowlu’s forwarding address is still in your Gmail forwarding list. Remove it by hand: in Gmail, open Settings, then See all settings, then Forwarding and POP/IMAP, and remove it there.";
  function emBtn(label, act, id, extra) { return '<button class="b' + (extra || "") + '" data-em-act="' + act + '" data-em-id="' + h(id) + '">' + h(label) + "</button>"; }
  function emCopy(key, address) { return '<span class="em-line"><code>' + h(address) + '</code> <button class="b" data-em-copy="' + h(key) + '">Copy</button></span>'; }
  function mailDeleteSentence() {
    var s = SETM.s;
    return !s || !s.ok || (s.mailboxes || []).length ? " " + EM_FORWARD_SENTENCE : "";
  }
  function mailBoxRow(m) {
    var id = m.address_id, ask = SETM.confirming === id;
    var m365Backfill = m.open_backfill && m.provider === "m365";
    var resumable = !m.turned_off && ((m.open_backfill && !m365Backfill) || ["off", "quiet", "blocked", "paused_send"].indexOf(m.state) >= 0);
    var say = m.turned_off ? "Turned off in Knowlu." : (EM_STATE[m.state] || "");
    if (m365Backfill && !m.turned_off) { say += " New mail is arriving. Knowlu will bring in older Outlook mail on a later run."; }
    else if (m.open_backfill && !m.turned_off) { say += " Still bringing in recent mail; Resume carries on."; }
    var line = m.unbound_reason ? '<span class="em-line em-amber">' + h(m.unbound_note || "") + " (" + h(m.unbound_since_live) + ")" +
      (m.suggest_rotate === true && !m.turned_off ? " " + emBtn("Rotate", "rotate", id).replace("<button", "<button" + owedAttrs()) : "") + "</span>" : "";
    var rotate = m.suggest_rotate !== true && !m.turned_off ? emBtn("Rotate", "rotate", id).replace("<button", "<button" + owedAttrs()) : "";
    return '<div class="em-box"><span class="em-name">' + h(EM_PROVIDER[m.provider] || "Email") + "</span>" +
      (resumable ? (RECONSENT_OWED ? emBtn("Resume", "resume", id).replace("<button ", '<button disabled title="' + h(OWED_LINE) + '" ') : emBtn("Resume", "resume", id)) : "") + rotate + emBtn("Turn off email", "off1", id) +
      '<span class="em-line">' + h(say) + "</span>" + (m.address ? emCopy("m:" + id, m.address) : "") + line +
      (ask ? '<span class="em-confirm">' + h(EM_TURN_OFF_CONFIRM) + "</span>" + emBtn("Yes, turn off email", "off2", id, " crit") + emBtn("Cancel", "cancel", id) : "") + "</div>";
  }
  function renderMailBoxes() {
    var s = SETM.s, html = "";
    if (s && s.ok && s.mailboxes) { s.mailboxes.forEach(function (m) { html += mailBoxRow(m); }); }
    else if (s) { html = '<div class="em-box"><span class="em-line">Knowlu could not read your mailboxes just now.</span></div>'; }
    SETM.notes.forEach(function (n, i) {
      html += '<div class="em-box"><span class="em-line">' + n.lines.map(h).join(" ") + "</span>" + (n.address ? emCopy("n:" + i, n.address) : "") + "</div>";
    });
    EL("set-email-boxes").innerHTML = html;
    EL("set-email-keep").checked = !s || s.keep_session !== false;
    EL("set-email-profile").hidden = !(s && s.profile_delete_pending);
    EL("set-email-profile-note").textContent = "A saved email sign-in could not be deleted from this computer yet. Knowlu tries again each time it starts.";
  }
  function mailBoxesRow() {
    return invoke("mail_status", {}).then(function (s) { SETM.s = s || null; renderMailBoxes(); }).catch(function () {});
  }
  function emNote(r) {
    var lines = [], still = false;
    (r.steps || []).forEach(function (st) {
      if (st.step === "remove_address" && st.state === "needs_you") { still = true; lines.push(EM_STILL); }
      else if (st.message) { lines.push(st.message); }
    });
    if (!r.ok) { lines.unshift("Email could not be turned off yet" + (r.error ? ": " + r.error + "." : ".") + " Turn off email again to retry."); }
    else if (!r.needs_you) { lines.unshift("Email is turned off for this mailbox. What Knowlu already added stays in your plan."); }
    return { lines: lines, address: r.needs_you || still ? r.address || null : null };
  }
  EL("set-email-boxes").addEventListener("click", function (e) {
    var cp = e.target.closest("[data-em-copy]");
    if (cp) {
      var key = cp.getAttribute("data-em-copy"), addr = null;
      if (key.indexOf("n:") === 0) { addr = (SETM.notes[Number(key.slice(2))] || {}).address; }
      else { addr = ((SETM.s && SETM.s.mailboxes) || []).filter(function (m) { var mid = m.address_id; return "m:" + mid === key; }).map(function (m) { return m.address; })[0]; }
      if (addr && navigator.clipboard) { navigator.clipboard.writeText(addr).catch(function () {}); }
      return;
    }
    var b = e.target.closest("[data-em-act]"); if (!b || SETM.busy) { return; }
    var act = b.getAttribute("data-em-act"), id = b.getAttribute("data-em-id");
    if (act === "off1") { SETM.confirming = id; renderMailBoxes(); return; }
    if (act === "cancel") { SETM.confirming = null; renderMailBoxes(); return; }
    SETM.busy = true;
    var done = function () { SETM.busy = false; };
    if (act === "off2") {
      SETM.confirming = null;
      invoke("mail_disconnect", { addressId: id }).then(function (r) { SETM.notes = [emNote(r || {})]; done(); return mailBoxesRow(); })
        .catch(function () { SETM.notes = [{ lines: ["Email could not be turned off yet. Turn off email again to retry."], address: null }]; done(); return mailBoxesRow(); });
    } else if (act === "resume") {
      invoke("mail_sign_in", { addressId: id }).then(function (r) {
        SETM.notes = r && r.ok ? [] : [{ lines: [(r && r.error) || "Could not sign in to your mail."], address: null }]; done(); return mailBoxesRow();
      }).catch(function () { done(); });
    } else if (act === "rotate") {
      invoke("mail_rotate", { addressId: id }).then(function (r) {
        done();
        if (r && r.ok) { SETM.notes = []; return openMailProgress(); }
        SETM.notes = [{ lines: [(r && r.error) || "Could not start a new address."], address: null }]; renderMailBoxes();
      }).catch(function () { done(); });
    } else { done(); }
  });
  EL("set-email-keep").addEventListener("change", function () {
    invoke("mail_keep_session", { on: EL("set-email-keep").checked }).then(function (r) {
      EL("set-email-keep-note").textContent = !r || !r.ok ? ((r && r.error) || "could not change it") :
        r.profiles_left ? "Some saved email sign-in could not be deleted yet; Knowlu tries again when it starts." : "";
      return mailBoxesRow();
    }).catch(function () { return mailBoxesRow(); });
  });
  // ---- end mail settings ----
  // ---- mail applied (email spec §4.14, D10, T15.4). What the backfill added, by course, each row with Undo through
  // the console's own writers. Every string from a message goes through h(); no usage event is sent from here. ----
  var MA = { rows: [], note: "" };
  function maRow(x, kind) {
    var i = MA.rows.push({ kind: kind, row: x }) - 1, main, from = "";
    if (kind === "changed") {
      main = h(x.title) + " · " + h(x.field) + ": " + h(x.old == null ? "none" : x.old) + " → " + h(x.new == null ? "none" : x.new);
      from = [x.sender ? h(x.sender) : "", x.date ? h(x.date) : ""].filter(function (t) { return t; }).join(" · ");
    } else if (kind === "notice") {
      main = h(x.title) + (x.opened_at ? " · " + h(x.opened_at) : "");
    } else {
      main = h(x.title) + (x.due ? " · due " + h(x.due) : "");
    }
    return '<div class="ma-row"><span class="ma-main">' + main + "</span>" + (from ? '<span class="ma-from">' + from + "</span>" : "") +
      (x.changed_since ? '<span class="ma-since">changed since</span>' : '<button class="b" data-ma-undo="' + i + '">Undo</button>') + "</div>";
  }
  function maSection(title, rows, kind) {
    return rows.length ? "<h4>" + h(title) + "</h4>" + rows.map(function (x) { return maRow(x, kind); }).join("") : "";
  }
  function renderMailApplied(r) {
    MA.rows = [];
    var html = "";
    if (!r || r.ok === false || !r.courses) {
      EL("ma-body").innerHTML = '<p class="meta">Knowlu could not read what came from your email just now.</p>';
      return;
    }
    r.courses.forEach(function (g) {
      var arch = g.archived.items.length ? "<details><summary>" + h("Past items archived (" + g.archived.count + ")") + "</summary>" +
        g.archived.items.map(function (x) { return maRow(x, "archived"); }).join("") + "</details>" : "";
      html += '<div class="ma-course"><h3>' + h(g.course == null ? "Notices" : g.course) + "</h3>" +
        maSection("Items created", g.created, "created") + maSection("Fields changed", g.changed, "changed") + arch +
        maSection("Notices opened", g.notices, "notice") + "</div>";
    });
    if (!r.courses.length) { html = '<p class="meta">Nothing has come in from your email yet.</p>'; }
    EL("ma-body").innerHTML = (MA.note ? '<p class="meta">' + h(MA.note) + "</p>" : "") + html;
  }
  function loadMailApplied() {
    return invoke("mail_applied", {}).then(renderMailApplied).catch(function () { renderMailApplied(null); });
  }
  function openMailApplied() {
    MA.note = ""; EL("mail-applied").hidden = false; maybeReconsent();
    return loadMailApplied();
  }
  EL("ma-close").addEventListener("click", function () { EL("mail-applied").hidden = true; maybeReconsent(); });
  EL("ma-body").addEventListener("click", function (e) {
    var b = e.target.closest("[data-ma-undo]"); if (!b) { return; }
    var at = MA.rows[Number(b.getAttribute("data-ma-undo"))]; if (!at) { return; }
    var row = at.row, p;
    if (at.kind === "changed") { var fields = {}; fields[row.field] = row.old; p = invoke("set_fields", { view: stateView(), id: row.id, fields: fields }); }
    else if (at.kind === "notice") { p = invoke("close_info", { view: stateView(), id: row.id }); }
    else { p = invoke("delete_note", { view: stateView(), id: row.id }); }
    p.then(function (env) {
      applyEnvelope(env, function (msg) { MA.note = "Could not undo that: " + msg; });
      return loadMailApplied();
    }).catch(function () {});
  });
  EL("mp-applied").addEventListener("click", openMailApplied);
  EL("set-email-applied").addEventListener("click", openMailApplied);
  // ---- end mail applied ----
  function openSettings() {
    EL("settings").hidden = false;
    maybeReconsent(); owedSettings();
    loadPrefs(); loadInterests();
    checkAccount();
    gradesStatus();
    mailSettingsRow();
    invoke("settings_context", {}).then(function (c) {
      current.vaultPath = c.vault; current.version = c.version; current.profileName = c.profile_name;
      current.registryError = c.registry_error || "";
      return invoke("get_settings", {});
    }).then(function (r) { if (r.ok) { renderSettings(r.settings); } }).catch(function () {});
  }
  window.KNOWLU_OPEN_SETTINGS = openSettings;

  EL("settings").addEventListener("click", function (e) {
    if (e.target.closest("#set-close")) { closeSettings(); return; }
    if (e.target.closest("#set-google-connect")) { connectGoogle("gmail"); return; }
    if (e.target.closest("#set-google-reconnect")) { connectGoogle("reconnect"); return; }
    if (e.target.closest("#set-google-retry")) { loadGoogleRow(); return; }
    if (e.target.closest("#set-google-check")) { checkGoogleAgain(); return; }
    if (e.target.closest("#set-google-disconnect-1")) { SET.confirming = true; SET.googleNote = ""; renderGoogleRow(); return; }
    if (e.target.closest("#set-google-disconnect-2")) { disconnectGoogle(); return; }
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
    if (e.target.closest("#set-update-check")) { checkForUpdates(true); return; }
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
    if (e.key === "Escape" && !EL("report").hidden) { EL("report").hidden = true; maybeReconsent(); return; }
    if (e.key === "Escape" && !EL("settings").hidden) { closeSettings(); }
  });
  // ---- C1 Task 17: the issue report (legal note §9). **The text the user reads is the payload** —
  // `report.rs` builds and scrubs it, the textarea shows it, and the send posts exactly what is on
  // screen. Nothing is rebuilt after the user has looked away.
  function openReport() {
    EL("report").hidden = false; maybeReconsent();
    EL("report-note").textContent = "";
    EL("report-text").value = "Loading…";
    invoke("report_preview", { view: stateView() }).then(function (r) {
      EL("report-text").value = r.ok ? r.text : ("could not build the report: " + r.error);
    }).catch(function () { EL("report-text").value = "could not build the report"; });
  }
  window.KNOWLU_OPEN_REPORT = openReport;
  EL("report-cancel").addEventListener("click", function () { EL("report").hidden = true; maybeReconsent(); });
  EL("set-report-go").addEventListener("click", openReport);
  EL("report-send").addEventListener("click", function () {
    EL("report-send").disabled = true;
    // Exactly what is on screen. Rebuilding it here would send something the user never read.
    invoke("report_send", { text: EL("report-text").value }).then(function (r) {
      EL("report-note").textContent = r.ok ? "Sent. Thank you." : r.error;
      EL("report-send").disabled = false;
      if (r.ok) { setTimeout(function () { EL("report").hidden = true; maybeReconsent(); }, 1500); }
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
    EL("set-delete-note").textContent = "This deletes your vault, your backups, your account and everything we hold. It cannot be undone." + mailDeleteSentence();
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
   *  flag and two writes rather than a state field. Without it, two presses on Continue with Google
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
    // L8 S5 (spec 3.4): ask first. The prices appear only on a not-entitled answer (a failed check
    // answers not entitled too, so nobody is left without a way forward); an entitled answer goes
    // straight to finishUpgrade() and never flashes them.
    return checkEntitled().then(function (yes) {
      if (!yes) { EL("up-subscribe").hidden = false; return; }
      return finishUpgrade();
    });
  }
  EL("upgrade").addEventListener("click", function (e) {
    // The same branch the setup shell has, and it matters more here: this window has a working console to
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
      // setup shell's own call carries the same comment for the same reason.
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

  // ---- L4 (plan 4.6): the privacy re-consent panel. A `.setpanel` beside `#upgrade`, and like it the
  // panel never hides today's page. It has no Not now, no Close and no Escape: the way out is Accept,
  // or Settings, where Delete my data stays reachable. `RECONSENT_OWED` is the one owed flag and is
  // set only by a reply that says `needs_accept === true`; a failed or unreachable check never raises
  // the panel (D4). `RECONSENT_BUSY` makes two presses on Accept one command.
  var RECONSENT_OWED = false;
  var RECONSENT_BUSY = false;
  var PRIVACY_ASKED = false;
  var OWED_LINE = "Accept the privacy update to use this";
  // One key, the version the lines describe; a static pin fails unless it is the only key and equals
  // `account.rs`'s PRIVACY_VERSION, so a bump of the constant cannot ship these lines by accident.
  var PRIVACY_CHANGES = {
    "2026-10-20": [
      "It now covers connecting Google Calendar and Gmail, the email you forward to Knowlu (received by Amazon Web Services) and what is kept from it, the Blackboard and mail sign-ins Knowlu keeps on this computer, your grades in your account, daily counts of your AI calls, any free tester access, and the interests you give for campus events.",
      "Until you accept, Knowlu keeps today's page and your calendar up to date, but does not sync, import or read anything else new.",
      "To leave instead, use Settings → Delete my data."
    ]
  };
  /** The attributes of a start control while the update is owed; empty otherwise. */
  function owedAttrs() { return RECONSENT_OWED ? ' disabled title="' + h(OWED_LINE) + '"' : ""; }
  function maybeReconsent() {
    // Two `.setpanel`s never stack: the panel stands down while any other is open and returns on close.
    var other = !EL("settings").hidden || !EL("report").hidden || !EL("mail-progress").hidden ||
      !EL("mail-applied").hidden || !EL("upgrade").hidden;
    if (other || !RECONSENT_OWED) { EL("reconsent").hidden = true; return; }
    EL("reconsent").hidden = false;
  }
  /** Settings while the update is owed: the one line, and the email set-up button locked. The Google
   *  row, the mailbox rows and the grades strip read the flag where they are drawn. */
  function owedSettings() {
    var note = EL("set-owed-note");
    note.textContent = RECONSENT_OWED ? OWED_LINE + "." : "";
    note.hidden = !RECONSENT_OWED;
    EL("set-email-start").disabled = RECONSENT_OWED;
    if (SETM.s) { renderMailBoxes(); }
    if (SET.google) { renderGoogleRow(); }
    drawGrades();
    refreshGradesDrawer();
    if (current.state && current.state.topline) { renderSyncLine(current.state); }
  }
  function fillReconsent(p) {
    // `current` is the build's PRIVACY_VERSION, the table's one key: no fallback to another key.
    var key = p.current;
    var lines = ["Knowlu's privacy policy changed on " + key + "."].concat(PRIVACY_CHANGES[key] || []);
    EL("rc-changes").innerHTML = lines.map(function (l) { return "<p>" + h(l) + "</p>"; }).join("");
  }
  /** One `privacy_status` call per launch, for a signed-in vault only (`checkAccount` also runs on
   *  every `openSettings`). Anything but an exact `true` leaves the panel down. */
  function askPrivacy(s) {
    if (PRIVACY_ASKED || !s || s.needs_account) { return; }
    PRIVACY_ASKED = true;
    return invoke("privacy_status", {}).then(function (p) {
      if (p && p.needs_accept === true) { fillReconsent(p); RECONSENT_OWED = true; owedSettings(); maybeReconsent(); }
    }).catch(function () {
      // The flag never outlives a hidden panel: a throw above clears it and redraws the controls.
      RECONSENT_OWED = false; EL("reconsent").hidden = true;
      try { owedSettings(); } catch (x) { /* nothing more to draw */ }
    });
  }
  function acceptPrivacy() {
    if (RECONSENT_BUSY) { return; }
    RECONSENT_BUSY = true; EL("rc-accept").disabled = true; EL("rc-error").textContent = "";
    invoke("accept_privacy", {}).then(function (r) {
      RECONSENT_BUSY = false; EL("rc-accept").disabled = false;
      if (!r || !r.ok) { EL("rc-error").textContent = (r && r.error) || UNREACHABLE; return; }
      RECONSENT_OWED = false;
      EL("reconsent").hidden = true;
      owedSettings();
      poll();
    }).catch(function () { RECONSENT_BUSY = false; EL("rc-accept").disabled = false; EL("rc-error").textContent = UNREACHABLE; });
  }
  EL("reconsent").addEventListener("click", function (e) {
    var policy = e.target.closest("a.policy");
    if (policy) {
      e.preventDefault();
      invoke("open_policy", { which: policy.getAttribute("data-policy") }).catch(function () {});
      return;
    }
    if (e.target.closest("#rc-accept")) { acceptPrivacy(); return; }
    if (e.target.closest("#rc-settings")) { openSettings(); return; }
  });
  // ---- end L4 re-consent

  // Plan 4a Task 2: the console is one of three things this window can be. `launch_state` says
  // which: a console over a resolved profile, the picker (more than one profile), or the setup
  // shell (none). The console's own listeners are bound only on the console path — a picker window has
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
    // D6: the mail announcement was taken at the old wizard's end and setup has not begun: it begins now, in this window,
    // but only once the privacy answer is in; while an update is owed it waits (the offer marker stays).
    checkAccount().then(function () { return mailFirstOpen(); });
  }

  function renderPicker(l) {
    var ps = l.profiles || [];
    EL("picker").hidden = false;
    EL("setup").hidden = true;
    document.querySelector(".app").hidden = true;
    EL("pick-lede").textContent = ps.length ? "Each profile is its own plan folder and account on this computer." : "No profile on this machine yet.";
    // `setup_unfinished` is the launch state's own flag for a profile whose setup stopped early (F3);
    // the page says so and offers Continue setup, which is the same open. It reads nothing else.
    EL("pick-list").innerHTML = ps.map(function (p) {
      return '<div class="row pick" data-profile="' + h(p.id) + '"><div class="ttl"><span class="a">' + h(p.name) +
        '</span><span class="meta">' + (p.setup_unfinished ? "Setup not finished" : h(p.vault)) + "</span></div>" +
        '<div class="acts"><button class="b pri y" data-open="' + h(p.id) + '">' + (p.setup_unfinished ? "Continue setup" : "Open") + "</button></div></div>";
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
  // account's own copy arrives by signing in, through the setup shell's account step (H11a), with no route
  // and no code to type. `pick_folder` and `restore_vault` have both existed since C1 with no caller;
  // this is the first one. Only two fields of a restored vault's plan are ever read
  // (`finish_or_roll_back` -> `finish_profile_in`: the autostart choice and the local-judgment offer
  // marker), so the rest of this minimal plan is never looked at — the fields still have to be the
  // shapes `WizardPlan` requires to deserialize.
  //
  // M3 (fix round 1): a name and an autostart choice, asked rather than assumed, with the defaults
  // the setup shell makes a folder with ("Knowlu", Start Knowlu with Windows checked). Without a name, a second restore always landed at
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
    // N6 (fix round 2): disabled before the call, the same guard the shell's own Continue has
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
      // all, the one outcome neither the shell's Continue nor pick-adopt's own chain guards
      // against either. Shown and re-enabled here rather than left silent.
      return invoke("open_profile", { id: a.profile.id }).then(function (o) {
        if (o && o.ok === false) { EL("pick-lede").textContent = o.error; go.disabled = false; }
      });
    }).catch(function () { go.disabled = false; });
  });

  // R-P4a-15, the picker's other door: the setup shell, opened from a machine that already has a
  // profile. The shell carries the sign-in commands, so nothing has to be relaunched to get here.
  EL("pick-add").addEventListener("click", function () {
    invoke("launch_state", {}).then(function (l) { startSetup(l || {}); }).catch(function () {});
  });

  // ---- O3 F10: the setup shell. Steps 1 and 2 only (welcome, your account), in this same document.
  // Everything after the account is the onboarding mode's (the rail's other rows are its benefits,
  // static words). The shell holds no vault until the account is signed in: it signs in, makes the
  // folder with its defaults, and relaunches into that profile. It computes nothing and shows only
  // what a command answered.
  //
  // **One string, two languages.** `account::UNREACHABLE` is the first clause of every transport
  // failure the Rust side emits, and this is the page's copy of it. They are pinned to each other by
  // `static_assets.rs::the_unreachable_clause_is_one_string_on_both_sides`, the same way `PRIVACY` is
  // pinned to the site's — because a guard that silently stops matching is worse than no guard.
  var UNREACHABLE = "the account service could not be reached";
  var PRIVACY = "Your tasks and notes live in a plain-text folder on this machine and in your Knowlu account, so every computer you sign in on opens on the same day; our servers keep them encrypted at rest, beside your account, the judgments made for you and what you correct, and none of it is ever sold or shared.";

  // The five screens: welcome, account (the ask), browser (Google is open), code (the emailed code)
  // and done (signed in, the folder made). State lives on `SU` and `renderSetup` is its one painter.
  var SU = null;
  // The rail's one row that changes here, and the two words under it, per screen.
  var SU_RAIL = {
    welcome: { no: "Before you start", left: "About 9 minutes left" },
    account: { no: "Step 1 of 9", left: "About 9 minutes left" },
    browser: { no: "Step 1 of 9", left: "About 9 minutes left" },
    code: { no: "Step 1 of 9", left: "About 9 minutes left" },
    done: { no: "Step 2 of 9", left: "About 8 minutes left" }
  };

  function startSetup(l, at) {
    SU = { l: l || {}, step: at || "welcome", email: "", codeEmail: "", emailOpen: false, busy: false, making: false,
           error: "", codeError: "", doneError: "", folder: "", profileId: "", restore: "" };
    EL("setup").hidden = false;
    EL("picker").hidden = true;
    document.querySelector(".app").hidden = true;
    EL("su-privacy").textContent = PRIVACY;
    renderSetup();
  }

  function suShow(step) {
    SU.step = step;
    renderSetup();
    var head = EL("su-h-" + step);
    if (head) { head.focus(); }
  }

  function renderSetup() {
    ["welcome", "account", "browser", "code", "done"].forEach(function (s) { EL("su-" + s).hidden = SU.step !== s; });
    var r = SU_RAIL[SU.step];
    EL("su-stepno").textContent = r.no;
    EL("su-left").textContent = r.left;
    // The account row: Now while the ask is open, Done with its proof once signed in; the school row
    // is the next one. Nothing else in the rail moves in the shell.
    var rows = {};
    document.querySelectorAll("#su-rail .su-row").forEach(function (li) { rows[li.getAttribute("data-rail")] = li; li.className = "su-row"; });
    var acct = rows.account, done = SU.step === "done";
    if (SU.step !== "welcome") { acct.classList.add(done ? "done" : "now"); }
    if (done) { rows.school.classList.add("next"); }
    acct.querySelector(".su-st").textContent = SU.step === "welcome" ? "" : (done ? "Done" : "Now");
    acct.querySelector(".su-pf").textContent = done ? "Signed in as " + SU.email : "";
    EL("su-email-field").hidden = !SU.emailOpen;
    EL("su-google").disabled = SU.busy;
    EL("su-use-code").disabled = SU.busy;
    EL("su-magic").disabled = SU.busy;
    EL("su-code-go").disabled = SU.busy;
    EL("su-error").textContent = SU.error;
    EL("su-browser-error").textContent = SU.error;
    EL("su-code-error").textContent = SU.codeError;
    EL("su-code-why").textContent = "Knowlu sent a code to " + SU.codeEmail + ".";
    EL("su-signed-as").textContent = SU.email;
    EL("su-folder-line").textContent = SU.making ? "Setting up your plan folder on this computer…"
      : (SU.folder ? "Your plan folder is ready on this computer: " + SU.folder + ". It’s plain text files you can open." : "");
    EL("su-restore-line").textContent = SU.restore;
    EL("su-done-error").textContent = SU.doneError;
    EL("su-continue").disabled = SU.making || !SU.profileId || SU.busy;
    EL("su-retry").hidden = SU.making || !!SU.profileId;
    EL("su-enter").hidden = SU.making || !SU.profileId;
  }

  // M1 (fix round 1): the one place all three of the restore's outcomes are worded, so a student
  // never reads a claim the actual result disagrees with. `restored` is `create_vault`'s own
  // `{notes, records, empty, warnings}` (H11a's `sync::Restored`, C3' Task 9). `empty` alone cannot
  // tell "the account truly has nothing yet" from "the account could not be reached just now" —
  // `restore_into` folds a failed pull into the same `empty: true` so setup never rolls the vault
  // back over a hotel Wi-Fi — so `ok` is what tells the two apart here.
  // N3 (fix round 2): keyed on `ok` (an explicit field `create_vault`'s own envelope carries,
  // C3' Task 9's `sync::Restored.ok`), never on `warnings.length` — a perfectly successful pull can
  // carry warnings too (a refused sync card, an oversize row), and reading those as "could not be
  // reached" was the round 1 bug. The offline sentence says the notes arrive over the next SYNCS,
  // plural — one page at a time, so a large account takes more than one.
  function restoreSentence(restored) {
    if (!restored) { return ""; }
    if (!restored.ok) { return "Your account could not be reached just now; your plan will fill in over the next syncs."; }
    if (!restored.empty) { return "Your account already had a plan, and it just came back."; }
    return "Your account had no plan yet, so this is the first one.";
  }

  // The folder's name: "Knowlu", then "Knowlu 2", "Knowlu 3": the first one no profile on this
  // machine already has. `dest_for` still refuses a folder that exists and its sentence is shown.
  function suName() {
    var taken = {};
    (SU.l.profiles || []).forEach(function (p) { taken[String(p.name).toLowerCase()] = true; });
    var n = "Knowlu", i = 2;
    while (taken[n.toLowerCase()]) { n = "Knowlu " + i; i += 1; }
    return n;
  }

  // The defaults the account step makes a folder with (spec §1.9 A): the system's zone, the checks at
  // 12:00 and 18:00, start with Windows. No school, no feed, no coursework login and no mail offer:
  // each is asked on its own step, after the account.
  function suPlan() {
    return { ics_url: null, personal_calendar: null, ics_validated: false, ics_stored: false,
             personal_calendar_validated: false, personal_calendar_stored: false, google_calendar: false,
             offer_mail: false, timezone: SU.l.tz || "", slots: ["12:00", "18:00"],
             zybooks: false, vhl: false, autostart: true };
  }

  // The account is signed in (by Google or by the code): make the folder with the defaults, which
  // also brings back the account's own copy (H11a), then wait on Continue to relaunch into it.
  function suSignedIn(r) {
    SU.email = r.email || ""; SU.busy = false; SU.error = ""; SU.codeError = "";
    suShow("done");
    suMakeFolder();
  }

  function suMakeFolder() {
    SU.making = true; SU.doneError = ""; renderSetup();
    return invoke("create_vault", { name: suName(), plan: suPlan() }).then(function (r) {
      SU.making = false;
      if (!r.ok) { SU.doneError = r.error; renderSetup(); return; }
      SU.profileId = r.profile.id; SU.folder = r.profile.vault; SU.restore = restoreSentence(r.restored);
      renderSetup();
    }).catch(function () { SU.making = false; SU.doneError = UNREACHABLE; renderSetup(); });
  }

  // Continue relaunches into the new profile (`finish_onboarding`); a refusal is shown and the
  // button comes back.
  function suContinue() {
    if (SU.making || !SU.profileId || SU.busy) { return; }
    SU.busy = true; renderSetup();
    invoke("finish_onboarding", { id: SU.profileId }).then(function (o) {
      if (o && o.ok === false) { SU.busy = false; SU.doneError = o.error; renderSetup(); }
    }).catch(function () { SU.busy = false; renderSetup(); });
  }

  // One box says both things (spec §4.2 step 2: 18 or older, and the terms); every door checks it first.
  function suTicked() {
    if (EL("su-18").checked) { return true; }
    SU.error = "Tick the box to create an account."; renderSetup(); return false;
  }

  // Spec D1. One command: the URL, the loopback port, the verifier and both tokens are Rust's, and
  // the page never sees any of them. **`ageAttested`, not the Rust spelling** (Tauri v2 camel-cases
  // every argument key). The browser round trip can take a minute, so the screen says where to look.
  function suGoogle() {
    if (SU.busy || !suTicked()) { return; }
    SU.busy = true; SU.error = ""; suShow("browser");
    invoke("google_sign_in", { ageAttested: EL("su-18").checked }).then(function (r) {
      SU.busy = false;
      if (!r.ok) { SU.error = r.error; suShow("account"); return; }
      suSignedIn(r);
    }).catch(function () { SU.busy = false; SU.error = UNREACHABLE; suShow("account"); });
  }

  // The emailed code spends the same 20 an hour as the Google button, so a second press while one is
  // in flight is refused, and both doors share the one latch.
  function suSendCode(email, fromCodeStep) {
    if (SU.busy || !suTicked()) { return; }
    SU.busy = true; SU.error = ""; SU.codeError = ""; renderSetup();
    invoke("send_magic_link", { email: email, ageAttested: EL("su-18").checked }).then(function (r) {
      SU.busy = false;
      if (!r.ok) { if (fromCodeStep) { SU.codeError = r.error; renderSetup(); } else { SU.error = r.error; renderSetup(); } return; }
      SU.codeEmail = email; EL("su-code-in").value = "";
      suShow("code");
    }).catch(function () { SU.busy = false; SU.error = UNREACHABLE; renderSetup(); });
  }

  function suVerify() {
    if (SU.busy) { return; }
    SU.busy = true; SU.codeError = ""; renderSetup();
    invoke("verify_email_code", { email: SU.codeEmail, code: EL("su-code-in").value }).then(function (r) {
      SU.busy = false; EL("su-code-in").value = "";
      if (!r.ok) { SU.codeError = r.error; renderSetup(); return; }
      suSignedIn(r);
    }).catch(function () { SU.busy = false; SU.codeError = UNREACHABLE; renderSetup(); });
  }

  EL("setup").addEventListener("click", function (e) {
    // **The two policies open in the system browser, not in this window.** `app/static/` holds four
    // files, so a plain navigation to `terms.html` loses the only window Knowlu has — while the user
    // is being asked to tick a box saying they accept it. The `href` stays because it is the honest
    // markup and the static test reads it; this is what actually happens.
    var policy = e.target.closest("a.policy");
    if (policy) { e.preventDefault(); invoke("open_policy", { which: policy.getAttribute("data-policy") }).catch(function () {}); return; }
    if (e.target.closest("#su-start") || e.target.closest("#su-returning")) { suShow("account"); return; }
    if (e.target.closest("#su-google")) { suGoogle(); return; }
    if (e.target.closest("#su-use-code")) { SU.emailOpen = true; renderSetup(); EL("su-email").focus(); return; }
    if (e.target.closest("#su-magic")) { suSendCode(EL("su-email").value.trim(), false); return; }
    if (e.target.closest("#su-code-go")) { suVerify(); return; }
    if (e.target.closest("#su-resend")) { suSendCode(SU.codeEmail, true); return; }
    if (e.target.closest("#su-other-email")) { SU.emailOpen = true; SU.codeError = ""; suShow("account"); EL("su-email").focus(); return; }
    if (e.target.closest("#su-retry")) { suMakeFolder(); return; }
    if (e.target.closest("#su-continue")) { suContinue(); }
  });
  // Enter does what the screen's primary button does, from a field or from nowhere in particular.
  EL("setup").addEventListener("keydown", function (e) {
    if (e.key !== "Enter" || e.target.closest("button, a")) { return; }
    if (e.target.id === "su-email") { e.preventDefault(); suSendCode(EL("su-email").value.trim(), false); }
    else if (e.target.id === "su-code-in") { e.preventDefault(); suVerify(); }
    else if (SU.step === "done") { e.preventDefault(); suContinue(); }
  });

  // The one seam the headless checks use (S13). Nothing in the app calls these; they exist so a
  // Playwright page can put the setup shell, the picker or the settings overlay on screen without a
  // Tauri backend. `openSettings` is the console's own function, unchanged — with no backend its
  // two invokes reject and the rows keep their static markup, which is exactly the layout
  // `console-shots.py` is looking at; `settings-check.py` is the one that drives them with answers.
  window.KNOWLU_SHOTS = { startSetup: startSetup, renderPicker: renderPicker, openSettings: openSettings, openReport: openReport };

  invoke("launch_state", {}).then(function (l) {
    if (l && l.mode === "console") { bootConsole(); return; }
    if (l && l.mode === "wizard") { startSetup(l); return; }
    renderPicker(l || { profiles: [] });
  }).catch(function () { bootConsole(); });
})();
