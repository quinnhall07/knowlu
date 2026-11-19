/* Knowlu app: Report a problem (note §5 "Report a problem"; legal note §9; plan §5 T20). Two parts:
     the row     Data and privacy's Report a problem, registered as the Settings section `report`
     the sheet   `report` (KnowluSheets.define), also what the tray's KNOWLU_OPEN_REPORT requests (router.js)
   The text the student reads is the payload: report.rs builds and scrubs it (report_preview, with the page's view), the
   sheet shows it in an editable box, and Send posts exactly what is in the box (report_send). Nothing is rebuilt after
   the student has looked away. A preview that could not be built says so, leaves Send off (nothing unseen is sent) and
   offers Try again, which asks for the preview again. */
(function () {
  "use strict";

  var COPY = "This is exactly what will be sent. Emails, links and long keys are already replaced. Edit anything you do not want to send, and add what went wrong at the bottom.";
  var REPORT = { root: null, api: null, seq: 0 };
  var SHEET = { render: renderReport, close: onReportClosed };

  function el(id) { return document.getElementById(id); }
  function note(text) {
    var n = el("report-note");
    if (n) { n.textContent = text || ""; }
  }

  /* ---- the row ---- */
  function rowHtml() {
    return '<li class="st-row" id="set-report"><div class="st-l"><p class="st-n">Report a problem</p><p class="st-hint">You read and can edit the whole report before anything is sent.</p></div>' +
      '<div class="st-c"><button class="obtn" type="button" id="set-report-go">Write a report</button></div></li>';
  }

  /* ---- the sheet ---- */
  function reportHtml() {
    return '<div class="sheet-in wide sh rp" aria-labelledby="report-h"><h2 class="sh-h" id="report-h" tabindex="-1">Report a problem</h2>' +
      '<p class="sh-p">' + h(COPY) + "</p>" +
      '<label class="sh-l" for="report-text">What will be sent</label>' +
      '<textarea class="sh-ta" id="report-text" rows="12" spellcheck="false" disabled></textarea>' +
      '<p class="sh-note" id="report-note" role="status"></p>' +
      '<div class="sh-acts"><button class="tbtn decline" type="button" id="report-cancel">Cancel</button><button class="obtn" type="button" id="report-retry" hidden>Try again</button><button class="btn small" type="button" id="report-send" disabled>Send</button></div></div>';
  }
  function renderReport(host, api) {
    REPORT.api = api;
    host.innerHTML = reportHtml();
    REPORT.root = host.querySelector(".rp");
    REPORT.root.addEventListener("click", onReportClick);
    el("report-h").focus({ preventScroll: true });
    openReport();
  }
  function onReportClosed() {
    REPORT.seq += 1;
    REPORT.root = null;
    REPORT.api = null;
  }
  function openReport() {
    REPORT.seq += 1;
    var seq = REPORT.seq;
    var ta = el("report-text");
    note("");
    el("report-retry").hidden = true;
    ta.value = "Loading…";
    KnowluData.call("report_preview", {}).then(function (r) {
      if (seq !== REPORT.seq || !REPORT.root) { return; }
      if (!r || !r.ok) {
        ta.value = "";
        note("Knowlu could not build the report: " + KnowluKit.refusal(r));
        el("report-retry").hidden = false;
        return;
      }
      ta.value = r.text;
      ta.disabled = false;
      el("report-send").disabled = false;
    });
  }
  function sendReport() {
    var btn = el("report-send");
    if (!btn || btn.disabled) { return; }
    btn.disabled = true;
    var seq = REPORT.seq;
    KnowluData.call("report_send", { text: el("report-text").value }).then(function (r) {
      if (seq !== REPORT.seq || !REPORT.root) { return; }
      if (r && r.ok) {
        note("Sent. Thank you.");
        setTimeout(function () { if (seq === REPORT.seq) { KnowluSheets.close("report"); } }, 1500);
        return;
      }
      btn.disabled = false;
      note(KnowluKit.refusal(r));
    });
  }
  function onReportClick(e) {
    if (e.target.closest("#report-send")) { sendReport(); return; }
    if (e.target.closest("#report-retry")) { openReport(); return; }
    if (e.target.closest("#report-cancel") && REPORT.api) { REPORT.api.close(); }
  }

  KnowluSheets.define("report", SHEET);

  KnowluRouter.addSection("report", {
    mount: function (host) {
      host.innerHTML = rowHtml();
      host.querySelector("#set-report-go").addEventListener("click", function () { KnowluSheets.request("report"); });
    }
  });
})();
