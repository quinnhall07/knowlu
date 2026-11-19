/* Knowlu app: the Waiting deck, carried into Home's Waiting (v0.4.0 note §4 "Waiting: cards", §5 "Decisions deck";
   plan §5 T15). Home's Waiting (home_aside.js, T8) draws the header and hands its card slot to KnowluSlots.deck(el);
   this module registers the renderer with KnowluSlots.addDeck.

   The deck draws S.decisions.cards only (N1), whole and in the engine's order, one card at a time: Back and Next step
   through every pending card and the position says "2 of 5". A card shows its title, the engine's reason, where it came
   from (a Gmail card says so; only a mail card names a sender, its day and a quote, each through h()), what an
   amendment would change, and Approve and Reject (Accept and Decline on an event card). Snooze, with the engine's
   tomorrow as its date, and the flag (eight categories and free text) live in the card's overflow. A commitment-ask
   card offers Answer, which opens Details (the `card` panel): the decision note and the answer form live there.
   One click commits; a card in flight is marked busy and a second click is ignored; a refusal gives the card back and
   says why. Each decision sends one event, naming the approval by id; none carries a word of the card or the note. */
(function () {
  "use strict";
  var CATS = [["wrong-effort", "Wrong estimate"], ["wrong-course", "Wrong class"], ["duplicate", "Duplicate"], ["should-not-exist", "Shouldn't exist"], ["wrong-tier", "Wrong tier"], ["wrong-date", "Wrong date"], ["wrong-verdict", "Wrong call"], ["other", "Other"]];
  var DAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
  var DAY_NAMES = { mon: "Mon", tue: "Tue", wed: "Wed", thu: "Thu", fri: "Fri", sat: "Sat", sun: "Sun" };
  var FIELD_WORDS = { due: "due date", effort_hours: "estimate", slice_hours: "session length", course: "class", title: "name", importance: "importance", progress: "progress", status: "status" };
  var PREV_SVG = '<svg viewBox="0 0 18 18" aria-hidden="true"><path d="M11 4.5L6.5 9l4.5 4.5"/></svg>';
  var NEXT_SVG = '<svg viewBox="0 0 18 18" aria-hidden="true"><path d="M7 4.5L11.5 9 7 13.5"/></svg>';
  var MORE_SVG = '<svg viewBox="0 0 18 18" aria-hidden="true"><circle cx="4" cy="9" r=".9"/><circle cx="9" cy="9" r=".9"/><circle cx="14" cy="9" r=".9"/></svg>';
  var FLAG_HTML = '<div class="wt-flag" role="group" aria-label="Flag this card"><div class="seg wt-cats">' + CATS.map(catHtml).join("") + "</div>" +
    '<textarea class="field wt-flag-t" rows="3" aria-label="What is wrong" placeholder="What is wrong, in your words"></textarea>' +
    '<div class="det-acts"><button class="tbtn" type="button" data-flag-send disabled>Flag</button><button class="qbtn" type="button" data-flag-cancel>Cancel</button></div></div>';

  var NOTES = Object.create(null);  /* the decision note typed in Details, per card id, while the page is open */
  var BUSY = Object.create(null);   /* the cards with a decision in flight */
  var POS = { id: null, at: 0 };    /* the card on show: its id, and its place when it leaves */
  var HOSTS = [];                   /* the elements the deck draws into */
  var PANEL = null;                 /* Details for a card: {id, body} */
  var PANEL_BOUND = null;

  function own(table, key) { return Object.prototype.hasOwnProperty.call(table, key); }
  function catHtml(cat) {
    return '<button class="obtn" type="button" data-cat="' + h(cat[0]) + '" aria-pressed="false">' + h(cat[1]) + "</button>";
  }
  function cardsOf(st) {
    return st && st.decisions && Array.isArray(st.decisions.cards) ? st.decisions.cards : [];
  }
  function findCard(id) {
    var hit = null;
    cardsOf(KnowluData.state()).forEach(function (card) { if (!hit && card.id === id) { hit = card; } });
    return hit;
  }
  function tomorrowOf(st) {
    return st && st.ahead && Array.isArray(st.ahead.buckets) && st.ahead.buckets[1] ? st.ahead.buckets[1].date : "";
  }
  /* the element showing card `id` inside `host`: the deck's card or Details' body */
  function findIn(host, id) {
    var hit = null;
    if (!host) { return null; }
    host.querySelectorAll(".wt-card, .wt-dt").forEach(function (n) {
      if (!hit && (n.getAttribute("data-id") || n.getAttribute("data-card-id")) === id) { hit = n; }
    });
    return hit;
  }
  function nodesFor(id) {
    var out = [];
    HOSTS.forEach(function (x) { var n = findIn(x.el, id); if (n) { out.push(n); } });
    if (PANEL) { var d = findIn(PANEL.body, id); if (d) { out.push(d); } }
    return out;
  }
  function lock(node, on) {
    node.querySelectorAll("button, input, textarea").forEach(function (b) {
      if (b.matches("[data-deck-prev], [data-deck-next]")) { return; }
      b.disabled = on;
    });
  }

  /* ---- what a card says ---- */
  function verdictLabels(kind) {
    if (kind === "event-accept" || kind === "event-check") { return { pri: "Accept", sec: "Decline" }; }
    return { pri: "Approve", sec: "Reject" };
  }
  /* Gmail D15: a card from Gmail says so, by its source_uid; never a sender, which such a card does not carry */
  function sourceLabelHtml(c) {
    return c.source_uid && String(c.source_uid).indexOf("gmail:") === 0 ? '<p class="wt-src">from Gmail</p>' : "";
  }
  /* Email D21: a mail card names its sender and the mail's day and quotes the evidence; all three are the sender's own
     text, untrusted, and each goes through h() here and nowhere else */
  function mailCardLineHtml(c) {
    if (!c.from && !c.mail_date && !c.evidence) { return ""; }
    var fromHtml = c.from ? h(c.from) : "";
    var dayHtml = c.mail_date ? h(c.mail_date) : "";
    var whoHtml = (fromHtml && dayHtml ? fromHtml + " · " + dayHtml : fromHtml + dayHtml);
    return (whoHtml ? '<p class="mail-from">' + whoHtml + "</p>" : "") + (c.evidence ? '<p class="mail-quote"><q>' + h(c.evidence) + "</q></p>" : "");
  }
  /* the engine's reason is its note's first paragraph; its bold markers are not words */
  function plainHtml(textHtml) { return String(textHtml).replace(/\*\*/g, ""); }
  function valueText(field, v) {
    if (v == null || v === "") { return "None"; }
    if (field === "due") { return KnowluFmt.due(v) || String(v); }
    if ((field === "effort_hours" || field === "slice_hours") && typeof v === "number") { return KnowluFmt.minutes(v * 60); }
    if (field === "progress") { return String(v) + "%"; }
    if (field === "course") {
      var entry = KnowluCourses.match(v);
      return entry ? entry.title || entry.slug : String(v);
    }
    return typeof v === "object" ? JSON.stringify(v) : String(v);
  }
  /* an amendment's changes, as the new value of each field */
  function changeList(c) {
    var out = [];
    var changes = c.changes && typeof c.changes === "object" && !Array.isArray(c.changes) ? c.changes : null;
    if (c.kind !== "amend" || !changes) { return out; }
    Object.keys(changes).forEach(function (field) {
      var chg = changes[field];
      var next = chg && typeof chg === "object" && own(chg, "to") ? chg.to : chg;
      var word = own(FIELD_WORDS, field) ? FIELD_WORDS[field] : field.replace(/_/g, " ");
      out.push({ label: "New " + word, text: valueText(field, next) });
    });
    return out;
  }
  function changeLineHtml(x) {
    return '<p class="wt-chg"><span>' + h(x.label) + "</span>" + h(x.text) + "</p>";
  }
  function changeRowHtml(x) {
    return '<div class="det-r"><dt>' + h(x.label) + "</dt><dd>" + h(x.text) + "</dd></div>";
  }
  function ageText(days) {
    if (!days) { return "Since today"; }
    return days === 1 ? "1 day" : days + " days";
  }

  /* ---- the deck ---- */
  function menuHtml(tomorrow) {
    return '<div class="wt-menu" hidden><div class="wt-snooze"><label class="wt-lbl"><span>Snooze until</span><input class="field wt-date" type="date" data-snooze-date min="' + h(tomorrow) + '" value="' + h(tomorrow) + '"></label>' +
      '<button class="tbtn" type="button" data-verdict="snoozed">Snooze</button></div>' +
      '<div class="wt-menu-acts"><button class="qbtn" type="button" data-flag>Flag</button><button class="qbtn" type="button" data-card>Details</button></div><div data-flag-host></div></div>';
  }
  function cardHtml(c, tomorrow) {
    var labels = verdictLabels(c.kind);
    var priHtml = (c.kind === "commitment-ask" ? '<button class="tbtn" type="button" data-answer-in>Answer&hellip;</button>' : '<button class="tbtn" type="button" data-verdict="approved">' + h(labels.pri) + "</button>");
    return '<article class="wt-card" data-id="' + h(c.id) + '" data-kind="approval" data-det>' +
      '<div class="wt-top"><i class="mk wait" aria-hidden="true"></i><button class="wt-nm" type="button" data-card>' + h(c.title) + "</button></div>" +
      '<div class="wt-body"><p class="wt-why">' + plainHtml(h(c.why || c.kind)) + "</p>" + sourceLabelHtml(c) + changeList(c).map(changeLineHtml).join("") + mailCardLineHtml(c) +
      '<div class="wt-acts">' + priHtml + '<button class="qbtn" type="button" data-verdict="rejected">' + h(labels.sec) + "</button>" +
      '<button class="obtn wt-more" type="button" data-card-more aria-expanded="false" aria-label="More">' + MORE_SVG + "</button></div>" + menuHtml(tomorrow) + "</div></article>";
  }
  function navHtml(at, n) {
    var backHtml = '<button class="obtn" type="button" data-deck-prev aria-label="Back"' + (at === 0 ? " disabled" : "") + ">" + PREV_SVG + "</button>";
    var nextHtml = '<button class="obtn" type="button" data-deck-next aria-label="Next"' + (at === n - 1 ? " disabled" : "") + ">" + NEXT_SVG + "</button>";
    return (n > 1 ? '<div class="wt-nav"><span class="wt-pos">' + h(String(at + 1) + " of " + n) + "</span>" + backHtml + nextHtml + "</div>" : "");
  }
  /* an open flag (the same node, its toggles and its text) and an open overflow survive a redraw of the same card */
  function preserveAcross(el, id, paint) {
    var old = el.querySelector(".wt-card[data-id]");
    var same = !!old && old.getAttribute("data-id") === id;
    var flag = same ? old.querySelector(".wt-flag") : null;
    var menu = same ? old.querySelector(".wt-menu") : null;
    var open = !!menu && !menu.hidden;
    var date = open ? menu.querySelector("[data-snooze-date]").value : "";
    if (flag) { flag.remove(); }
    paint();
    var card = el.querySelector(".wt-card[data-id]");
    if (!card || card.getAttribute("data-id") !== id) { return; }
    if (open) { setMenu(card, true); card.querySelector("[data-snooze-date]").value = date; }
    if (flag) { card.querySelector("[data-flag-host]").appendChild(flag); }
  }
  function drawHost(el) {
    var st = KnowluData.state();
    var cards = cardsOf(st);
    if (!cards.length) {
      el.textContent = "";
      return;
    }
    var at = -1;
    cards.forEach(function (card, i) { if (at < 0 && card.id === POS.id) { at = i; } });
    if (at < 0) { at = Math.min(POS.at, cards.length - 1); }
    POS.at = at;
    POS.id = cards[at].id;
    var id = POS.id;
    var tomorrow = tomorrowOf(st);
    preserveAcross(el, id, function () {
      el.innerHTML = '<div class="wt">' + cardHtml(cards[at], tomorrow) + navHtml(at, cards.length) + "</div>";
    });
    var card = findIn(el, id);
    if (card && BUSY[id]) { card.dataset.busy = "1"; card.classList.add("gone"); lock(card, true); }
    KnowluKit.watchSeen();
  }
  function drawAll() {
    HOSTS = HOSTS.filter(function (x) { return x.el.isConnected; });
    HOSTS.forEach(function (x) { drawHost(x.el); });
  }
  function step(el, by) {
    var cards = cardsOf(KnowluData.state());
    if (!cards.length) { return; }
    POS.at = Math.max(0, Math.min(cards.length - 1, POS.at + by));
    POS.id = cards[POS.at].id;
    drawAll();
    var b = el.querySelector(by < 0 ? "[data-deck-prev]" : "[data-deck-next]");
    var other = el.querySelector(by < 0 ? "[data-deck-next]" : "[data-deck-prev]");
    if (b && !b.disabled) { b.focus(); } else if (other) { other.focus(); }
  }
  function setMenu(card, on) {
    var menu = card.querySelector(".wt-menu");
    var b = card.querySelector("[data-card-more]");
    menu.hidden = !on;
    b.setAttribute("aria-expanded", on ? "true" : "false");
  }

  /* ---- deciding ---- */
  function decideCard(id, verdict, snoozeUntil, host) {
    if (BUSY[id]) { return; }   /* in flight from the deck or from its Details: one decision per card */
    var card = findIn(host, id);
    if (card && card.dataset.busy) { return; }
    if (verdict === "snoozed" && !snoozeUntil) {
      KnowluKit.toast("Choose a date to snooze it until", null, null, true);
      return;
    }
    var note = own(NOTES, id) ? NOTES[id] : "";
    BUSY[id] = 1;
    if (card) {
      card.dataset.busy = "1";
      card.classList.add("gone");
      lock(card, true);
    }
    nodesFor(id).forEach(function (n) { n.dataset.busy = "1"; lock(n, true); });
    KnowluData.call("decide", { id: id, verdict: verdict, note: note, snoozeUntil: snoozeUntil || null }).then(function (env) {
      delete BUSY[id];
      nodesFor(id).concat(card ? [card] : []).forEach(function (card) {
        delete card.dataset.busy;
        card.classList.remove("gone");
        lock(card, false);
      });
      if (env && env.ok) {
        ev(verdict === "snoozed" ? "decision_deferred" : "decision_made", id, "approval");
        delete NOTES[id];
        if (PANEL && PANEL.id === id) { KnowluPanel.close(); }
        return;
      }
      KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
    });
  }

  /* ---- the flag ---- */
  function openFlag(card) {
    var host = card.querySelector("[data-flag-host]");
    if (!host || host.querySelector(".wt-flag")) { return; }
    host.innerHTML = FLAG_HTML;
    var first = host.querySelector("[data-cat]");
    if (first) { first.focus(); }
  }
  function sendFlag(card) {
    var pop = card.querySelector(".wt-flag");
    if (!pop) { return; }
    var id = card.getAttribute("data-id");
    var cats = Array.prototype.map.call(pop.querySelectorAll('[data-cat][aria-pressed="true"]'), function (b) { return b.getAttribute("data-cat"); });
    if (!cats.length) { return; }
    var ta = pop.querySelector("textarea");
    var send = pop.querySelector("[data-flag-send]");
    send.disabled = true;
    KnowluData.call("open_issue", { target: id, categories: cats, text: ta.value }).then(function (env) {
      if (env && env.ok) {
        ev("issue_opened", id, "approval");
        pop.remove();
        KnowluKit.toast("Flagged");
        return;
      }
      send.disabled = false;
      KnowluKit.toast(KnowluKit.refusal(env), null, null, true);
    });
  }

  function bindHost(el) {
    el.addEventListener("click", function (e) {
      var t = e.target;
      if (t.closest("[data-deck-prev]")) { step(el, -1); return; }
      if (t.closest("[data-deck-next]")) { step(el, 1); return; }
      var card = t.closest(".wt-card");
      if (!card) { return; }
      if (card && card.dataset.busy) { return; }
      var id = card.getAttribute("data-id");
      if (t.closest("[data-card-more]")) { setMenu(card, card.querySelector(".wt-menu").hidden); return; }
      if (t.closest("[data-answer-in]") || t.closest("[data-card]")) { KnowluPanel.open("card", id); return; }
      if (t.closest("[data-flag]")) { openFlag(card); return; }
      var cat = t.closest("[data-cat]");
      if (cat) {
        cat.setAttribute("aria-pressed", cat.getAttribute("aria-pressed") === "true" ? "false" : "true");
        card.querySelector("[data-flag-send]").disabled = !card.querySelector('.wt-flag [data-cat][aria-pressed="true"]');
        return;
      }
      if (t.closest("[data-flag-cancel]")) { card.querySelector(".wt-flag").remove(); return; }
      if (t.closest("[data-flag-send]")) { sendFlag(card); return; }
      var v = t.closest("[data-verdict]");
      if (!v) { return; }
      var verdict = v.getAttribute("data-verdict");
      decideCard(id, verdict, verdict === "snoozed" ? card.querySelector("[data-snooze-date]").value : null, el);
    });
    el.addEventListener("keydown", function (e) {
      var card = e.target.closest ? e.target.closest(".wt-card") : null;
      if (e.key !== "Escape" || !card || card.querySelector(".wt-menu").hidden) { return; }
      e.stopPropagation();
      setMenu(card, false);
      card.querySelector("[data-card-more]").focus();
    });
  }
  function renderDeck(el) {
    if (!HOSTS.some(function (x) { return x.el === el; })) {
      HOSTS.push({ el: el });
      bindHost(el);
    }
    drawHost(el);
  }

  /* ---- Details for a card: the note, the decision, and a commitment-ask card's answer form ---- */
  function askTimeHtml() {
    var daysHtml = DAYS.map(function (d) { return '<button class="obtn" type="button" data-ask-day="' + h(d) + '" aria-pressed="false">' + h(DAY_NAMES[d]) + "</button>"; }).join("");
    return '<div class="ask-time"><div class="seg ask-days">' + daysHtml + '</div><div class="ask-hours"><input class="field" type="time" data-part="start" aria-label="Starts"><span>to</span><input class="field" type="time" data-part="end" aria-label="Ends"></div></div>';
  }
  function askFormHtml(middleHtml) {
    return '<section class="wt-sec"><h3 class="sec-h">When it meets</h3><div class="ask-times">' + askTimeHtml() + "</div>" +
      '<button class="tbtn ask-more" type="button" data-ask-more>Add another time</button><p class="ask-err" role="alert" hidden></p></section>' + middleHtml +
      '<div class="det-acts"><button class="ibtn" type="button" data-ask-save>Save times</button><button class="qbtn" type="button" data-verdict="rejected">No set times</button>' +
      '<button class="qbtn" type="button" data-verdict="snoozed">Snooze</button></div>';
  }
  function cardPanelHtml(c, tomorrow) {
    var labels = verdictLabels(c.kind);
    var factsHtml = '<dl class="det wt-facts">' + changeList(c).map(changeRowHtml).join("") + '<div class="det-r"><dt>Waiting</dt><dd>' + h(ageText(c.age_days)) + "</dd></div></dl>";
    var middleHtml = '<section class="wt-sec"><h3 class="sec-h">Note</h3><textarea class="field wt-note" data-card-note rows="3" aria-label="Note" placeholder="A note with your decision, if you want one"></textarea>' +
      '<label class="wt-lbl wt-snooze-d"><span>Snooze until</span><input class="field wt-date" type="date" data-snooze-date min="' + h(tomorrow) + '" value="' + h(tomorrow) + '"></label></section>';
    var actsHtml = '<div class="det-acts"><button class="ibtn" type="button" data-verdict="approved">' + h(labels.pri) + '</button><button class="qbtn" type="button" data-verdict="rejected">' + h(labels.sec) + "</button>" +
      '<button class="qbtn" type="button" data-verdict="snoozed">Snooze</button></div>';
    return '<div class="wt-dt" data-card-id="' + h(c.id) + '"><p class="wt-why">' + plainHtml(h(c.why || c.kind)) + "</p>" + sourceLabelHtml(c) + mailCardLineHtml(c) + factsHtml +
      (c.kind === "commitment-ask" ? askFormHtml(middleHtml) : middleHtml + actsHtml) + "</div>";
  }
  function answerAsk(wrap) {
    var id = wrap.getAttribute("data-card-id");
    if (wrap.dataset.busy || BUSY[id]) { return; }
    var say = wrap.querySelector(".ask-err");
    var meets = [];
    wrap.querySelectorAll(".ask-time").forEach(function (row) {
      var days = Array.prototype.map.call(row.querySelectorAll('[data-ask-day][aria-pressed="true"]'), function (b) { return b.getAttribute("data-ask-day"); });
      var start = row.querySelector('[data-part="start"]').value;
      var end = row.querySelector('[data-part="end"]').value;
      if (days.length || start || end) { meets.push({ days: days, start: start, end: end }); }
    });
    if (!meets.length) {
      say.textContent = "Choose the days and times it meets, or No set times.";
      say.hidden = false;
      return;
    }
    say.hidden = true;
    BUSY[id] = 1;
    wrap.dataset.busy = "1";
    lock(wrap, true);
    KnowluData.call("answer_card", { id: id, meets: meets }).then(function (env) {
      delete BUSY[id];
      delete wrap.dataset.busy;
      lock(wrap, false);
      if (env && env.ok) {
        ev("decision_made", id, "approval");
        if (PANEL && PANEL.id === id) { KnowluPanel.close(); }
        KnowluKit.toast("Saved");
        return;
      }
      say.textContent = KnowluKit.refusal(env);
      say.hidden = false;
    });
  }
  function bindPanel(body) {
    body.addEventListener("click", function (e) {
      var wrap = e.target.closest(".wt-dt");
      if (!wrap || !PANEL || wrap.dataset.busy) { return; }
      var t = e.target;
      var day = t.closest("[data-ask-day]");
      if (day) { day.setAttribute("aria-pressed", day.getAttribute("aria-pressed") === "true" ? "false" : "true"); return; }
      if (t.closest("[data-ask-more]")) {
        var box = document.createElement("div");
        box.innerHTML = askTimeHtml();
        wrap.querySelector(".ask-times").appendChild(box.firstChild);
        return;
      }
      if (t.closest("[data-ask-save]")) { answerAsk(wrap); return; }
      var v = t.closest("[data-verdict]");
      if (!v) { return; }
      var verdict = v.getAttribute("data-verdict");
      decideCard(PANEL.id, verdict, verdict === "snoozed" ? wrap.querySelector("[data-snooze-date]").value : null, body);
    });
    body.addEventListener("input", function (e) {
      var ta = e.target.closest("[data-card-note]");
      if (!ta || !PANEL) { return; }
      var id = PANEL.id;
      NOTES[id] = ta.value;
    });
  }

  KnowluData.on(function (st) {
    HOSTS = HOSTS.filter(function (x) { return x.el.isConnected; });
    HOSTS.forEach(function (x) { drawHost(x.el); });
    if (PANEL && !findCard(PANEL.id)) { KnowluPanel.close(); }
  });

  KnowluPanel.add("card", {
    render: function (body, id, api) {
      var card = findCard(id);
      PANEL = { id: id, body: body };
      if (PANEL_BOUND !== body) { bindPanel(body); PANEL_BOUND = body; }
      api.check(null);
      if (!card) {
        api.title("");
        body.textContent = "";
        return;
      }
      api.title(card.title);
      body.innerHTML = cardPanelHtml(card, tomorrowOf(KnowluData.state()));
      var ta = body.querySelector("[data-card-note]");
      if (ta && own(NOTES, id)) { ta.value = NOTES[id]; }
      if (BUSY[id]) { var wrap = findIn(body, id); wrap.dataset.busy = "1"; lock(wrap, true); }
    },
    close: function () {
      PANEL = null;
    }
  });

  KnowluSlots.addDeck(renderDeck);
})();
