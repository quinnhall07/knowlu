/* Knowlu app: the data layer (note D3, plan §3). The only file that reaches the host: every command goes through
   KnowluData.call, and only this file names window.__TAURI__.core.

   KnowluData.call(cmd, args)  the one invoke wrapper. A command whose signature takes `view` (VIEWED, read from app/src
                               by ui_shell.rs) gets the current page's state view added and its answer's `state` adopted
                               at once, never held as a reorder (F19), as long as the page's view is still the one the
                               call carried (else the page polls its own view). preview_window is a read: its state is
                               never adopted (M4). Always resolves, never rejects: an envelope, or {ok: false, error,
                               state: null} when the host could not be reached. The answer is remembered for last(cmd).
   KnowluData.state()          the state on screen; null before the first answer and during the first run (I4)
   KnowluData.on(fn)           fn(state) on every newly painted revision, and fn(null) on every first-run answer;
                               returns the function that stops it
   KnowluData.firstRun()       the envelope's first_run block while the first run is going, else null
   KnowluData.last(cmd)        the last answer of a command called through here (grades_status and mail_status ride the
                               poll; google_status is read only by Settings when it mounts, P4), else null
   KnowluData.onPoll(fn)       fn({ok, error}) after every poll: once when the state answers (ok: true) or fails
                               (ok: false, with the reason: a rejected call or an envelope with ok: false; the failure is
                               also last("state")), and again when grades_status and mail_status land. The Status dot
                               and any line that says the engine did not answer read it; returns the function that stops it
   KnowluData.refresh()        poll now
   KnowluData.view(v)          the router's: the page's state view ("today" or "all") changed
   KnowluData.start()          boot's, in the console only: poll at launch, on focus and every 60 s (3 s while first_run)

   A changed order waits for the pointer: when a new revision reorders the rows while the pointer is over a list
   ([data-rows]), the new state is held and painted when the pointer leaves that list (R28-R30). */
