//! The mail setup's window and script runner (email-forwarding plan T9.2; spec D5, D19, §4.2, §4.3,
//! §5.4, §6.2; the T0.S1-S3 report). The runner and its checks are pure and tested through [`Page`];
//! the window half at the bottom is the one place a mail window is built.
//!
//! **One window, `mail`**, with no capability grant (Tauri injects its internals into every webview,
//! and Quinn's ruling of 2026-10-02 accepts that only while no capability covers a mail window, so
//! every invoke from it is denied), on the mailbox's kept profile ([`crate::mail_session`]). It is
//! visible only to sign in; then it steps aside (hidden, out of the taskbar: S2's mode) and comes
//! back, centred and focused, only when the site needs the student. Google's "verify it's you"
//! popup is the one other window, `mail-popup`, in the same profile (S1b).
//!
//! **The runner** evaluates a compiled-in script by name (D19), never text from anywhere else, and
//! only while the top-level page is on the provider's mail hosts (D5): checked host-side before
//! every evaluation, and again in the page through `location.host`, because a script queued before
//! a cross-origin navigation can still run on the new document (S1c). The result comes back through
//! WebView2's script-completion value (S1a), never IPC. Every refusal is a fixed word: no page text,
//! address, URL or session ever reaches an error, a log or telemetry. Setup drives the page by
//! script evaluation only; nothing here simulates a keyboard or a mouse, and the window has no
//! debugging port (a debug build's diagnostic hook aside, `crate::debug_port`, which no release
//! build contains).
use crate::mail_steps::Provider;
use serde_json::{json, Value};

pub mod backfill;
pub mod commands;
pub mod host;
pub mod live;
pub mod recover;
mod runner;
pub mod setup;
pub mod slot;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// The mail window's one label (spec §5.4).
pub const WINDOW: &str = "mail";
/// Google's "verify it's you" popup (S1b): the mail window's child, in the same profile.
pub const POPUP: &str = "mail-popup";

// ---- origins (D5) -----------------------------------------------------------------------------

/// The hosts a step script may run on. Outlook lives at `outlook.cloud.microsoft` now and
/// `outlook.office.com` redirects there (S1c), so both. A guided provider runs nothing.
pub fn mail_hosts(provider: Provider) -> &'static [&'static str] {
    match provider {
        Provider::M365 => &["outlook.office.com", "outlook.cloud.microsoft"],
        Provider::Gmail => &["mail.google.com"],
        Provider::Guided => &[],
    }
}

/// The provider's own sign-in hosts: where a popup may go (S1b), and never where a script runs.
fn sign_in_hosts(provider: Provider) -> &'static [&'static str] {
    match provider {
        Provider::M365 => &["login.microsoftonline.com"],
        Provider::Gmail => &["accounts.google.com", "mail-settings.google.com"],
        Provider::Guided => &[],
    }
}

/// Whether `url` is `https://<one of the provider's mail hosts>…` exactly (grades' `on_host`: no
/// user part, no port, case-blind host).
pub fn on_mail_host(provider: Provider, url: &str) -> bool {
    mail_hosts(provider).iter().any(|h| crate::grades::on_host(url, h))
}

/// S1b: the popup's first request is `about:blank`; after that only the provider's own sign-in or
/// mail hosts. Anything else, and every popup for a guided provider, is denied.
pub fn popup_allowed(provider: Provider, url: &str) -> bool {
    if mail_hosts(provider).is_empty() {
        return false;
    }
    url == "about:blank"
        || on_mail_host(provider, url)
        || sign_in_hosts(provider).iter().any(|h| crate::grades::on_host(url, h))
        || sign_in_hops(provider).iter().any(|&(host, path)| on_hop(url, host, path))
}

/// Single pages on another host that the provider's sign-in passes through. T18c (live, 2026-10-18): once
/// the phone approves, Google's verify popup goes by YouTube's `SetSID` to carry the session across
/// Google's domains, and a refused hop leaves the popup on "Loading". The path, not the host, is allowed.
fn sign_in_hops(provider: Provider) -> &'static [(&'static str, &'static str)] {
    match provider {
        Provider::Gmail => &[("accounts.youtube.com", "/accounts/SetSID")],
        Provider::M365 | Provider::Guided => &[],
    }
}

