//! **T9.2: the mail window and the script runner** (spec tests 33 and 34, D5, D19; the T0.S1-S3
//! report). Every page here is a fake that records what it was asked to evaluate: no window opens
//! and nothing reaches a network. Names, addresses and school hosts are synthetic (`.invalid`).
use knowlu::mail::{popup_allowed, run, script, scripts, signed_in, MailWindow, Page, Refusal, INIT_SCRIPT, POPUP, WINDOW};
use knowlu::mail_steps::Provider;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

const WAIT: Duration = Duration::from_millis(400);

/// A page at `url`. Each evaluation is recorded, answered from `replies` (none left is no answer in
/// time), and then the page moves to the next of `moves`, if any: a redirect between two calls.
struct Fake {
    url: RefCell<Option<String>>,
    moves: RefCell<VecDeque<String>>,
    replies: RefCell<VecDeque<String>>,
    evals: RefCell<Vec<String>>,
}

impl Fake {
    fn at(url: &str) -> Fake {
        Fake { url: RefCell::new(Some(url.into())), moves: Default::default(), replies: Default::default(), evals: Default::default() }
    }
    /// What WebView2's completion hands back for a script whose value is `JSON.stringify(v)`.
    fn says(self, v: Value) -> Fake {
        self.replies.borrow_mut().push_back(serde_json::to_string(&v.to_string()).unwrap());
        self
    }
    fn then_moves_to(self, url: &str) -> Fake {
        self.moves.borrow_mut().push_back(url.into());
        self
    }
}

impl Page for Fake {
    fn url(&self) -> Option<String> {
        self.url.borrow().clone()
    }
    fn eval(&self, js: &str, _wait: Duration) -> Option<String> {
        self.evals.borrow_mut().push(js.to_string());
        if let Some(next) = self.moves.borrow_mut().pop_front() {
            *self.url.borrow_mut() = Some(next);
        }
        self.replies.borrow_mut().pop_front()
    }
}

fn capability_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir("capabilities")
        .expect("app/capabilities")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no capability files were read");
    files
}

// ---- test 33: the label, the capabilities, one window ---------------------------------------

#[test]
fn the_mail_label_has_no_capability_grant() {
    assert_eq!(WINDOW, "mail");
    assert_eq!(POPUP, "mail-popup");
    let text = std::fs::read_to_string("capabilities/default.json").unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["windows"], json!(["main"]), "the console window is the only window granted anything");
    assert!(v.get("webviews").is_none(), "no webview is granted anything by label");
    for label in [WINDOW, POPUP] {
        assert!(!text.contains(&format!("\"{label}\"")), "capabilities/default.json names {label}");
    }
}

/// Quinn's IPC ruling (2026-10-02): Tauri injects `__TAURI_INTERNALS__` into every webview, so the
/// mail window's safety is that no capability covers it and every invoke from it is denied. The
/// entries are globs (`acl/capability.rs`), so the pin is an allow-list: every `windows` and
/// `webviews` entry is exactly `main`. A `remote` block or any web URL in a capability would hand a
/// remote origin an IPC grant, and an inline capability in `tauri.conf.json` would bypass the files.
#[test]
fn no_capability_names_a_mail_window_or_a_remote_url() {
    for path in capability_files() {
        let text = std::fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for key in ["windows", "webviews"] {
            for label in v.get(key).and_then(Value::as_array).cloned().unwrap_or_default() {
                assert_eq!(label.as_str(), Some("main"), "{} grants `{label}` through `{key}`", path.display());
            }
        }
        for label in [WINDOW, POPUP] {
            assert!(!text.contains(&format!("\"{label}")), "{} names {label}", path.display());
        }
        assert!(!text.contains("\"remote\""), "{} carries a remote urls block", path.display());
        for scheme in ["http://", "https://", "http:\\/\\/", "https:\\/\\/"] {
            assert!(!text.contains(scheme), "{} names a web URL", path.display());
        }
    }
    let conf: Value = serde_json::from_str(&std::fs::read_to_string("tauri.conf.json").unwrap()).unwrap();
    for cap in conf.pointer("/app/security/capabilities").and_then(Value::as_array).cloned().unwrap_or_default() {
        assert!(cap.is_string(), "tauri.conf.json defines a capability inline: {cap}");
    }
}

