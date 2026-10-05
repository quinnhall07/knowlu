//! **T9.2: the mail window and the script runner** (spec tests 33 and 34, D5, D19; the T0.S1-S3
//! report). Every page here is a fake that records what it was asked to evaluate: no window opens
//! and nothing reaches a network. Names, addresses and school hosts are synthetic (`.invalid`).
use knowlu::mail::{popup_allowed, run, script, scripts, signed_in, MailWindow, Page, PageTitle, Refusal, INIT_SCRIPT, POPUP, WINDOW};
use knowlu::mail::live::sweep_at_launch;
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

/// `MailWindow` is one flag for the whole process: the tests here that take it run one at a time.
static WINDOW_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn one_mail_window_at_a_time() {
    let _one = WINDOW_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let first = MailWindow::try_take().expect("the first taker gets the mail window");
    assert!(MailWindow::try_take().is_none(), "a second mail window while one is held");
    drop(first);
    let again = MailWindow::try_take().expect("released when the holder is dropped");
    drop(again);
}

/// Checkpoint C (spec §5.4): the launch sweep deletes `mail-setup` only while it holds the mail window,
/// so a setup the student starts at launch cannot open WebView2 on a folder being deleted; with the
/// window already held it sweeps nothing. It hands the window on to the resume.
#[test]
fn the_launch_sweep_runs_only_while_it_holds_the_mail_window() {
    let _one = WINDOW_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = std::env::temp_dir().join(format!("knowlu-mail-window-sweep-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let net = knowlu::mail_session::setup_dir(&d).join("Default").join("Network");
    std::fs::create_dir_all(&net).unwrap();
    std::fs::write(net.join("Cookies"), "synthetic").unwrap();
    let busy = MailWindow::try_take().expect("a setup holds the mail window");
    assert!(sweep_at_launch(&d, MailWindow::try_take()).is_none(), "the window is busy: no sweep");
    assert!(net.join("Cookies").exists(), "the open window's profile is left alone");
    drop(busy);
    let held = sweep_at_launch(&d, MailWindow::try_take()).expect("the sweep hands its window on");
    assert!(!knowlu::mail_session::setup_dir(&d).exists(), "the leftover mail-setup is swept");
    assert!(MailWindow::try_take().is_none(), "the window is still held after the sweep");
    drop(held);
    assert!(MailWindow::try_take().is_some());
    let _ = std::fs::remove_dir_all(&d);
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

/// T10.2a: Microsoft 365's four scripts are compiled in, run for that provider alone, and carry the
/// caller's arguments and the script's own source into the page.
#[test]
fn the_m365_scripts_run_for_m365_only() {
    let on = "https://outlook.cloud.microsoft/mail/options/forwarding";
    for name in ["m365/signin.js", "m365/forwarding_read.js", "m365/forwarding_set.js", "m365/forwarding_off.js"] {
        assert!(script(name).is_some(), "{name} is compiled in");
        let gmail = Fake::at("https://mail.google.com/mail/u/0/").says(json!({ "state": "started" }));
        assert_eq!(run(&gmail, Provider::Gmail, name, &json!({}), WAIT), Err(Refusal::NotForProvider), "{name}");
        assert!(gmail.evals.borrow().is_empty(), "{name} was evaluated for Gmail");
        let page = Fake::at(on).says(json!({ "state": "started" })).says(json!({ "state": "done", "value": { "result": "ok" } }));
        let args = json!({ "address": "box-1@in.example.invalid" });
        assert_eq!(run(&page, Provider::M365, name, &args, WAIT), Ok(json!({ "result": "ok" })), "{name}");
        let evals = page.evals.borrow();
        assert!(evals[0].contains(script(name).unwrap().source()), "{name}: the compiled-in source runs");
        assert!(evals[0].contains("box-1@in.example.invalid"), "{name}: the arguments reach the page");
    }
}

/// T10.2b: Microsoft 365's four mailbox scripts (search, list, forward as attachment, Sent cleanup) are
/// compiled in and run for that provider alone.
#[test]
fn the_m365_mailbox_scripts_run_for_m365_only() {
    let on = "https://outlook.cloud.microsoft/mail/";
    for name in ["m365/search.js", "m365/list.js", "m365/forward.js", "m365/sent_cleanup.js"] {
        assert!(script(name).is_some(), "{name} is compiled in");
        let gmail = Fake::at("https://mail.google.com/mail/u/0/").says(json!({ "state": "started" }));
        assert_eq!(run(&gmail, Provider::Gmail, name, &json!({}), WAIT), Err(Refusal::NotForProvider), "{name}");
        assert!(gmail.evals.borrow().is_empty(), "{name} was evaluated for Gmail");
        let page = Fake::at(on).says(json!({ "state": "started" })).says(json!({ "state": "done", "value": { "result": "ok" } }));
        assert_eq!(run(&page, Provider::M365, name, &json!({ "nonce": "n-1" }), WAIT), Ok(json!({ "result": "ok" })), "{name}");
        assert!(page.evals.borrow()[0].contains(script(name).unwrap().source()), "{name}: the compiled-in source runs");
    }
}

/// T10.3a: Gmail's six forwarding scripts (read, add, add-done, set, off, remove) are compiled in, run for
/// that provider alone, and carry the caller's arguments and the script's own source into the page.
#[test]
fn the_gmail_forwarding_scripts_run_for_gmail_only() {
    let on = "https://mail.google.com/mail/u/0/#settings/fwdandpop";
    let names = [
        "gmail/forwarding_read.js",
        "gmail/forwarding_add.js",
        "gmail/forwarding_add_done.js",
        "gmail/forwarding_set.js",
        "gmail/forwarding_off.js",
        "gmail/forwarding_remove_address.js",
    ];
    for name in names {
        assert!(script(name).is_some(), "{name} is compiled in");
        let m365 = Fake::at("https://outlook.cloud.microsoft/mail/").says(json!({ "state": "started" }));
        assert_eq!(run(&m365, Provider::M365, name, &json!({}), WAIT), Err(Refusal::NotForProvider), "{name}");
        assert!(m365.evals.borrow().is_empty(), "{name} was evaluated for Microsoft 365");
        let page = Fake::at(on).says(json!({ "state": "started" })).says(json!({ "state": "done", "value": { "result": "ok" } }));
        let args = json!({ "address": "box-1@in.example.invalid" });
        assert_eq!(run(&page, Provider::Gmail, name, &args, WAIT), Ok(json!({ "result": "ok" })), "{name}");
        let evals = page.evals.borrow();
        assert!(evals[0].contains(script(name).unwrap().source()), "{name}: the compiled-in source runs");
        assert!(evals[0].contains("box-1@in.example.invalid"), "{name}: the arguments reach the page");
    }
}

/// T10.3b: Gmail's four mailbox scripts (search, list, forward as attachment, Sent cleanup), and T11.2's
/// conversation reader, are compiled in and run for that provider alone.
#[test]
fn the_gmail_mailbox_scripts_run_for_gmail_only() {
    let on = "https://mail.google.com/mail/u/0/#inbox";
    for name in ["gmail/search.js", "gmail/list.js", "gmail/forward.js", "gmail/sent_cleanup.js", "gmail/thread.js"] {
        assert!(script(name).is_some(), "{name} is compiled in");
        let m365 = Fake::at("https://outlook.cloud.microsoft/mail/").says(json!({ "state": "started" }));
        assert_eq!(run(&m365, Provider::M365, name, &json!({}), WAIT), Err(Refusal::NotForProvider), "{name}");
        assert!(m365.evals.borrow().is_empty(), "{name} was evaluated for Microsoft 365");
        let page = Fake::at(on).says(json!({ "state": "started" })).says(json!({ "state": "done", "value": { "result": "ok" } }));
        assert_eq!(run(&page, Provider::Gmail, name, &json!({ "nonce": "n-1" }), WAIT), Ok(json!({ "result": "ok" })), "{name}");
        assert!(page.evals.borrow()[0].contains(script(name).unwrap().source()), "{name}: the compiled-in source runs");
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

/// The page title X2 reads is the current mail window's latest document title: a new window starts
/// with none, and a title from a window that is gone (a late event) is dropped.
#[test]
fn the_page_title_store_keeps_the_current_windows_latest_title_only() {
    let store = PageTitle::new();
    assert_eq!(store.get(), None, "no window, no title");
    let first = store.begin();
    assert_eq!(store.get(), None, "a new window has no title until its page names one");
    store.set(first, "Sign in to your account".into());
    store.set(first, "Mail - Sam Student - Outlook".into());
    assert_eq!(store.get().as_deref(), Some("Mail - Sam Student - Outlook"), "the latest title");
    let second = store.begin();
    assert_eq!(store.get(), None, "the last window's title is gone");
    store.set(first, "Inbox - old@example.invalid - Gmail".into());
    assert_eq!(store.get(), None, "a closed window's late title is dropped");
    store.set(second, "Inbox - student@example.invalid - Gmail".into());
    assert_eq!(store.get().as_deref(), Some("Inbox - student@example.invalid - Gmail"));
    store.clear();
    assert_eq!(store.get(), None, "closing the window forgets its title");
    store.set(second, "Inbox - student@example.invalid - Gmail".into());
    assert_eq!(store.get(), None, "a closed window's late title is dropped");
}

/// Found live in T18c (2026-10-05): `WebviewWindow::title` is the NATIVE title the app set in
/// `open` ("Sign in to your email"), never the page's, so a signed-in mailbox read as signed out
/// forever. Every title a mail detector reads is the page's document title, from
/// `on_document_title_changed` on the mail window's builder, and no mail source reads the native one.
#[test]
fn the_sign_in_detector_is_fed_the_page_title_never_the_native_window_title() {
    let mut files = vec![PathBuf::from("src/mail.rs")];
    files.extend(std::fs::read_dir("src/mail").unwrap().flatten().map(|e| e.path()));
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        assert!(!text.contains(".title()"), "{} reads the native window title", f.display());
    }
    let src = std::fs::read_to_string("src/mail.rs").unwrap().replace("\r\n", "\n");
    let open = &src[src.find("\nfn open(").expect("src/mail.rs has open")..];
    let open = &open[..open.find("\n}\n").expect("open ends")];
    assert!(open.contains("PAGE_TITLE.begin()"), "a new mail window keeps the last one's title");
    let builder = &open[open.find("WebviewWindowBuilder::new(app, WINDOW").expect("the mail window's builder")..];
    let builder = &builder[..builder.find(".build()").expect("the mail window's builder builds")];
    assert!(builder.contains(".on_document_title_changed("), "the mail window's page title is never read");
    let detector = &src[src.find("pub fn window_signed_in(").expect("window_signed_in")..];
    let detector = &detector[..detector.find("\n}\n").expect("window_signed_in ends")];
    assert!(detector.contains("page_title("), "the detector is not fed the page title");
    let close = &src[src.find("pub fn close(").expect("close")..];
    assert!(close[..close.find("\n}\n").unwrap()].contains("PAGE_TITLE.clear()"), "a closed window's title is kept");
    let live = std::fs::read_to_string("src/mail/live.rs").unwrap();
    assert!(live.contains("address_in_title(&page_title("), "Gmail's address hint is not read from the page title");
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
    // mail.rs's own submodules (T11.2's recover.rs) are mail code too.
    for e in std::fs::read_dir("src/mail").unwrap().flatten() {
        files.push(e.path());
    }
    assert!(files.iter().any(|p| p.ends_with("recover.rs")), "src/mail/recover.rs was not read");
    js_files(Path::new("assets/mail"), &mut files);
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        for word in ["SendInput", "keybd_event", "mouse_event", "SetCursorPos", "remote-debugging"] {
            assert!(!text.contains(word), "{} names {word}", f.display());
        }
    }
}