/// `https://<host><path>` exactly, any query: no user part, no port, the host case-blind.
fn on_hop(url: &str, host: &str, path: &str) -> bool {
    tauri::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.username().is_empty()
            && u.password().is_none()
            && u.port().is_none()
            && u.host_str().is_some_and(|h| h.eq_ignore_ascii_case(host))
            && u.path() == path
    })
}

/// Where setup's window opens: the provider's mailbox, which sends a signed-out student to sign in.
pub fn start_url(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::M365 => Some("https://outlook.office.com/mail/"),
        Provider::Gmail => Some("https://mail.google.com/mail/u/0/"),
        Provider::Guided => None,
    }
}

// ---- the sign-in detector (X2's title signal) ---------------------------------------------------

/// X2's signal: the page is on a mail host and its title reads as a signed-in mailbox. Outlook's is
/// `<folder> - <name> - Outlook`; Gmail's is `<folder> - <address> - Gmail` (a Google Workspace
/// mailbox ends with its organisation's name instead), so for Gmail a middle part must be an
/// address. A step-script probe confirms it before anything acts (D5). `title` is the page's
/// document title ([`PageTitle`]), never the native window's.
pub fn signed_in(provider: Provider, url: Option<&str>, title: &str) -> bool {
    if !url.is_some_and(|u| on_mail_host(provider, u)) {
        return false;
    }
    let parts: Vec<&str> = title.split(" - ").collect();
    let named = parts.len() >= 3 && parts.iter().all(|p| !p.trim().is_empty());
    match provider {
        Provider::M365 => named && parts.last() == Some(&"Outlook"),
        Provider::Gmail => named && parts[1..parts.len() - 1].iter().any(|p| p.contains('@')),
        Provider::Guided => false,
    }
}

/// The open mail window's page title: WebView2's document title, which is the only title X2 reads.
/// `WebviewWindow::title` is the native window's title (the one [`open`] sets, "Sign in to your
/// email"), never the page's; feeding it to [`signed_in`] kept setup on "Sign in" forever (T18c,
/// 2026-10-05). The mail window's `on_document_title_changed` writes here under the generation
/// [`PageTitle::begin`] gave it, so a late title from a window that is gone is dropped. Held in
/// memory only while that window is open; never logged, written or sent.
pub struct PageTitle(std::sync::Mutex<(u64, Option<String>)>);

impl PageTitle {
    pub const fn new() -> PageTitle {
        PageTitle(std::sync::Mutex::new((0, None)))
    }

    fn slot(&self) -> std::sync::MutexGuard<'_, (u64, Option<String>)> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// A new mail window: the last one's title is gone. The generation its title handler writes under.
    pub fn begin(&self) -> u64 {
        let mut s = self.slot();
        *s = (s.0 + 1, None);
        s.0
    }

    /// The page of the window of `generation` now has `title`; ignored for any other window.
    pub fn set(&self, generation: u64, title: String) {
        let mut s = self.slot();
        if s.0 == generation {
            s.1 = Some(title);
        }
    }

    pub fn get(&self) -> Option<String> {
        self.slot().1.clone()
    }

    /// The window is closed: its title is forgotten and its late titles are dropped.
    pub fn clear(&self) {
        let mut s = self.slot();
        *s = (s.0 + 1, None);
    }
}

impl Default for PageTitle {
    fn default() -> PageTitle {
        PageTitle::new()
    }
}

/// The one mail window's page title.
pub static PAGE_TITLE: PageTitle = PageTitle::new();

// ---- one mail window at a time (spec §5.4) -----------------------------------------------------

static HELD: AtomicBool = AtomicBool::new(false);

/// Held by whoever drives the mail window (setup, a repair sign-in, the slot's mail step) for the
/// whole of its run; dropping it frees the next. Grades' `Capturing` rule.
pub struct MailWindow(());

impl MailWindow {
    pub fn try_take() -> Option<MailWindow> {
        HELD.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).ok().map(|_| MailWindow(()))
    }

    /// The mail window for a sign-out or a deletion (spec §4.13): taken now when it is free; otherwise
    /// `stop_holder` is called (it stops and closes whatever holds the window) and the hold is asked for
    /// again every 250 ms, with the stop called again on every miss, until `deadline`. `None` once the
    /// deadline passes with the window still held.
    pub fn take_by(deadline: Instant, mut stop_holder: impl FnMut()) -> Option<MailWindow> {
        loop {
            if let Some(w) = MailWindow::try_take() {
                return Some(w);
            }
            stop_holder();
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return None;
            }
            std::thread::sleep(left.min(Duration::from_millis(250)));
        }
    }
}

