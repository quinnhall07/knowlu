/* Knowlu app: the update line (note §5 "Update offer"; plan §5 T19). Two places, one state:
     #top-note            the top band's one short line (plan §3): "Knowlu 0.4.1 is ready." with Restart to update,
                          then the install's own messages (installing, waiting for a running slot, a failure with Try
                          again, restart to finish). Empty while nothing is staged; the shell hides it then.
     the Updates row      This computer's row, registered as the Settings section `updates`: the version, the last
                          check and its outcome, with Check now.
   One check at launch (KnowluBoot.onConsole), quiet; Check now is the manual one. The page renders the updater's answer
   and decides nothing: `staged` is already held back in Rust while a slot runs (updates::update_offer), and the release
   host lives in app/tauri.conf.json, never here. The startup check and the housekeeping thread share one flight, so a
   check that lost it asks again in 10 s, up to six times, on one timer; every check clears the updater's action error,
   so no retry runs once an install was pressed or while the line shows an install message. The two refusal words
   are updates.rs's (ALREADY_CHECKING exactly, SLOT_RUNNING by its start). */
(function () {
  "use strict";

  /* installing: an install is in flight; attempted: one was ever pressed; note: the line shows an install message;
     retries and timer: the lost-flight retry; text: the Updates row's line from the last check */
  var UPD = { installing: false, attempted: false, note: false, retries: 0, timer: null, text: "" };
  var ALREADY_CHECKING = "a check is already running";
  var ROW = { host: null, api: null };

  function el(id) { return document.getElementById(id); }
  function firstLine(s) { return String(s == null ? "" : s).split("\n")[0]; }
  /* a local time from the updater's UTC stamp; "" when it does not parse */
  function localHM(iso) {
    var d = new Date(iso);
    if (!iso || isNaN(d.getTime())) { return ""; }
    return KnowluFmt.clock(d);
  }
  function setUpdateState(text) {
    var s = el("set-update-state");
    if (s) { s.textContent = text; }
  }
  function idleState() {
    if (UPD.text) { return UPD.text; }
    var c = ROW.api ? ROW.api.context() : null;
    return c && c.ok && c.version ? "version " + c.version + " — not checked yet" : "not checked yet";
  }

  /* ---- #top-note ---- */
  function noteHtml(text, action) {
    var actHtml = "";
    if (action) {
      actHtml = '<button class="tbtn" type="button" data-install>' + h(action) + "</button>";
    }
    return '<span class="tn-say">' + h(text) + "</span>" + actHtml;
  }
  function say(text, action) {
    var n = el("top-note");
    if (!n) { return; }
    n.innerHTML = noteHtml(text, action);
    UPD.note = true;
  }

  /* The Updates row's line for one finished check: every outcome says something. Whether a retry follows decides what
     it may promise (after an install attempt none does). */
  function updateOutcome(u, retrying) {
    if (u.error) {
      if (u.error === ALREADY_CHECKING) { return retrying ? "a check is already running — the result will show here" : "a check is already running — try again shortly"; }
      return "could not check — " + firstLine(u.error);
    }
    if (u.staged) { return u.staged + " is available"; }
    if (u.held) { return u.held + " is staged; installs after the run"; }
    return "up to date (checked " + (localHM(u.last_check) || KnowluFmt.clock(new Date())) + ")";
  }

  /* M8: version, last check, staged or not, the check's error; the last action's outcome rides as its own suffix so a
     failed install stays legible beside "up to date". `held` is a bundle the running slot holds back (R-P4a-24): it is
     on this machine, so it is never called up to date and never offered. Mid-install the line belongs to installUpdate,
     and a quiet check leaves an install message alone. A failed check renders like any other: every reply carries the
     updater's whole state (commands.rs), so the row names the failure (last_error) and the top band follows the reply's
     staged, which already says whether anything is still offered. */
  function renderUpdateOffer(u, quiet) {
    var when = localHM(u.last_check) ? ", checked " + localHM(u.last_check) : "";
    var what = u.staged ? " — " + u.staged + " ready" : u.held ? " — " + u.held + " staged; installs after the run" : u.last_error ? " — " + firstLine(u.last_error) : " — up to date";
    var act = u.action_error ? " · last action: " + firstLine(u.action_error) : "";
    if (u.version) { UPD.text = "version " + u.version + what + when + act; }
    if (UPD.installing || (quiet && UPD.note)) { return; }
    UPD.note = false;
    var n = el("top-note");
    if (!n) { return; }
    n.innerHTML = u.staged ? noteHtml("Knowlu " + u.staged + " is ready.", "Restart to update") : "";
  }

  /* `manual` is Check now: the row says checking, then the outcome. The quiet form (launch, the retry) refreshes the
     row only with the version line. Either way a staged update raises the same line in the top band. */
  function checkForUpdates(manual) {
    var btn = el("set-update-check");
    clearTimeout(UPD.timer);
    if (manual === true) {
      UPD.retries = 0;
      if (btn) { btn.disabled = true; }
      setUpdateState("checking…");
    }
    return KnowluData.call("check_for_updates", {}).then(function (u) {
      u = u || {};
      renderUpdateOffer(u, manual !== true);
      var retry = u.error === ALREADY_CHECKING && UPD.retries < 6 && !UPD.attempted && !UPD.note;
      if (manual === true) { setUpdateState(updateOutcome(u, retry)); }
      else { setUpdateState(idleState()); }
      if (retry) {
        UPD.retries++;
        clearTimeout(UPD.timer);
        UPD.timer = setTimeout(function () { if (!UPD.attempted && !UPD.note && !UPD.installing) { checkForUpdates(); } }, 10000);
      }
      var again = el("set-update-check");
      if (manual === true && again) { again.disabled = false; }
    });
  }

  /* Restart to update: disabled and installing on the first press. A running slot (often, just after launch) is not a
     failure: the line says it is waiting and the install is retried every 5 s for up to 5 minutes. Any other refusal
     stays on the line with Try again. On Windows a good install exits the process; where it does not, the line says
     to restart. */
  function installUpdate(btn) {
    if (UPD.installing) { return; }
    var waits = 0;
    function fail(msg) {
      UPD.installing = false;
      say("update failed — " + msg, "Try again");
      KnowluKit.toast("Update failed — " + msg, null, null, true);
    }
    function attempt() {
      KnowluData.call("install_update", {}).then(function (r) {
        if (r && r.ok) {
          setTimeout(function () { UPD.installing = false; say("restart Knowlu to finish the update", ""); }, 15000);
          return;
        }
        var msg = firstLine((r && r.error) || "refused");
        if (msg.indexOf("a slot is running") === 0 && waits < 60) {
          if (waits === 0) { say("waiting for the current sync to finish — Knowlu will install the update when it ends", ""); }
          waits++;
          setTimeout(attempt, 5000);
          return;
        }
        fail(msg);
      });
    }
    UPD.attempted = true;
    UPD.installing = true;
    UPD.note = false;
    clearTimeout(UPD.timer);
    btn.disabled = true;
    btn.textContent = "Installing…";
    attempt();
  }

  /* ---- the Updates row ---- */
  function rowHtml() {
    return '<li class="st-row" id="set-updates"><div class="st-l"><p class="st-n">Updates</p><p class="st-hint sx-line" id="set-update-state" role="status"></p></div>' +
      '<div class="st-c"><button class="obtn" type="button" id="set-update-check">Check now</button></div></li>';
  }

  var top = document.getElementById("top-note");
  if (top) {
    top.addEventListener("click", function (e) {
      var ins = e.target.closest("[data-install]");
      if (ins && !ins.disabled) { installUpdate(ins); }
    });
  }

  KnowluRouter.addSection("updates", {
    mount: function (host, api) {
      ROW.host = host;
      ROW.api = api || null;
      host.innerHTML = rowHtml();
      setUpdateState(idleState());
      host.querySelector("#set-update-check").addEventListener("click", function (e) {
        if (!e.currentTarget.disabled) { checkForUpdates(true); }
      });
    },
    context: function () { if (ROW.host && !UPD.text) { setUpdateState(idleState()); } },
    unmount: function () { ROW.host = null; ROW.api = null; }
  });

  KnowluBoot.onConsole(function () { checkForUpdates(); });
})();
