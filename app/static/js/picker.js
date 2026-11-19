/* Knowlu app: the picker (v0.4.0 note §6; plan §5 T21, T22 folded in). When launch_state answers mode "picker" (more
   than one profile on this machine, or none), boot.js calls window.KnowluPicker.render(launch, host): the profiles
   listed with Open, and three doors: an existing vault, a local backup folder, and a new vault (the same wizard, in
   place). It runs under run_shell and calls only shell commands, through its own invoke() over KnowluData.call.
   Rows are keyed by data-profile, never data-id. */
(function () {
  "use strict";

  var PICKER_HTML =
    '<section class="pick" id="picker">' +
    '<div class="pick-in">' +
    '<h1 class="wm pick-wm">Know<b>lu</b></h1>' +
    '<p class="pick-lede" id="pick-lede"></p>' +
    '<div class="pick-list" id="pick-list"></div>' +
    '<div class="pick-doors">' +
    '<button class="btn" type="button" id="pick-add">Create a new vault…</button>' +
    '<button class="wbtn" type="button" id="pick-adopt">Use an existing vault…</button>' +
    '<button class="wbtn" type="button" id="pick-restore-backup" aria-describedby="pick-restore-why">Restore from a backup folder…</button>' +
    '</div>' +
    '<p class="pick-why" id="pick-restore-why">This restores from a local Backups folder on this device, never from your account &mdash; sign in instead to bring back your account&rsquo;s vault.</p>' +
    '<div class="pick-restore" id="pick-restore-options" hidden>' +
    '<input class="field" type="text" id="pick-restore-name" placeholder="Knowlu" aria-label="Name">' +
    '<label class="wiz-box"><input type="checkbox" id="pick-restore-autostart" checked><span>Start Knowlu with Windows</span></label>' +
    '<button class="ibtn" type="button" id="pick-restore-go">Restore</button>' +
    '</div>' +
    '</div>' +
    '</section>';

  var HOST = null;
  var PICK_RESTORE_BACKUP = "";

  function EL(id) { return document.getElementById(id); }

  // The one way the picker reaches the host, as the wizard's: an unreachable host's answer becomes a rejection.
  function invoke(cmd, args) {
    return KnowluData.call(cmd, args || {}).then(function (v) {
      var answered = KnowluData.last(cmd) === v;
      if (!answered && v && v.ok === false && v.state === null && Object.keys(v).length === 3) { throw new Error(v.error); }
      return v;
    });
  }

  // The id of a row's name, which its Open points at: the row index and the profile id reduced to one token, so it is
  // unique and a single entry in aria-describedby's list of ids.
  function nameId(p, i) {
    return "pick-name-" + i + "-" + String(p.id).replace(/[^A-Za-z0-9_-]/g, "_");
  }

  function rowHtml(p, i) {
    return '<div class="pick-row" data-profile="' + h(p.id) + '"><div class="pick-t"><span class="pick-name" id="' + h(nameId(p, i)) + '">' + h(p.name) +
      '</span><span class="pick-path">' + h(p.vault) + '</span></div>' +
      '<button class="ibtn pick-open" type="button" data-open="' + h(p.id) + '" aria-describedby="' + h(nameId(p, i)) + '">Open</button></div>';
  }

  function renderPicker(l, host) {
    HOST = host || EL("boot");
    var ps = (l && l.profiles) || [];
    HOST.innerHTML = PICKER_HTML;
    EL("pick-lede").textContent = ps.length ? "Which vault?" : "No profile on this machine yet.";
    EL("pick-list").innerHTML = ps.map(rowHtml).join("");
    bind();
  }

  function bind() {
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

    // The local Backups mirror, never the account: the account's own copy arrives by signing in, at the wizard's
    // Finish. A name and an autostart choice are asked, with the wizard's own defaults; only those two fields of
    // this minimal plan are read for a restored vault, the rest is the shape WizardPlan requires.
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
      // Disabled before the call, re-enabled on every refusal: a double press must not start two restores.
      var go = EL("pick-restore-go");
      go.disabled = true;
      var name = EL("pick-restore-name").value.trim() || "Knowlu";
      var plan = {
        ics_url: null, personal_calendar: null, timezone: "", slots: ["12:00", "18:00"],
        zybooks: false, vhl: false, autostart: EL("pick-restore-autostart").checked,
      };
      invoke("restore_vault", { backup: PICK_RESTORE_BACKUP, name: name, plan: plan }).then(function (a) {
        if (!a.ok) { EL("pick-lede").textContent = a.error; go.disabled = false; return; }
        // open_profile can itself refuse (the profile vanished, the relaunch failed): shown, and pressable again.
        return invoke("open_profile", { id: a.profile.id }).then(function (o) {
          if (o && o.ok === false) { EL("pick-lede").textContent = o.error; go.disabled = false; }
        });
      }).catch(function () { go.disabled = false; });
    });

    // The picker's other door: the same wizard, in place. The shell carries every wizard command.
    EL("pick-add").addEventListener("click", function () {
      invoke("launch_state", {}).then(function (l) {
        if (window.KnowluWizard && typeof window.KnowluWizard.start === "function") { window.KnowluWizard.start(l || {}, HOST); }
      }).catch(function () {});
    });
  }

  window.KnowluPicker = {
    render: renderPicker
  };
})();