impl Drop for MailWindow {
    fn drop(&mut self) {
        HELD.store(false, Ordering::SeqCst);
    }
}

// ---- the stop (spec §4.13) ----------------------------------------------------------------------------

static STOPS: AtomicU64 = AtomicU64::new(0);

/// Spec §4.13: a sign-out or a deletion closes the mail windows before it deletes every kept mail profile,
/// and nothing may open one in between. `account`'s `close_mail_windows` calls this before it closes them:
/// a run that began earlier ([`Run`], the slot's mail step) opens no further mailbox's profile. A setup or
/// a resume has its own stop (`setup::SETUPS`), which the same close sets.
pub fn stop_runs() {
    STOPS.fetch_add(1, Ordering::SeqCst);
}

/// When a run that opens mail profiles began, against [`stop_runs`]: taken at its start, asked before
/// every window it opens and again right after.
#[derive(Debug, Clone, Copy)]
pub struct Run(u64);

impl Run {
    pub fn begin() -> Run {
        Run(STOPS.load(Ordering::SeqCst))
    }

    /// Whether [`stop_runs`] was called since this run began: a stopped run opens no mail window.
    pub fn stopped(self) -> bool {
        STOPS.load(Ordering::SeqCst) != self.0
    }
}

// ---- the compiled-in scripts (D19) --------------------------------------------------------------

/// The initialisation script of every mail document (S3): it answers `registerProtocolHandler`
/// with nothing, so the "open email links" prompt never shows. It is not a step script and no name
/// runs it.
pub const INIT_SCRIPT: &str = include_str!("../assets/mail/init.js");

/// One step script: `app/assets/mail/<path>`, compiled in. A script is the body of an async function
/// of `args`; what it returns is its result. One under `m365/` or `gmail/` runs only for that
/// provider; one at the top runs for either.
#[derive(Debug)]
pub struct Script {
    path: &'static str,
    source: &'static str,
}

impl Script {
    pub fn path(&self) -> &'static str {
        self.path
    }
    pub fn source(&self) -> &'static str {
        self.source
    }
    fn runs_for(&self, provider: Provider) -> bool {
        match self.path.split_once('/') {
            None => true,
            Some(("m365", _)) => provider == Provider::M365,
            Some(("gmail", _)) => provider == Provider::Gmail,
            Some(_) => false,
        }
    }
}

macro_rules! compiled {
    ($($path:literal),* $(,)?) => {
        &[$(Script { path: $path, source: include_str!(concat!("../assets/mail/", $path)) }),*]
    };
}

/// Every step script there is. T10 adds each provider's under `m365/` and `gmail/`: so far Microsoft
/// 365's sign-in probe and forwarding scripts (T10.2a) and its mailbox scripts (T10.2b), and Gmail's
/// forwarding scripts (T10.3a) and mailbox scripts (T10.3b), and Gmail's conversation reader (T11.2).
static SCRIPTS: &[Script] = compiled![
    "ready.js",
    "m365/signin.js",
    "m365/forwarding_read.js",
    "m365/forwarding_set.js",
    "m365/forwarding_off.js",
    "m365/search.js",
    "m365/list.js",
    "m365/forward.js",
    "m365/sent_cleanup.js",
    "gmail/forwarding_read.js",
    "gmail/forwarding_add.js",
    "gmail/forwarding_add_done.js",
    "gmail/forwarding_set.js",
    "gmail/forwarding_off.js",
    "gmail/forwarding_remove_address.js",
    "gmail/search.js",
    "gmail/list.js",
    "gmail/forward.js",
    "gmail/sent_cleanup.js",
    "gmail/thread.js",
];

pub fn scripts() -> &'static [Script] {
    SCRIPTS
}

/// The compiled-in script at `path`, exactly as listed; anything else is `None`.
pub fn script(path: &str) -> Option<&'static Script> {
    SCRIPTS.iter().find(|s| s.path == path)
}

// ---- the runner (S1a, S1c) ----------------------------------------------------------------------

/// The page a script runs in: the mail window, or a test's fake.
pub trait Page {
    /// The top-level document's URL, or `None` when there is no page.
    fn url(&self) -> Option<String>;
    /// Evaluates `js` in the page and hands back the completion's JSON value, or `None` when none
    /// came within `wait` (the window went, or the document changed under it).
    fn eval(&self, js: &str, wait: Duration) -> Option<String>;
}

