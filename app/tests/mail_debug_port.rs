//! **The debug build's mail-window debugging port** (T18c diagnosis, 2026-10-05). A debug build may
//! open read-only CDP on the `mail` window's own WebView2 environment when `KNOWLU_MAIL_DEBUG_PORT`
//! names a port, so a live Outlook page can be inspected. A release build carries none of it: the
//! module is compiled only under `cfg(debug_assertions)`, and so is its one call site. The product's
//! mail sources still name no debugging port (spec D5, test 34 in `mail_window.rs`).
#![cfg(debug_assertions)]
use knowlu::debug_port::{browser_args, parse_port, MAIL_PORT_VAR, WRY_DEFAULT_ARGS};

fn src(path: &str) -> String {
    std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

#[test]
fn the_port_parser_takes_only_a_port_number() {
    for bad in ["", " ", "abc", "92a", "+9333", "-1", " 9333", "9333 ", "0", "00", "65536", "99999", "4294967296"] {
        assert_eq!(parse_port(bad), None, "{bad:?} parsed as a port");
    }
    assert_eq!(parse_port("1"), Some(1));
    assert_eq!(parse_port("9334"), Some(9334));
    assert_eq!(parse_port("65535"), Some(65535));
}

#[test]
fn the_args_keep_wrys_defaults_and_allow_only_the_loopback_origin() {
    // Wry 0.55's own default, which `additional_browser_args` replaces rather than extends.
    assert_eq!(WRY_DEFAULT_ARGS, "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection");
    assert_eq!(
        browser_args(9334),
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
         --remote-debugging-port=9334 --remote-allow-origins=http://127.0.0.1:9334"
    );
}

#[test]
fn the_hook_is_debug_only_and_reads_one_variable() {
    assert_eq!(MAIL_PORT_VAR, "KNOWLU_MAIL_DEBUG_PORT");
    let lib = src("src/lib.rs");
    assert!(lib.contains("#[cfg(debug_assertions)] pub mod debug_port;"), "debug_port is not debug-only");
    assert_eq!(lib.matches("mod debug_port").count(), 1, "debug_port is declared twice");

    let hook = src("src/debug_port.rs");
    assert_eq!(hook.matches("env::var").count(), 1, "the hook reads more than one variable");
    assert!(hook.contains("std::env::var(MAIL_PORT_VAR)"), "the hook reads something else");
    assert!(!hook.contains("set_var") && !hook.contains("vars()"), "the hook touches the environment");

    // The one call site: in `open`, on the mail window's builder, gated line by line.
    for entry in std::fs::read_dir("src").unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let text = src(path.to_str().unwrap());
        let uses = text.matches("debug_port::").count() + text.matches("additional_browser_args").count();
        match name.as_str() {
            "lib.rs" | "debug_port.rs" => {}
            "mail.rs" => assert_eq!(uses, 2, "mail.rs uses the hook more than once"),
            _ => assert_eq!(uses, 0, "{name} uses the debug hook or sets browser args"),
        }
    }
    let mail = src("src/mail.rs");
    let open = &mail[mail.find("\nfn open(").expect("src/mail.rs has open")..];
    let open = &open[..open.find("\n}\n").expect("open ends")];
    let call = open.find("debug_port::mail_browser_args()").expect("open calls the hook");
    let before: Vec<&str> = open[..call].lines().collect();
    assert_eq!(before[before.len() - 2].trim(), "#[cfg(debug_assertions)]", "the call is not debug-only");
    assert!(open[call..].contains(".additional_browser_args(&args)"), "the args are not on the mail builder");
    assert!(open.find("debug_port::").unwrap() < open.find(".build()").unwrap(), "the hook runs after build");
}