(function () {
  "use strict";
  var VIEWED = ["answer_card", "backup_now", "capture_registrar", "close_info", "commitments_confirm", "create_task", "decide", "delete_note", "open_issue", "remove_lane_date", "report_preview", "resolve_issue", "set_body", "set_fields", "set_interests", "set_preferences", "sync"];
  var POLL_MS = 60000;
  var FIRST_RUN_MS = 3000;
  var S = null;        /* the state on screen */
  var REV = null;      /* its revision */
  var HELD = null;     /* a newer state waiting for the pointer to leave the list */
  var HELD_ON = null;  /* the list it waits on */
  var FIRST = null;    /* the first_run block, while it lasts */
  var VIEW = "today";  /* the state view the poll asks for */
  var SEQ = 0;
  var FIRST_TIMER = null;
  var STARTED = false;
  var SUBS = [];
  var POLL_SUBS = [];
  var LAST = Object.create(null);

  function invoke(cmd, args) {
    try {
      return Promise.resolve(window.__TAURI__.core.invoke(cmd, args || {}));
    } catch (e) {
      return Promise.reject(e);
    }
  }
  function unreachable(e) { return { ok: false, error: String((e && e.message) || e || "unreachable"), state: null }; }
  function stateView() { return VIEW; }
  function emit(st) {
    SUBS.slice().forEach(function (fn) {
      try { fn(st); } catch (e) { window.console.error(e); }
    });
  }

  /* the ids of the rows a page orders, in order: Must do's groups, then a horizon view's list */
  function orderOf(st) {
    var ids = [];
    if (st && st.must_do && Array.isArray(st.must_do.groups)) {
      st.must_do.groups.forEach(function (g) { (g.rows || []).forEach(function (r) { ids.push(r.id); }); });
    }
    if (st && st.list && Array.isArray(st.list.rows)) { st.list.rows.forEach(function (r) { ids.push(r.id); }); }
    return ids.join(",");
  }

  function paint(st) {
    HELD = null;
    S = st;
    REV = st.revision;
    emit(st);
  }
  function arrived(st) {
    if (REV !== null && st.revision === REV && !HELD) { return; }
    var list = document.querySelector("[data-rows]:hover");
    if (S && list && orderOf(st) !== orderOf(S)) {
      HELD = st;
      if (HELD_ON !== list) {
        HELD_ON = list;
        list.addEventListener("pointerleave", function () { HELD_ON = null; if (HELD) { paint(HELD); } }, { once: true });
      }
      return;
    }
    paint(st);
  }
  function adopt(st) {
    HELD = null;
    paint(st);
  }
  function polled(outcome) {
    POLL_SUBS.slice().forEach(function (fn) {
      try { fn(outcome); } catch (e) { window.console.error(e); }
    });
  }
  /* the state did not answer: record it, tell the hook, and keep the first run's 3 s cadence */
  function pollFailed(error) {
    var why = String((error && error.message) || error || "unreachable");
    LAST.state = { ok: false, error: why, state: null };
    polled({ ok: false, error: why });
    if (FIRST) {
      clearTimeout(FIRST_TIMER);
      FIRST_TIMER = setTimeout(poll, FIRST_RUN_MS);
    }
  }
  function firstRunArrived(block) {
    FIRST = block;
    S = null;
    REV = null;
    HELD = null;
    emit(null);
    clearTimeout(FIRST_TIMER);
    FIRST_TIMER = setTimeout(poll, FIRST_RUN_MS);
  }

  function poll() {
    var seq = ++SEQ, v = stateView();
    return invoke("state", { view: v }).then(function (env) {
      if (seq !== SEQ || v !== stateView()) { return; }
      if (!env || env.ok === false) { pollFailed(env ? env.error : "no answer"); return; }
      LAST.state = env;
      if (env.first_run) { firstRunArrived(env.first_run); polled({ ok: true, error: null }); return; }
      FIRST = null;
      if (env.state) { arrived(env.state); }
      polled({ ok: true, error: null });
      invoke("grades_status", {}).then(function (g) { LAST.grades_status = g; polled({ ok: true, error: null }); }, function () {});
      invoke("mail_status", {}).then(function (m) { LAST.mail_status = m; polled({ ok: true, error: null }); }, function () {});
    }, function (e) {
      if (seq === SEQ) { pollFailed(e); }
    });
  }

  function call(cmd, args) {
    var a = {};
    Object.keys(args || {}).forEach(function (k) { a[k] = args[k]; });
    if (VIEWED.indexOf(cmd) >= 0) { a.view = stateView(); }
    return invoke(cmd, a).then(function (env) {
      LAST[cmd] = env;
      if (env && VIEWED.indexOf(cmd) >= 0 && env.state && typeof env.state === "object") {
        if (a.view === stateView()) { adopt(env.state); } else { poll(); }
      }
      return env;
    }, unreachable);
  }

  window.KnowluData = {
    call: call,
    state: function () { return S; },
    on: function (fn) {
      SUBS.push(fn);
      return function () { var i = SUBS.indexOf(fn); if (i >= 0) { SUBS.splice(i, 1); } };
    },
    onPoll: function (fn) {
      POLL_SUBS.push(fn);
      return function () { var i = POLL_SUBS.indexOf(fn); if (i >= 0) { POLL_SUBS.splice(i, 1); } };
    },
    firstRun: function () {
      return FIRST;
    },
    last: function (cmd) { return Object.prototype.hasOwnProperty.call(LAST, cmd) ? LAST[cmd] : null; },
    refresh: function () { return poll(); },
    view: function (v) {
      var next = v === "all" ? "all" : "today";
      if (next === VIEW) { if (HELD) { paint(HELD); } return; }
      VIEW = next;
      S = null;
      REV = null;
      HELD = null;
      if (STARTED) { poll(); }
    },
    start: function () {
      if (STARTED) { return; }
      STARTED = true;
      window.addEventListener("focus", poll);
      setInterval(poll, POLL_MS);
      poll();
    }
  };
})();