/// Why a script produced no result. Each is a fixed word; none carries page text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No script of that name is compiled in.
    Unknown,
    /// The script belongs to the other provider.
    NotForProvider,
    NoPage,
    /// The page is not on the provider's mail hosts (sign-in, SSO, consent, anywhere else).
    OffOrigin,
    TimedOut,
    /// The page loaded a new document and the result went with the old one.
    Lost,
    /// The script threw.
    Failed,
    Unreadable,
}

impl Refusal {
    pub fn as_str(self) -> &'static str {
        match self {
            Refusal::Unknown => "no such step script",
            Refusal::NotForProvider => "a step script for another provider",
            Refusal::NoPage => "the mail window is not open",
            Refusal::OffOrigin => "the page is not the mailbox",
            Refusal::TimedOut => "the page did not answer in time",
            Refusal::Lost => "the page moved before the step finished",
            Refusal::Failed => "the step script failed",
            Refusal::Unreadable => "the page's answer was unreadable",
        }
    }
}

/// How often a running script's result is asked for.
const POLL: Duration = Duration::from_millis(50);
/// Where a run's result waits in the page, keyed by run id; never enumerable.
const SLOT: &str = "__knowlu_mail";
static RUN_ID: AtomicU64 = AtomicU64::new(1);

/// Runs the compiled-in script `path` in `page` for `provider` and returns its result, within
/// `timeout`. The script is started (its result kept in the page under a fresh run id), then its
/// result is asked for every [`POLL`] until it is there. Before each evaluation the page's URL is
/// checked host-side, and each evaluation checks `location.host` in the page again, so nothing is
/// evaluated on a page off the provider's mail hosts. Call it from a background thread: the window's
/// evaluation completes on the main thread.
pub fn run<P: Page + ?Sized>(page: &P, provider: Provider, path: &str, args: &Value, timeout: Duration) -> Result<Value, Refusal> {
    let script = script(path).ok_or(Refusal::Unknown)?;
    if !script.runs_for(provider) {
        return Err(Refusal::NotForProvider);
    }
    let hosts = serde_json::to_string(mail_hosts(provider)).map_err(|_| Refusal::Unreadable)?;
    let args = serde_json::to_string(args).map_err(|_| Refusal::Unreadable)?;
    let id = RUN_ID.fetch_add(1, Ordering::SeqCst);
    let deadline = Instant::now() + timeout;
    let started = ask(page, provider, &start_js(&hosts, id, script.source, &args), deadline)?;
    match started.get("state").and_then(Value::as_str) {
        Some("started") => {}
        other => return Err(refusal_for(other)),
    }
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(Refusal::TimedOut);
        }
        std::thread::sleep(POLL.min(left));
        let polled = ask(page, provider, &poll_js(&hosts, id), deadline)?;
        match polled.get("state").and_then(Value::as_str) {
            Some("running") => continue,
            Some("done") => return Ok(polled.get("value").cloned().unwrap_or(Value::Null)),
            other => return Err(refusal_for(other)),
        }
    }
}

fn refusal_for(state: Option<&str>) -> Refusal {
    match state {
        Some("refused") => Refusal::OffOrigin,
        Some("lost") => Refusal::Lost,
        Some("failed") => Refusal::Failed,
        _ => Refusal::Unreadable,
    }
}

/// One evaluation behind the host-side origin check. The page answers `JSON.stringify` of an
/// object, which the completion hands back as a JSON string.
fn ask<P: Page + ?Sized>(page: &P, provider: Provider, js: &str, deadline: Instant) -> Result<Value, Refusal> {
    let url = page.url().ok_or(Refusal::NoPage)?;
    if !on_mail_host(provider, &url) {
        return Err(Refusal::OffOrigin);
    }
    let wait = deadline.saturating_duration_since(Instant::now());
    if wait.is_zero() {
        return Err(Refusal::TimedOut);
    }
    let raw = page.eval(js, wait).ok_or(Refusal::TimedOut)?;
    let text: String = serde_json::from_str(&raw).map_err(|_| Refusal::Unreadable)?;
    let value: Value = serde_json::from_str(&text).map_err(|_| Refusal::Unreadable)?;
    if value.is_object() { Ok(value) } else { Err(Refusal::Unreadable) }
}

