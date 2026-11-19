/* Knowlu app: Status (note §4 Settings "Status"; plan §5 T19). Registered as the Settings section `status`
   (KnowluRouter.addSection), drawn before This computer. Everything here is the read model's, never computed:
     Sync      topline.sync, the engine's own SyncStatus, with Sync now (a start control: it locks while the privacy
               update is owed, KnowluKit.owed / owedHtml, and the section redraws when that flag changes)
     Backup    topline.backup (Back up now is This computer's, drawn by Settings itself)
     Schedule  topline.last_slot, topline.startup_missed and topline.scheduler, as the app attaches them
     Flagged        issues_panel.rows, each with a Resolve box (resolve_issue, the student's own words)
     Warnings       runs_panel.warnings and the journal's S.warnings, then every note in S.unreadable
     Runs           runs_panel.expected and runs_panel.recent with each run's whole summary, else its empty text
   The sources in trouble (issues_panel.sources) are drawn once on the page, under Sources (sources.js), never here.
   While the privacy update is owed, the section's first line says why Sync now is locked (KnowluKit.OWED_LINE).
   Every topline object may be absent (a fresh vault, a hard-error envelope); nothing here throws on a missing key.
   The skip and error words are the engine's (cloudmodel.rs, sync.rs) and SYNC_SAYS says what a student reads for each;
   ui_status_update.rs reads those sources and fails if a word has no entry. Reads nothing on its own: the state is
   KnowluData's, and update(state) repaints. */