#[test]
fn one_mail_window_at_a_time() {
    let first = MailWindow::try_take().expect("the first taker gets the mail window");
    assert!(MailWindow::try_take().is_none(), "a second mail window while one is held");
    drop(first);
    let again = MailWindow::try_take().expect("released when the holder is dropped");
    drop(again);
}

/// S1b: the popup's first request is `about:blank`; then only the provider's own hosts.
#[test]
fn the_popup_opens_only_for_about_blank_and_the_providers_own_hosts() {
    assert!(popup_allowed(Provider::Gmail, "about:blank"));
    assert!(popup_allowed(Provider::Gmail, "https://accounts.google.com/v3/signin/challenge"));
    assert!(popup_allowed(Provider::Gmail, "https://mail-settings.google.com/mail/vf-x"));
    assert!(popup_allowed(Provider::M365, "about:blank"));
    assert!(popup_allowed(Provider::M365, "https://login.microsoftonline.com/common/reprocess"));
    for (p, url) in [
        (Provider::Gmail, "about:srcdoc"),
        (Provider::Gmail, "http://accounts.google.com/"),
        (Provider::Gmail, "https://accounts.google.com.example.invalid/"),
        (Provider::Gmail, "https://login.microsoftonline.com/"),
        (Provider::M365, "https://accounts.google.com/"),
        (Provider::M365, "https://sso.school.example.invalid/"),
        (Provider::Guided, "about:blank"),
        (Provider::Gmail, "javascript:alert(1)"),
    ] {
        assert!(!popup_allowed(p, url), "{p:?} allowed a popup at {url}");
    }
}

/// S1b, on the popup's own window (no window opens in a test, so this reads `src/mail.rs`): every
/// navigation in the popup passes [`popup_allowed`], the popup opens no popup of its own, and its
/// `window.close()` destroys the Tauri window. wry's own close handler destroys only the webview's
/// container, which left an empty frame in the spike, so `popup_open` would stay true.
#[test]
fn the_popup_is_filtered_opens_nothing_and_closes_its_window() {
    let src = std::fs::read_to_string("src/mail.rs").unwrap().replace("\r\n", "\n");
    let start = src.find("fn open_popup(").expect("src/mail.rs has open_popup");
    let body = &src[start..];
    let body = &body[..body.find("\n}\n").expect("open_popup ends")];
    let builder = &body[body.find("WebviewWindowBuilder::new(app, POPUP").expect("the popup's builder")..];
    let builder = &builder[..builder.find(".build()").expect("the popup's builder builds")];
    assert!(builder.contains(".on_navigation(move |u| popup_allowed(provider, u.as_str()))"), "the popup navigates unfiltered");
    assert!(builder.contains(".on_new_window(|_, _| NewWindowResponse::Deny)"), "the popup may open popups");
    assert!(body.contains("destroy_on_close_request("), "open_popup leaves window.close() to wry");
    let hook = &src[src.find("fn destroy_on_close_request(").expect("the close hook")..];
    let hook = &hook[..hook.find("\n}\n").expect("the close hook ends")];
    assert!(hook.contains("add_WindowCloseRequested") && hook.contains(".destroy()"), "the close hook does not destroy the window");
}

// ---- test 33: the origin check (S1c) ---------------------------------------------------------