/// The in-page head every evaluation shares: the host list, the result slot, the run id, and the
/// `location.host` guard.
fn guard(hosts: &str, id: u64) -> String {
    let mut js = String::from("(function(){var H=");
    js.push_str(hosts);
    js.push_str(",K=\"");
    js.push_str(SLOT);
    js.push_str("\",I=");
    js.push_str(&id.to_string());
    js.push_str(";try{if(H.indexOf(location.host)<0){return JSON.stringify({state:\"refused\"});}");
    js
}

const TAIL: &str = "}catch(e){return JSON.stringify({state:\"failed\"});}})()";

/// Starts `source` as the body of an async function of `args`, keeping its settled result under
/// the run id, and answers `started` at once.
fn start_js(hosts: &str, id: u64, source: &str, args: &str) -> String {
    let mut js = guard(hosts, id);
    js.push_str("var s=window[K];if(!s){s={};Object.defineProperty(window,K,{value:s});}s[I]={state:\"running\"};");
    js.push_str("Promise.resolve().then(function(){return (async function(args){\n");
    js.push_str(source);
    js.push_str("\n})(");
    js.push_str(args);
    js.push_str(");}).then(function(v){s[I]={state:\"done\",value:(v===undefined?null:v)};},function(){s[I]={state:\"failed\"};});");
    js.push_str("return JSON.stringify({state:\"started\"});");
    js.push_str(TAIL);
    js
}

/// Asks for run `id`'s result: `running`, the settled result (taken out of the page), or `lost`
/// when the document that ran it is gone.
fn poll_js(hosts: &str, id: u64) -> String {
    let mut js = guard(hosts, id);
    js.push_str("var s=window[K],r=s&&s[I];if(!r){return JSON.stringify({state:\"lost\"});}");
    js.push_str("if(r.state===\"running\"){return JSON.stringify({state:\"running\"});}");
    js.push_str("delete s[I];return JSON.stringify(r);");
    js.push_str(TAIL);
    js
}

// ---- the window (D5; S1b, S2, S3) ---------------------------------------------------------------
//
// Built only on a profile folder `mail_session::is_session_dir` accepts. Visible to sign in; hidden
// and out of the taskbar while steps run (S2 passed both that and off-screen; hidden is grades'
// mode). The mail window has no navigation filter: a school's own sign-in can be on any host, and
// nothing runs there. The popup has one ([`popup_allowed`], S1b).
use crate::mail_session::is_session_dir;
use std::path::Path;
use tauri::webview::{NewWindowFeatures, NewWindowResponse};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// The mail window as a [`Page`]: its URL, and WebView2's script-completion value (S1a).
pub struct Window(pub tauri::WebviewWindow);

impl Page for Window {
    fn url(&self) -> Option<String> {
        self.0.url().ok().map(|u| u.to_string())
    }
    fn eval(&self, js: &str, wait: Duration) -> Option<String> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.0.eval_with_callback(js, move |raw| drop(tx.send(raw))).ok()?;
        rx.recv_timeout(wait).ok()
    }
}

/// [`run`] in the open mail window.
pub fn run_in_window(app: &tauri::AppHandle, provider: Provider, path: &str, args: &Value, timeout: Duration) -> Result<Value, Refusal> {
    let w = app.get_webview_window(WINDOW).ok_or(Refusal::NoPage)?;
    run(&Window(w), provider, path, args, timeout)
}

/// The open mail window's page title ([`PAGE_TITLE`]); `None` with no mail window.
pub fn page_title(app: &tauri::AppHandle) -> Option<String> {
    app.get_webview_window(WINDOW)?;
    PAGE_TITLE.get()
}

/// [`signed_in`] for the open mail window, now: its URL and its page's title.
pub fn window_signed_in(app: &tauri::AppHandle, provider: Provider) -> bool {
    let Some(w) = app.get_webview_window(WINDOW) else { return false };
    let url = w.url().ok().map(|u| u.to_string());
    signed_in(provider, url.as_deref(), &page_title(app).unwrap_or_default())
}

/// The sign-in window: `mail`, visible and centred, at the provider's mailbox, on `profile`.
pub fn open_visible(app: &tauri::AppHandle, data_dir: &Path, profile: &Path, provider: Provider) -> Result<(), String> {
    open(app, data_dir, profile, provider, true)
}

/// The same window for a saved sign-in's run (the slot's mail step, a resumed backfill): hidden.
pub fn open_hidden(app: &tauri::AppHandle, data_dir: &Path, profile: &Path, provider: Provider) -> Result<(), String> {
    open(app, data_dir, profile, provider, false)
}