(function () {
  "use strict";

  /* C3' (R-C3'-exec-39 N1 and N4, -43): the engine's word for a skipped or failed sync -> [tone, what a student reads].
     Amber keeps changes on this computer until the student acts or the network returns; calm is ordinary. "no
     entitlement" is also what a paying student reads after more than 72 hours offline, so it says what is known and
     never that the subscription is inactive. */
  var SYNC_SAYS = {
    "no session": ["amber", "signed out — sign in to sync"],
    "signed out": ["amber", "signed out — sign in to sync"],
    "no entitlement": ["amber", "can't confirm your subscription — changes stay on this computer"],
    "offline: the account could not be reached": ["amber", "offline — changes stay on this computer"],
    "no account": ["calm", "sync skipped — no account"],
    "another sync is running": ["calm", "sync skipped — another sync is running"]
  };

  var STATUS = { host: null };
  /* Sync now can take half a minute and a poll repaints the section meanwhile, so busy is a flag the button reads */
  var syncing = false;

  function el(id) { return document.getElementById(id); }
  function cap(s) { s = String(s || ""); return s.charAt(0).toUpperCase() + s.slice(1); }
  function firstLine(s) { return String(s == null ? "" : s).split("\n")[0]; }

  /* a local time from a stamp (the engine and the app write UTC, or an offset); "" when it does not parse */
  function localHM(iso) {
    var d = new Date(iso);
    if (!iso || isNaN(d.getTime())) { return ""; }
    return KnowluFmt.clock(d);
  }
  /* "Today, 7:01 AM" from a read-model stamp; the stamp itself when the formatter cannot read it */
  function stamp(x) {
    var p = KnowluFmt.parse(x);
    if (!p) { return String(x || ""); }
    return KnowluFmt.day(x) + (p.time ? ", " + KnowluFmt.clock(p.d) : "");
  }

  /* one engine word -> [tone, text]; a word the table does not know keeps its first line after prefix, in tone */
  function syncSays(word, tone, prefix) {
    var w = firstLine(word);
    return Object.prototype.hasOwnProperty.call(SYNC_SAYS, w) ? SYNC_SAYS[w] : [tone, (prefix || "") + w];
  }
  /* Sync now's refusal says what the line says, never the raw word */
  function syncRefusal(err) { KnowluKit.toast(cap(syncSays(err, "amber")[1]), null, null, true); }

  function joinHtml(leftHtml, rightHtml) {
    return leftHtml ? leftHtml + " · " + rightHtml : rightHtml;
  }
  /* a line's first word starts with a capital; the rest of the line reads on after a middle dot */
  function addBitHtml(lineHtml, tone, text) {
    return joinHtml(lineHtml, bitHtml(tone, lineHtml ? text : cap(text)));
  }
  function bitHtml(tone, text) {
    return (tone === "amber" ? '<span class="sx-amber">' : '<span class="sx-calm">') + h(text) + "</span>";
  }

  /* ---- the section ---- */
  function sectionHtml() {
    return '<section class="st-sec sx" data-section="status" aria-labelledby="st-h-status"><h2 class="st-h" id="st-h-status" tabindex="-1">Status</h2>' +
      '<p class="st-hint st-warn" id="sx-owed" hidden></p>' +
      '<ul class="st-rows">' +
      '<li class="st-row"><div class="st-l"><p class="st-n">Sync</p><p class="st-hint sx-line" id="sx-sync"></p></div><div class="st-c" id="sx-syncctl"></div></li>' +
      '<li class="st-row"><div class="st-l"><p class="st-n">Backup</p><p class="st-hint sx-line" id="sx-backup"></p></div></li>' +
      '<li class="st-row"><div class="st-l"><p class="st-n">Schedule</p><p class="st-hint sx-line" id="sx-slot"></p></div></li>' +
      "</ul>" +
      '<div class="sx-part"><p class="st-sub">Flagged issues</p><ul class="st-rows" id="sx-issues"></ul></div>' +
      '<div class="sx-part" id="sx-warns" hidden></div>' +
      '<div class="sx-part"><p class="st-sub">Runs</p><ul class="st-rows" id="sx-runs"></ul></div>' +
      "</section>";
  }
  function syncButtonHtml() {
    return KnowluKit.owedHtml('<button class="obtn" type="button" data-sync data-start' + KnowluKit.owed() + (syncing ? " disabled" : "") + ">" + (syncing ? "Syncing…" : "Sync now") + "</button>");
  }

  /* Fix round 1, I1: SyncStatus::of stamps `at` on every run, a skipped one too, so s.skipped is read before s.at, or a
     lapsed subscription or a signed-out machine reads as in step for as long as the state lasts. */
  function renderSyncLine(st) {
    var t = (st && st.topline) || {}, s = t.sync || {}, b = t.backup || {};
    var syncHtml = "";
    var backupHtml = "";
    var slotHtml = "";
    var said = null;
    if (s.last_error) {
      said = syncSays(s.last_error, "amber");
      syncHtml = addBitHtml(syncHtml, said[0], said[1]);
    } else if (s.skipped) {
      /* a skip word the table does not know is ordinary until it is added, so calm */
      said = syncSays(s.skipped, "calm", "sync skipped — ");
      syncHtml = addBitHtml(syncHtml, said[0], said[1]);
    } else if (s.at) {
      syncHtml = addBitHtml(syncHtml, "calm", "in step with your account");
    } else {
      syncHtml = addBitHtml(syncHtml, "calm", "not synced yet");
    }
    /* `at` is stamped on every run, so a skip or an error says tried, never synced */
    if (s.at && localHM(s.at)) { syncHtml = addBitHtml(syncHtml, "calm", (s.last_error || s.skipped ? "tried " : "synced ") + localHM(s.at)); }
    if (b.last_error) { backupHtml = addBitHtml(backupHtml, "amber", "backup: " + firstLine(b.last_error)); }
    else if (b.behind_days != null && b.behind_days >= 1) { backupHtml = addBitHtml(backupHtml, "amber", "backup " + b.behind_days + " day" + (b.behind_days === 1 ? "" : "s") + " behind"); }
    else if (b.last_ok) { backupHtml = addBitHtml(backupHtml, "calm", "backed up"); }
    else { backupHtml = addBitHtml(backupHtml, "calm", "not backed up yet"); }
    /* last_slot may be absent (no slot has run yet). Both stamps can be absent too (the placeholder while another slot
       runs), and a refused slot never ran the engine, so it says refused and why (RunSummary.reason). */
    if (t.last_slot) {
      var when = localHM(t.last_slot.ended || t.last_slot.started || "");
      var why = t.last_slot.reason;
      var tries = t.last_slot.attempts || 0;
      var body = "last slot" + (when ? " " + when : "") + (why ? " refused — " + why : (t.last_slot.ok ? " ok" : " failed")) + (tries > 1 ? " (attempt " + tries + ")" : "");
      slotHtml = addBitHtml(slotHtml, t.last_slot.ok && !why ? "calm" : "amber", body);
    }
    if (t.startup_missed > 0) { slotHtml = addBitHtml(slotHtml, "amber", t.startup_missed + " slot" + (t.startup_missed === 1 ? "" : "s") + " missed while quit"); }
    if (t.scheduler && t.scheduler.mode === "app") { slotHtml = addBitHtml(slotHtml, t.scheduler.paused ? "amber" : "calm", "scheduler " + (t.scheduler.paused ? "paused" : "on")); }
    if (!slotHtml) { slotHtml = addBitHtml(slotHtml, "calm", "no slot has run yet"); }
    el("sx-sync").innerHTML = syncHtml;
    el("sx-backup").innerHTML = backupHtml;
    el("sx-slot").innerHTML = slotHtml;
    drawSyncButton();
  }
  function drawSyncButton() {
    el("sx-syncctl").innerHTML = syncButtonHtml();
  }
  /* note §5, review M5: the lock says why in words, not only in each control's tip */
  function drawOwed() {
    var line = el("sx-owed");
    if (!line) { return; }
    line.hidden = !KnowluKit.isOwed();
    line.textContent = KnowluKit.isOwed() ? KnowluKit.OWED_LINE + "." : "";
  }

  function setSyncBusy(on) {
    syncing = on;
    Array.prototype.forEach.call(document.querySelectorAll("[data-sync]"), function (b) {
      b.disabled = on || KnowluKit.isOwed();
      b.textContent = on ? "Syncing…" : "Sync now";
    });
  }
  /* every outcome says something: the time, the skip in the line's words, the refusal, or that it could not start */
  function runSync() {
    if (syncing) { return; }
    setSyncBusy(true);
    KnowluData.call("sync", {}).then(function (env) {
      setSyncBusy(false);
      ev("sync_run", null, null, null);
      if (!env || (!env.ok && !env.state)) {
        KnowluKit.toast(cap("sync could not start — " + KnowluKit.refusal(env)), null, null, true);
        return;
      }
      if (!env.ok) {
        syncRefusal(env.error);
        return;
      }
      var s = (env.state.topline && env.state.topline.sync) || {};
      if (s.skipped) { KnowluKit.toast(cap(syncSays(s.skipped, "calm", "sync skipped — ")[1])); }
      else { KnowluKit.toast("Synced " + (localHM(s.at) || KnowluFmt.clock(new Date()))); }
    });
  }

  /* ---- what needs a look ---- */
  function issueHtml(r) {
    var meta = [];
    if (r.opened_at) { meta.push("Flagged " + stamp(r.opened_at)); }
    if (r.categories && r.categories.length) { meta.push(r.categories.join(", ")); }
    return '<li class="st-row sx-issue" data-id="' + h(r.id) + '" data-kind="issue"><div class="st-l"><p class="st-n">' + h(r.title) + "</p>" +
      '<p class="st-hint">' + h(meta.join(" · ")) + "</p></div>" +
      '<div class="st-c st-wide"><input class="field" type="text" maxlength="400" data-res="' + h(r.id) + '" aria-label="' + h("Resolution for " + r.title) + '" placeholder="What was wrong">' +
      '<button class="obtn" type="button" data-resolve="' + h(r.id) + '">Resolve</button></div></li>';
  }
  function emptyHtml(text) {
    return '<li class="st-row"><p class="st-hint">' + h(text) + "</p></li>";
  }
  /* The flagged rows (L7 3.6; the sources in trouble are Sources'). A resolution being typed survives a repaint. */
  function renderIssues(st) {
    var p = (st && st.issues_panel) || {};
    var typed = {};
    Array.prototype.forEach.call(STATUS.host.querySelectorAll("[data-res]"), function (i) {
      if (i.value) { typed[i.getAttribute("data-res")] = i.value; }
    });
    var focused = document.activeElement && document.activeElement.getAttribute ? document.activeElement.getAttribute("data-res") : null;
    var rowsHtml = (p.rows || []).map(issueHtml).join("");
    if (!rowsHtml) {
      rowsHtml = emptyHtml(p.empty_text || "No open issues.");
    }
    el("sx-issues").innerHTML = rowsHtml;
    Array.prototype.forEach.call(STATUS.host.querySelectorAll("[data-res]"), function (i) {
      var id = i.getAttribute("data-res");
      if (Object.prototype.hasOwnProperty.call(typed, id)) { i.value = typed[id]; }
      if (id === focused) { i.focus(); }
    });
  }
  function resolveIssue(id, btn) {
    var input = STATUS.host ? STATUS.host.querySelector('[data-res="' + id + '"]') : null;
    var text = input ? input.value.trim() : "";
    if (!text) {
      if (input) { input.focus(); }
      KnowluKit.toast("Say what was wrong first.", null, null, true);
      return;
    }
    btn.disabled = true;
    KnowluData.call("resolve_issue", { id: id, resolution: text }).then(function (r) {
      btn.disabled = false;
      if (r && r.ok) { KnowluKit.toast("Issue resolved"); }
      else { KnowluKit.toast(KnowluKit.refusal(r), null, null, true); }
    });
  }

  /* I1: the journal's warnings and the runs' own, then every note Knowlu could not read */
  function renderWarnings(st) {
    var runs = (st && st.runs_panel) || {};
    var warns = (runs.warnings || []).concat((st && st.warnings) || []);
    var unread = (st && st.unreadable) || [];
    var host = el("sx-warns");
    var listHtml = "";
    warns.forEach(function (w) {
      listHtml += '<li class="st-row"><p class="st-hint sx-warn">' + h(w) + "</p></li>";
    });
    unread.forEach(function (p) {
      listHtml += '<li class="st-row"><p class="st-hint sx-warn">' + "Knowlu could not read " + h(p) + "</p></li>";
    });
    host.hidden = !listHtml;
    if (!listHtml) {
      host.innerHTML = "";
      return;
    }
    host.innerHTML = '<p class="st-sub">Warnings</p><ul class="st-rows">' + listHtml + "</ul>";
  }

  /* every expected slot, then every recent run with its whole summary, else the engine's empty text */
  function expectedHtml(e) {
    var tone = e.status === "seen" ? "calm" : "amber";
    return '<li class="st-row sx-run"><div class="st-l"><p class="st-n">' + h(cap(e.runner)) + '</p><p class="st-hint">' + h("Due " + stamp(e.due)) + "</p></div>" +
      '<div class="st-c">' + bitHtml(tone, cap(e.status)) + "</div></li>";
  }
  function recentHtml(r) {
    var tone = r.result === "ok" || r.result === "running" ? "calm" : "amber";
    return '<li class="st-row sx-run"><div class="st-l"><p class="st-n">' + h(cap(r.runner) + ", " + stamp(r.ended || r.started || "")) + "</p>" +
      '<pre class="sx-sum">' + h(r.summary) + "</pre></div>" +
      '<div class="st-c">' + bitHtml(tone, cap(String(r.result || "").toLowerCase())) + "</div></li>";
  }
  function renderRunsView(st) {
    var p = (st && st.runs_panel) || {};
    var runsHtml = (p.expected || []).map(expectedHtml).join("") + (p.recent || []).map(recentHtml).join("");
    if (p.empty_text) {
      runsHtml += emptyHtml(p.empty_text);
    }
    el("sx-runs").innerHTML = runsHtml;
  }

  function render(st) {
    if (!STATUS.host) { return; }
    st = st || {};
    drawOwed();
    renderSyncLine(st);
    renderIssues(st);
    renderWarnings(st);
    renderRunsView(st);
  }

  function onClick(e) {
    var syncBtn = e.target.closest("[data-sync]");
    if (syncBtn) { if (!syncBtn.disabled) { runSync(); } return; }
    var res = e.target.closest("[data-resolve]");
    if (res && !res.disabled) { resolveIssue(res.getAttribute("data-resolve"), res); }
  }
  function onKey(e) {
    if (e.key !== "Enter") { return; }
    var input = e.target.closest ? e.target.closest("[data-res]") : null;
    if (!input) { return; }
    e.preventDefault();
    var btn = STATUS.host.querySelector('[data-resolve="' + input.getAttribute("data-res") + '"]');
    if (btn && !btn.disabled) { resolveIssue(input.getAttribute("data-res"), btn); }
  }

  KnowluRouter.addSection("status", {
    mount: function (host) {
      STATUS.host = host;
      host.innerHTML = sectionHtml();
      var sec = host.querySelector(".sx");
      sec.addEventListener("click", onClick);
      sec.addEventListener("keydown", onKey);
      render(KnowluData.state());
    },
    update: function (st) { render(st); },
    redraw: function () { if (STATUS.host) { drawSyncButton(); drawOwed(); } },
    unmount: function () { STATUS.host = null; }
  });
})();
