//! **Debug builds only** (declared under `cfg(debug_assertions)` in `lib.rs`): a diagnostic
//! debugging port on the `mail` window's own WebView2 environment, so a developer can inspect a live
//! mailbox page read-only over CDP (T18c, 2026-10-05). The mail window has its own environment (its
//! per-mailbox data folder), so the process-wide `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` port belongs
//! to the console's environment and never reaches it. Nothing here is in a release build, and the
//! product never drives a page over CDP (spec D5): this is a developer's window onto the page only.

/// The one variable the hook reads: a port number, nothing else.
pub const MAIL_PORT_VAR: &str = "KNOWLU_MAIL_DEBUG_PORT";

/// Wry 0.55's default WebView2 arguments (`wry/src/webview2/mod.rs`; no autoplay, no proxy, which the
/// mail window sets neither of). `additional_browser_args` replaces them, so they are kept here.
pub const WRY_DEFAULT_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

/// A port from its decimal digits only: no sign, no space, not 0, at most 65535.
pub fn parse_port(raw: &str) -> Option<u16> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u16>().ok().filter(|&p| p != 0)
}

/// Wry's defaults plus the port, with CDP's origin check open to the loopback page only.
pub fn browser_args(port: u16) -> String {
    format!("{WRY_DEFAULT_ARGS} --remote-debugging-port={port} --remote-allow-origins=http://127.0.0.1:{port}")
}

/// The mail window's browser arguments when [`MAIL_PORT_VAR`] names a port; `None` leaves wry's
/// defaults alone.
pub fn mail_browser_args() -> Option<String> {
    let raw = std::env::var(MAIL_PORT_VAR).ok()?;
    parse_port(&raw).map(browser_args)
}