/// The mail window's WebView2 arguments: wry 0.55's defaults (setting the window's arguments replaces
/// them, so they are repeated) and Chromium 155's `AvoidCorsURLLoaderRestartOnRedirect` turned off. With it on, a
/// same-origin request redirected cross-site loses its Origin header while the host intercepts requests:
/// live (2026-10-19, T18c) Gmail's "Forward as attachment" fetch of the message, which mail.google.com
/// redirects to mail-attachment.googleusercontent.com, got no Access-Control-Allow-Origin and failed.
pub const MAIL_BROWSER_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,AvoidCorsURLLoaderRestartOnRedirect";

fn open(app: &tauri::AppHandle, data_dir: &Path, profile: &Path, provider: Provider, visible: bool) -> Result<(), String> {
    if app.get_webview_window(WINDOW).is_some() {
        return Err("the mail window is already open".into());
    }
    if !is_session_dir(data_dir, profile) {
        return Err("not a mail profile folder".into());
    }
    let url: tauri::Url = start_url(provider)
        .ok_or_else(|| "this provider is set up by hand".to_string())?
        .parse()
        .map_err(|_| "the mailbox address is not a URL".to_string())?;
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let opener = app.clone();
    let generation = PAGE_TITLE.begin();
    let builder = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(url))
        .title(if visible { "Sign in to your email" } else { "Knowlu email setup" })
        .inner_size(1000.0, 760.0)
        .center()
        .visible(visible)
        .focused(visible)
        .skip_taskbar(!visible)
        .data_directory(profile.to_path_buf())
        .initialization_script(INIT_SCRIPT)
        .on_new_window(move |url, features| open_popup(&opener, provider, &url, features))
        .on_document_title_changed(move |w, title| {
            if w.label() == WINDOW {
                PAGE_TITLE.set(generation, title);
            }
        });
    // A debug build's diagnostic port on top of the mail window's own arguments, never a release's
    // (`crate::debug_port`).
    #[cfg(debug_assertions)]
    let args = crate::debug_port::mail_browser_args();
    #[cfg(not(debug_assertions))]
    let args: Option<String> = None;
    let builder = builder.additional_browser_args(args.as_deref().unwrap_or(MAIL_BROWSER_ARGS));
    let w = builder.build().map_err(|e| e.to_string())?;
    refuse_external_schemes(&w);
    Ok(())
}

/// S1b: Google's "verify it's you" popup, in the opener's environment (so its profile), with its
/// own window features. Its first request is `about:blank`; only [`popup_allowed`] URLs open, one
/// popup at a time, and the mail window comes back with it, because the site needs the student.
/// Every navigation in the popup passes [`popup_allowed`] too, and it opens no popup of its own.
/// It closes itself (`window.close()`): [`destroy_on_close_request`] destroys its Tauri window.
fn open_popup(app: &tauri::AppHandle, provider: Provider, url: &tauri::Url, features: NewWindowFeatures) -> NewWindowResponse<tauri::Wry> {
    if !popup_allowed(provider, url.as_str()) || app.get_webview_window(POPUP).is_some() {
        return NewWindowResponse::Deny;
    }
    let Ok(blank) = "about:blank".parse::<tauri::Url>() else { return NewWindowResponse::Deny };
    let built = WebviewWindowBuilder::new(app, POPUP, WebviewUrl::External(blank))
        .window_features(features)
        .title("Confirm it's you")
        // The popup shows the number the student matches on their phone (live, 2026-10-18), so it is the
        // topmost, focused window while it is open, above the mail window that comes back with it.
        .always_on_top(true)
        .focused(true)
        .initialization_script(INIT_SCRIPT)
        .on_navigation(move |u| popup_allowed(provider, u.as_str()))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build();
    match built {
        Ok(window) => {
            refuse_external_schemes(&window);
            destroy_on_close_request(app, &window);
            let _ = come_back(app);
            // After come_back, which focuses the mail window: the popup takes focus back, or the number it
            // shows sits behind the mail window and the student never sees it.
            let _ = window.set_focus();
            NewWindowResponse::Create { window }
        }
        Err(_) => NewWindowResponse::Deny,
    }
}

/// Whether Google's popup is open (the step waits while it is).
pub fn popup_open(app: &tauri::AppHandle) -> bool {
    app.get_webview_window(POPUP).is_some()
}

