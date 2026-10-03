// The mail window's initialisation script (email-forwarding T0.S3, plan T9.2). It runs before the
// page's own scripts in every document of the mail window and its popup. Outlook and Gmail each ask
// to become the handler for email links; that call is answered here and does nothing, so the
// browser's "open email links" prompt never shows. The host also cancels any launch of an external
// scheme. Nothing here reads the page, and nothing is logged.
(function () {
  try {
    var noop = function () {};
    if (typeof Navigator !== "undefined" && Navigator.prototype) {
      Object.defineProperty(Navigator.prototype, "registerProtocolHandler", { value: noop, configurable: false, writable: false });
    }
  } catch (e) {
    // A page that froze the prototype first keeps its own; the host-side cancel still holds.
  }
})();