/// D5 and S1c: a script runs only while the top-level page is on the provider's mail hosts, checked
/// host-side before every evaluation and again in the page (`location.host`). A sign-in or SSO page,
/// another provider's mailbox, plain http, a port, a user part or a look-alike host runs nothing.
#[test]
fn the_runner_refuses_a_page_off_the_providers_origins() {
    let args = json!({});
    for (p, url) in [
        (Provider::M365, "https://sso.school.example.invalid/idp/profile/SAML2/Redirect/SSO"),
        (Provider::M365, "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"),
        (Provider::M365, "http://outlook.office.com/mail/"),
        (Provider::M365, "https://outlook.office.com.example.invalid/mail/"),
        (Provider::M365, "https://outlook.cloud.microsoft:8443/mail/"),
        (Provider::M365, "https://student@outlook.cloud.microsoft/mail/"),
        (Provider::M365, "https://mail.google.com/mail/u/0/"),
        (Provider::Gmail, "https://accounts.google.com/v3/signin/identifier"),
        (Provider::Gmail, "https://outlook.cloud.microsoft/mail/"),
        (Provider::Gmail, "about:blank"),
        (Provider::Guided, "https://mail.example.invalid/"),
    ] {
        let page = Fake::at(url).says(json!({ "state": "started" }));
        assert_eq!(run(&page, p, "ready.js", &args, WAIT), Err(Refusal::OffOrigin), "{p:?} at {url}");
        assert!(page.evals.borrow().is_empty(), "{p:?} evaluated a script at {url}");
    }
    let nowhere = Fake::at("https://outlook.cloud.microsoft/mail/");
    *nowhere.url.borrow_mut() = None;
    assert_eq!(run(&nowhere, Provider::M365, "ready.js", &args, WAIT), Err(Refusal::NoPage));
    assert!(nowhere.evals.borrow().is_empty());

    // The positive control: Outlook's two hosts (S1c: outlook.office.com redirects to
    // outlook.cloud.microsoft) and Gmail's. The script starts, then one poll reads its result.
    for (p, url) in [
        (Provider::M365, "https://outlook.office.com/mail/"),
        (Provider::M365, "https://OUTLOOK.cloud.microsoft/mail/inbox"),
        (Provider::Gmail, "https://mail.google.com/mail/u/0/#inbox"),
    ] {
        let page = Fake::at(url).says(json!({ "state": "started" })).says(json!({ "state": "done", "value": "complete" }));
        assert_eq!(run(&page, p, "ready.js", &args, WAIT), Ok(json!("complete")), "{p:?} at {url}");
        let evals = page.evals.borrow();
        assert_eq!(evals.len(), 2, "one start and one poll");
        assert!(evals[0].contains(script("ready.js").unwrap().source()), "the compiled-in source runs");
        for js in evals.iter() {
            assert!(js.contains("location.host"), "every evaluation carries the in-page host guard");
        }
    }

    // A redirect after the start: the poll is never evaluated on the new origin.
    let page = Fake::at("https://outlook.cloud.microsoft/mail/")
        .says(json!({ "state": "started" }))
        .then_moves_to("https://sso.school.example.invalid/login");
    assert_eq!(run(&page, Provider::M365, "ready.js", &args, WAIT), Err(Refusal::OffOrigin));
    assert_eq!(page.evals.borrow().len(), 1);

    // The in-page guard's own refusal (the host moved under the host-side check) is the same word.
    let page = Fake::at("https://outlook.cloud.microsoft/mail/").says(json!({ "state": "refused" }));
    assert_eq!(run(&page, Provider::M365, "ready.js", &args, WAIT), Err(Refusal::OffOrigin));

    // No answer in time, a new document that lost the result, a throw, and garbage.
    let page = Fake::at("https://outlook.cloud.microsoft/mail/");
    assert_eq!(run(&page, Provider::M365, "ready.js", &args, WAIT), Err(Refusal::TimedOut));
    let page = Fake::at("https://outlook.cloud.microsoft/mail/").says(json!({ "state": "started" })).says(json!({ "state": "lost" }));
    assert_eq!(run(&page, Provider::M365, "ready.js", &args, WAIT), Err(Refusal::Lost));
    let page = Fake::at("https://outlook.cloud.microsoft/mail/").says(json!({ "state": "started" })).says(json!({ "state": "failed" }));
    assert_eq!(run(&page, Provider::M365, "ready.js", &args, WAIT), Err(Refusal::Failed));
    let page = Fake::at("https://outlook.cloud.microsoft/mail/");
    page.replies.borrow_mut().push_back("null".into());
    assert_eq!(run(&page, Provider::M365, "ready.js", &args, WAIT), Err(Refusal::Unreadable));
    let mut running = Fake::at("https://outlook.cloud.microsoft/mail/").says(json!({ "state": "started" }));
    for _ in 0..200 {
        running = running.says(json!({ "state": "running" }));
    }
    assert_eq!(run(&running, Provider::M365, "ready.js", &args, Duration::from_millis(150)), Err(Refusal::TimedOut));
    let all = [Refusal::Unknown, Refusal::NotForProvider, Refusal::NoPage, Refusal::OffOrigin, Refusal::TimedOut, Refusal::Lost, Refusal::Failed, Refusal::Unreadable];
    for r in all {
        assert!(!r.as_str().is_empty());
    }
}

// ---- D19: compiled-in scripts only -----------------------------------------------------------

fn js_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            js_files(&p, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("js") {
            out.push(p);
        }
    }
}