/// D5: once signed in, the window steps aside: hidden and out of the taskbar (S2).
pub fn step_aside(app: &tauri::AppHandle) -> Result<(), String> {
    let w = app.get_webview_window(WINDOW).ok_or_else(|| Refusal::NoPage.as_str().to_string())?;
    w.hide().map_err(|e| e.to_string())?;
    w.set_skip_taskbar(true).map_err(|e| e.to_string())
}

/// D5: the site needs the student (a popup, an MFA or SSO prompt, a consent page): the window comes
/// back, centred and focused.
pub fn come_back(app: &tauri::AppHandle) -> Result<(), String> {
    let w = app.get_webview_window(WINDOW).ok_or_else(|| Refusal::NoPage.as_str().to_string())?;
    w.set_skip_taskbar(false).map_err(|e| e.to_string())?;
    w.show().map_err(|e| e.to_string())?;
    let _ = w.unminimize();
    w.center().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())
}

/// Closes the popup and the mail window. The profile folder is `mail_session`'s to delete, which
/// retries while WebView2 lets go of it (S2: 0.8 to 1.3 s).
pub fn close(app: &tauri::AppHandle) {
    for label in [POPUP, WINDOW] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.destroy();
        }
    }
    PAGE_TITLE.clear();
}

/// S3: a page's launch of an external scheme (`mailto:`, an app's own) is cancelled, so no
/// "open this link" prompt and no other app ever opens from a mail window.
#[cfg(windows)]
fn refuse_external_schemes(w: &tauri::WebviewWindow) {
    let _ = w.with_webview(|pw| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_18;
        use windows_core::Interface;
        let Ok(core) = pw.controller().CoreWebView2() else { return };
        let Ok(core) = core.cast::<ICoreWebView2_18>() else { return };
        let handler = webview2_com::LaunchingExternalUriSchemeEventHandler::create(Box::new(|_, args| {
            if let Some(args) = args {
                args.SetCancel(true)?;
            }
            Ok(())
        }));
        let mut token = 0i64;
        let _ = core.add_LaunchingExternalUriScheme(&handler, &mut token);
    });
}

#[cfg(not(windows))]
fn refuse_external_schemes(_w: &tauri::WebviewWindow) {}

/// S1b: the popup's `window.close()` destroys its Tauri window. wry's own handler destroys only the
/// webview's container, which left an empty frame in the spike (and [`popup_open`] true). The
/// destroy runs off the WebView2 callback, on its own thread, as the spike verified.
#[cfg(windows)]
fn destroy_on_close_request(app: &tauri::AppHandle, w: &tauri::WebviewWindow) {
    let app = app.clone();
    let _ = w.with_webview(move |pw| unsafe {
        let Ok(core) = pw.controller().CoreWebView2() else { return };
        let handler = webview2_com::WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
            let app = app.clone();
            std::thread::spawn(move || {
                if let Some(popup) = app.get_webview_window(POPUP) {
                    let _ = popup.destroy();
                }
            });
            Ok(())
        }));
        let mut token = 0i64;
        let _ = core.add_WindowCloseRequested(&handler, &mut token);
    });
}

#[cfg(not(windows))]
fn destroy_on_close_request(_app: &tauri::AppHandle, _w: &tauri::WebviewWindow) {}

// ---- the ten console commands (spec section 6.2; email plan T11.4) -------------------------------------
//
// Thin wrappers: each gathers what its seam in [`commands`] needs from `ConsoleState` and the live window and
// calls it, and computes nothing. They are named in the console's `generate_handler!` list and not in the
// wizard's (H5, the controller's hand-off): the wizard has no profile folder and no vault. A command that
// waits on a window is `async`, so it never blocks the main thread (grades' rule).
use crate::state::ConsoleState;
use tauri::State;

/// The service's mailbox rows, or the name of why they could not be read.
fn listed(vault: &Path) -> Result<Vec<Value>, String> {
    live::client_for(vault).map_err(String::from).and_then(|c| c.list_addresses().map_err(|e| e.name().to_string()))
}

fn row_of(rows: Vec<Value>, id: &str) -> Option<Value> {
    rows.into_iter().find(|r| r["address_id"].as_str() == Some(id))
}

fn setup_running() -> bool {
    setup::SETUPS.current().is_some_and(|s| !s.is_done())
}

/// The mail window, waited for up to a minute (a setup or resume being stopped lets go of it).
fn take_window() -> Option<MailWindow> {
    for _ in 0..240 {
        if let Some(w) = MailWindow::try_take() {
            return Some(w);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    None
}

#[tauri::command(async)]
pub fn mail_status(cs: State<'_, ConsoleState>) -> Value {
    commands::status_for(listed(&cs.vault), &cs.data_dir, setup_running())
}

#[tauri::command(async)]
pub fn mail_setup_start(app: tauri::AppHandle, cs: State<'_, ConsoleState>, provider: String) -> Value {
    let unitid = Some(crate::grades::campus_of(&cs.vault).unitid).filter(|u| !u.is_empty());
    commands::setup_start_with(&cs.data_dir, &provider, unitid, |kind| live::start_setup(&app, kind).map(|_| ()))
}

#[tauri::command]
pub fn mail_setup_progress() -> Value {
    commands::progress_of(setup::SETUPS.current())
}

#[tauri::command]
pub fn mail_setup_answer(choice: Value) -> Value {
    commands::answer_with(setup::SETUPS.current(), &choice)
}

#[tauri::command]
pub fn mail_setup_stop() -> Value {
    commands::stop_with(setup::SETUPS.current())
}

#[tauri::command(async)]
pub fn mail_sign_in(app: tauri::AppHandle, cs: State<'_, ConsoleState>, address_id: String) -> Value {
    let rows = match listed(&cs.vault) {
        Ok(rows) => rows,
        Err(name) => return json!({ "ok": false, "signed_in": false, "error": name }),
    };
    let Some(row) = row_of(rows, &address_id) else { return json!({ "ok": false, "signed_in": false, "error": "no such mailbox" }) };
    if crate::mail_session::MailState::load(&cs.data_dir).is_ok_and(|s| s.is_turned_off(&address_id)) {
        return json!({ "ok": false, "signed_in": false, "error": "email is turned off for this mailbox" });
    }
    let provider = Provider::from_service(row["provider"].as_str().unwrap_or(""));
    let Some(_window) = MailWindow::try_take() else { return json!({ "ok": false, "signed_in": false, "error": "a mail window is already open" }) };
    let host = live::WindowHost::new(app.clone(), cs.data_dir.clone());
    commands::sign_in_with(&host, &recover::SystemClock, &cs.data_dir, provider, &address_id)
}

#[tauri::command(async)]
pub fn mail_disconnect(app: tauri::AppHandle, cs: State<'_, ConsoleState>, address_id: String) -> Value {
    // Step 0 is first here as well as in the seam: the window below can take a while to be had, and nothing
    // in that while may turn this mailbox's forwarding back on.
    if crate::mail_session::turn_off(&cs.data_dir, &address_id).is_err() {
        return json!({ "ok": false, "error": "could not record the mailbox as turned off; nothing was changed", "steps": [], "needs_you": false, "address": Value::Null });
    }
    if let Some(run) = setup::SETUPS.current() {
        run.stop();
    }
    let Some(_window) = take_window() else {
        return json!({ "ok": false, "error": "a mail window is already open", "steps": [], "needs_you": false, "address": Value::Null });
    };
    let client = match live::client_for(&cs.vault) {
        Ok(c) => c,
        Err(name) => return json!({ "ok": false, "error": name, "steps": [], "needs_you": false, "address": Value::Null }),
    };
    let host = live::WindowHost::new(app.clone(), cs.data_dir.clone());
    commands::disconnect_with(&commands::Disconnect { host: &host, client: &client, clock: &recover::SystemClock, data_dir: &cs.data_dir }, &address_id)
}

#[tauri::command(async)]
pub fn mail_rotate(app: tauri::AppHandle, cs: State<'_, ConsoleState>, address_id: String) -> Value {
    let row = listed(&cs.vault).ok().and_then(|rows| row_of(rows, &address_id));
    commands::rotate_with(&cs.data_dir, &address_id, row.as_ref(), |kind| live::start_setup(&app, kind).map(|_| ()))
}

#[tauri::command(async)]
pub fn mail_keep_session(app: tauri::AppHandle, cs: State<'_, ConsoleState>, on: bool) -> Value {
    commands::keep_session_with(&cs.data_dir, on, || {
        if let Some(run) = setup::SETUPS.current() {
            run.stop();
        }
        close(&app);
    })
}

#[tauri::command(async)]
pub fn mail_applied(cs: State<'_, ConsoleState>, since: Option<String>) -> Value {
    commands::applied_with(&cs.vault, since.as_deref(), commands::run_engine)
}