/// D19: the runner takes a script's name, never its text, and knows only the scripts compiled in
/// with `include_str!` from `app/assets/mail/`. Every `.js` file there is compiled in byte for byte;
/// the initialisation script (S3) is compiled in too but is not a step script, so no name runs it.
#[test]
fn the_runner_runs_only_compiled_in_scripts() {
    let on = "https://outlook.cloud.microsoft/mail/";
    for name in ["return document.cookie", "../ready.js", "m365/../ready.js", "READY.JS", "ready", "", "init.js", "/ready.js"] {
        let page = Fake::at(on).says(json!({ "state": "started" }));
        assert_eq!(run(&page, Provider::M365, name, &json!({}), WAIT), Err(Refusal::Unknown), "{name:?}");
        assert!(page.evals.borrow().is_empty(), "{name:?} was evaluated");
        assert!(script(name).is_none(), "{name:?}");
    }
    let root = Path::new("assets/mail");
    let mut files = Vec::new();
    js_files(root, &mut files);
    assert!(!files.is_empty());
    for f in &files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(f).unwrap();
        if rel == "init.js" {
            assert_eq!(INIT_SCRIPT, text, "init.js is compiled in as it is");
        } else {
            let s = script(&rel).unwrap_or_else(|| panic!("{rel} is under assets/mail but not compiled in"));
            assert_eq!(s.source(), text, "{rel} is compiled in as it is");
            assert_eq!(s.path(), rel);
        }
    }
    assert_eq!(scripts().len() + 1, files.len(), "every compiled-in script is a file under assets/mail");
    for s in scripts() {
        let dir = s.path().split_once('/').map(|(d, _)| d);
        assert!(matches!(dir, None | Some("m365") | Some("gmail")), "{} is in no provider's folder", s.path());
    }
}

// ---- X2's title signal -----------------------------------------------------------------------

#[test]
fn the_sign_in_detector_reads_m365_and_gmail_titles() {
    let outlook = "https://outlook.cloud.microsoft/mail/inbox";
    let gmail = "https://mail.google.com/mail/u/0/#inbox";
    assert!(signed_in(Provider::M365, Some(outlook), "Mail - Sam Student - Outlook"));
    assert!(signed_in(Provider::M365, Some("https://outlook.office.com/mail/"), "Inbox - Sam Student - Outlook"));
    assert!(signed_in(Provider::Gmail, Some(gmail), "Inbox (3) - student@example.invalid - Gmail"));
    assert!(signed_in(Provider::Gmail, Some(gmail), "Inbox - student@school.example.invalid - Example School Mail"));
    for (p, url, title) in [
        (Provider::M365, Some(outlook), "Outlook"),
        (Provider::M365, Some(outlook), "Sign in to your account"),
        (Provider::M365, Some(outlook), " - Outlook"),
        (Provider::M365, Some(outlook), ""),
        (Provider::M365, Some("https://login.microsoftonline.com/common/oauth2"), "Mail - Sam Student - Outlook"),
        (Provider::M365, None, "Mail - Sam Student - Outlook"),
        (Provider::M365, Some(gmail), "Inbox (3) - student@example.invalid - Gmail"),
        (Provider::Gmail, Some(gmail), "Gmail"),
        (Provider::Gmail, Some(gmail), "Inbox - Gmail"),
        (Provider::Gmail, Some("https://accounts.google.com/v3/signin"), "Inbox (3) - student@example.invalid - Gmail"),
        (Provider::Gmail, Some(outlook), "Mail - Sam Student - Outlook"),
        (Provider::Guided, Some(outlook), "Mail - Sam Student - Outlook"),
    ] {
        assert!(!signed_in(p, url, title), "{p:?} at {url:?} titled {title:?}");
    }
}

// ---- test 34: no OS input, no debugging port --------------------------------------------------

#[test]
fn no_os_input_and_no_debugging_port_in_mail_sources() {
    let mut files: Vec<PathBuf> = std::fs::read_dir("src")
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            name.starts_with("mail") && name.ends_with(".rs")
        })
        .collect();
    assert!(files.iter().any(|p| p.ends_with("mail.rs")), "src/mail.rs was not read");
    js_files(Path::new("assets/mail"), &mut files);
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        for word in ["SendInput", "keybd_event", "mouse_event", "SetCursorPos", "remote-debugging"] {
            assert!(!text.contains(word), "{} names {word}", f.display());
        }
    }
}
