//! The LMS calendar link, captured through a sign-in window (spec §11a).
//!
//! **Knowlu never asks for a campus credential.** The student signs in on the university's own page,
//! in a window that has *no capability grant* — so that page cannot reach a single Tauri command —
//! and in incognito mode, so the live session lives in memory rather than in a profile on disk. We
//! keep the calendar link. We keep nothing else: not the password, not the cookies, not the session.
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use serde_json::{json, Value};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// The window's label. One at a time, by construction: `open_window_at` shows the window already
/// carrying this label rather than building a second (R-C1-41, M5).
pub const WINDOW: &str = "lms-signin";

/// Every session directory this module makes starts with this, and nothing else it deletes does.
const SESSION_PREFIX: &str = "knowlu-lms-session-";

/// How long a leftover session directory has to have been sitting in the temp folder before the
/// sweep takes it. An hour, because a second Knowlu part-way through its own capture is not ours to
/// delete and a student can spend a long time on a campus SSO page.
const STALE_AFTER: Duration = Duration::from_secs(60 * 60);

/// WebView2's browser process exits asynchronously and keeps the profile open for a moment after the
/// window is gone, so the first `remove_dir_all` usually loses. Twelve tries at 250 ms is three
/// seconds of patience, spent on a detached thread where nobody is waiting for it.
const WIPE_TRIES: u32 = 12;
const WIPE_BACKOFF: Duration = Duration::from_millis(250);

/// At `RunEvent::Exit` there is no thread left to detach to, so the same wipe runs inline on a much
/// shorter leash. Whatever it cannot finish, the next launch's sweep takes.
const EXIT_TRIES: u32 = 4;

/// **Not under `%LOCALAPPDATA%\knowlu`.** A campus session's cookies are not app data — they are
/// somebody's live login — so the window is built incognito (its session lives in memory) and
/// whatever WebView2 writes anyway goes to a throwaway directory named for this process and this
/// capture. It is deleted when the window is destroyed, again at exit, and by the next launch's
/// sweep if this process died before it could. `app/tests/lms_link.rs` pins that this path is under
/// the system temp folder and is neither the app's own data root nor anything inside a profile.
pub fn session_dir() -> PathBuf {
    std::env::temp_dir().join(format!("{SESSION_PREFIX}{}-{}", std::process::id(), knowlu_engine::ids::new_id("cap")))
}

/// The one shape [`wipe_dir`] will delete: a `knowlu-lms-session-*` directory sitting directly in the
/// system temp folder. `close_lms_window` takes no path at all any more (R-C1-40, I2) — this is the
/// second lock on the same door, so a recursive delete in this module can never be aimed anywhere
/// but at a directory this module made.
pub fn is_session_dir(dir: &Path) -> bool {
    let temp = std::env::temp_dir();
    dir.parent() == Some(temp.as_path())
        && dir.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(SESSION_PREFIX))
}

/// Delete a session directory, patiently. Returns once it is gone or the tries run out; a path that
/// is not one of ours is refused outright rather than deleted.
fn wipe_dir(dir: &Path, tries: u32) -> bool {
    if !is_session_dir(dir) { return false; }
    for attempt in 0..tries {
        if !dir.exists() { return true; }
        if std::fs::remove_dir_all(dir).is_ok() { return true; }
        if attempt + 1 < tries { std::thread::sleep(WIPE_BACKOFF); }
    }
    !dir.exists()
}

/// The directory the open sign-in window is using, held by the app rather than by the page.
///
/// **The page is not the only thing that ends a capture** (R-C1-40, I1): the window's own close
/// button, a crash of the campus page and app exit all end one too, and none of them calls a
/// command. So the app remembers the path and wipes it from the window's `Destroyed` event and again
/// on the way out — `take_session` empties the slot, which is what makes all three idempotent.
#[derive(Default)]
pub struct LmsSession(std::sync::Mutex<Option<PathBuf>>);

fn remember_session(app: &tauri::AppHandle, dir: &Path) {
    if let Some(state) = app.try_state::<LmsSession>() {
        if let Ok(mut held) = state.0.lock() { *held = Some(dir.to_path_buf()); }
    }
}

fn take_session(app: &tauri::AppHandle) -> Option<PathBuf> {
    let state = app.try_state::<LmsSession>()?;
    let mut held = state.0.lock().ok()?;
    held.take()
}

/// Wipe the session directory the app is holding, on a detached thread. This runs from a window
/// event, and WebView2 still has the profile open at that moment — waiting three seconds on the
/// event loop's thread would freeze the wizard behind it.
pub fn wipe_session(app: &tauri::AppHandle) {
    if let Some(dir) = take_session(app) {
        std::thread::spawn(move || { wipe_dir(&dir, WIPE_TRIES); });
    }
}

/// The same wipe on the way out, inline and on a short leash: after `RunEvent::Exit` a detached
/// thread dies with the process. Whatever this cannot finish, the next launch's sweep takes.
///
/// **The second pass is not belt and braces** (R-C1-41, M2). When `Destroyed` is delivered before
/// `RunEvent::Exit` — which is the `app.exit(0)` path the wizard's relaunch takes — the handler has
/// already emptied the slot and handed the path to a thread the process is about to kill, so
/// `take_session` finds nothing here and the directory would survive until some later launch
/// happened to sweep it. Sweeping **this process's own** prefix at zero age closes that race: by the
/// time the event loop is exiting, every session directory named for this pid is finished with.
pub fn wipe_session_on_exit(app: &tauri::AppHandle) {
    if let Some(dir) = take_session(app) { wipe_dir(&dir, EXIT_TRIES); }
    sweep_stale_sessions_in(&std::env::temp_dir(), &mine_prefix(), None, SystemTime::now(), Duration::ZERO);
}

/// The run of a session directory's name that marks it as this process's.
fn mine_prefix() -> String {
    format!("{SESSION_PREFIX}{}-", std::process::id())
}

/// The sweep's testable core. `take` is the name prefix to consider and `skip` the one to leave
/// alone; `now` and `older_than` are the clock, so a test drives it without a second process and
/// without waiting an hour. Returns how many it removed. `take` must itself begin with
/// [`SESSION_PREFIX`] — the guard that keeps this from ever being pointed at the rest of the temp
/// folder — and the only production callers pass the system temp folder.
///
/// **Two edges, left as they are on purpose** (R-C1-41, M6). (a) The age test reads the *directory's
/// own* mtime, which on Windows does not move when files inside its subdirectories are written, so a
/// second Knowlu whose student has been on an SSO page for over an hour looks stale to the crash
/// sweep; deleting a live WebView2 profile mostly fails on open handles, and the cost of getting it
/// wrong is a capture that has to be restarted, not lost data. (b) `skip` is keyed on the pid, so a
/// leftover from a dead process that happened to hold this pid is never swept — it waits for the
/// next launch with a different pid. Both are cheaper to name than to fix.
pub fn sweep_stale_sessions_in(temp: &Path, take: &str, skip: Option<&str>, now: SystemTime, older_than: Duration) -> usize {
    if !take.starts_with(SESSION_PREFIX) { return 0; }
    let Ok(entries) = std::fs::read_dir(temp) else { return 0 };
    let mut swept = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with(take) || skip.is_some_and(|s| name.starts_with(s)) { continue; }
        let stale = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age >= older_than);
        if stale && std::fs::remove_dir_all(entry.path()).is_ok() { swept += 1; }
    }
    swept
}

/// Session directories a crash left behind — the only thing that ever cleans up after a process that
/// died with a sign-in window open (R-C1-40, I1c). Not this process's own, and only ones that have
/// been sitting there for [`STALE_AFTER`]: another Knowlu, mid-capture, is not ours to delete.
/// Also called by `registrar::open_registrar_window`.
pub fn sweep_stale_sessions() {
    let mine = mine_prefix();
    sweep_stale_sessions_in(&std::env::temp_dir(), SESSION_PREFIX, Some(&mine), SystemTime::now(), STALE_AFTER);
}

/// Open the sign-in window, or show the one that is already open.
///
/// **The second *Sign in* click used to surface a Tauri internal string** (R-C1-41, M5): a close is
/// posted asynchronously while `build` refuses the still-registered label on this thread, so the page
/// got "a webview with label `lms-signin` already exists". The window the student already has is the
/// right answer, so it is shown and focused — and *not* closed on the way, which would have handed
/// them a window that vanished a moment after being told it was open. That also keeps the "one at a
/// time" guarantee: this is the only thing that builds the window, and it never builds a second.
pub fn open_window_at(app: &tauri::AppHandle, url: &str, data_dir: &Path) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(WINDOW) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Ok(());
    }
    let parsed: tauri::Url = url.parse().map_err(|e| format!("{url}: {e}"))?;
    let _ = std::fs::create_dir_all(data_dir);
    WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(parsed))
        .title("Sign in to your school")
        .inner_size(1000.0, 760.0)
        .center()
        // **Incognito** (R-C1-40, I1a): the student's campus session lives in memory and goes when
        // the webview does. `data_directory` stays for whatever WebView2 writes regardless — a
        // cache, a crash dump — which is what the wipe and the sweep below are for.
        .incognito(true)
        .data_directory(data_dir.to_path_buf())
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())?;
    // Only a builder that managed [`LmsSession`] can be wiped by the window's own close button; on
    // one that did not, this is a no-op and the sweep is what eventually takes the directory. Today
    // that is `run_shell`, which is the only builder these commands are registered on.
    remember_session(app, data_dir);
    Ok(())
}

pub fn window_url(app: &tauri::AppHandle) -> Option<String> {
    app.get_webview_window(WINDOW).and_then(|w| w.url().ok()).map(|u| u.to_string())
}

pub fn navigate(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    let w = app.get_webview_window(WINDOW).ok_or_else(|| "the sign-in window is not open".to_string())?;
    let parsed: tauri::Url = url.parse().map_err(|e| format!("{url}: {e}"))?;
    w.navigate(parsed).map_err(|e| e.to_string())
}

/// Close the window and delete the session directory **the app is holding** — not one a caller
/// names. Both halves are best effort: a window the student already closed is not an error, and the
/// directory goes to whichever of this call, the window's `Destroyed` event or `RunEvent::Exit` gets
/// there first, because `take_session` empties the slot. The delete itself is retried on a detached
/// thread, because WebView2 keeps the profile open for a moment after the window is gone; a wipe
/// that still loses is swept by the next launch.
pub fn close_and_wipe(app: &tauri::AppHandle) {
    close_window(app);
    wipe_session(app);
}

fn close_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(WINDOW) {
        let _ = w.close();
    }
}

/// The campus's capture path, recorded verbatim by Task 13's spike. `None` for a school nobody has
/// curated — which is most of the 4,319 — and that is what makes the panel show its paste field
/// instead of pretending.
/// Where the sign-in window opens for a curated school: **that school's own host**, never a constant.
pub fn lms_home(unitid: &str) -> Option<String> {
    crate::scaffold::curated(unitid).map(|c| format!("https://{}/", c.lms_host))
}

#[tauri::command(async)]
pub fn open_lms_window(app: tauri::AppHandle, unitid: String) -> Value {
    // Before anything else: whatever a crashed run left in the temp folder (R-C1-40, I1c).
    sweep_stale_sessions();
    let Some(url) = lms_home(&unitid) else {
        return json!({ "ok": false, "error": "we do not know your school's sign-in page yet — paste your calendar link below", "opened": false });
    };
    let dir = session_dir();
    match open_window_at(&app, &url, &dir) {
        // **The path is not in the envelope** (R-C1-40, I2): the page has no use for it now that
        // closing takes no argument, and a temp path handed to a page is a temp path that comes back.
        Ok(()) => json!({ "ok": true, "error": Value::Null, "opened": true }),
        Err(e) => json!({ "ok": false, "error": e, "opened": false }),
    }
}

/// **No path from the page** (R-C1-40, I2). This used to take the session directory as a string and
/// hand it straight to `remove_dir_all` — a recursive delete aimed by whatever called it, sitting on
/// the same IPC surface as the vault and backup paths. The app knows which directory is open; the
/// page does not need to, and an extra argument it still sends is simply not read.
#[tauri::command(async)]
pub fn close_lms_window(app: tauri::AppHandle) -> Value {
    close_and_wipe(&app);
    json!({ "ok": true, "error": Value::Null })
}

/// What a capture produced, in the words the panel shows: *we found N assignments across M courses.*
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct IcsLink {
    pub url: String,
    pub events: usize,
    pub courses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// The student closed the window, or the deadline passed with no link.
    NotFound,
    NotACalendarLink(String),
    Unreachable(String),
    /// The feed fetched and held nothing. An empty parse is a failure, never an empty semester.
    Empty,
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::NotFound => write!(f, "no calendar link was found — paste it below instead"),
            CaptureError::NotACalendarLink(u) => write!(f, "{u} does not look like a calendar feed link"),
            CaptureError::Unreachable(e) => write!(f, "the feed could not be read ({e})"),
            CaptureError::Empty => write!(f, "that feed is empty — check you copied the whole link"),
        }
    }
}

/// `webcal://` → `https://`, trimmed; everything else is handed back as it came (R-C1-22, and
/// R-C1-41's I2). It is the same feed over the same scheme underneath, and every calendar app that
/// accepts a `webcal://` link already does this rewrite silently — refusing it would be a wizard
/// failing on the one format Google and Outlook actually hand out *Copy public URL* as.
///
/// **One rule, one implementation**: `onboarding::normalize_personal_calendar` calls this rather than
/// keeping its own copy, so the two device paths to the same `calendar_ics` value cannot disagree.
/// The judgement about what to do with the result differs and stays where it is — a blank personal
/// calendar is `None` there, while a blank link here is refused by name by `looks_like_ics`.
pub fn https_from_webcal(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.strip_prefix("webcal://") {
        Some(rest) => format!("https://{rest}"),
        None => trimmed.to_string(),
    }
}

/// https, and either an `.ics` path or a query that says so. `http://` is refused outright: a feed
/// URL is a capability — anyone holding it reads the student's schedule — and sending one in the
/// clear on a campus network is not a thing to do once.
pub fn looks_like_ics(url: &str) -> bool {
    if !url.starts_with("https://") { return false; }
    let lower = url.to_ascii_lowercase();
    let path = lower.split('?').next().unwrap_or(&lower);
    path.ends_with(".ics") || lower.contains("format=ics") || lower.contains("/calendar/feed")
}

/// The first `https://…` run in `text` that looks like a feed. Works on a bare URL, on an address
/// bar's value and on a scrap of HTML, because all three are what the capture might hand it.
pub fn first_ics_link(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some(at) = rest.find("https://") {
        let tail = &rest[at..];
        let end = tail.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>').unwrap_or(tail.len());
        let candidate = &tail[..end];
        if looks_like_ics(candidate) { return Some(candidate.to_string()); }
        rest = &tail[end.max(1)..];
    }
    None
}

/// `(events, courses)`. Courses are counted by the `AAAA 000` code at the head of a `SUMMARY:`, which
/// is how every feed this product has met names them; a feed that names them otherwise reports 0
/// courses and its real event count, which is still a true sentence to show.
pub fn summarise(ics: &str) -> (usize, usize) {
    let mut events = 0usize;
    let mut codes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in ics.lines() {
        let line = line.trim_end_matches('\r');
        if line == "BEGIN:VEVENT" {
            events += 1;
        } else if let Some(rest) = line.strip_prefix("SUMMARY:") {
            let mut it = rest.split_whitespace();
            if let (Some(a), Some(b)) = (it.next(), it.next()) {
                let alpha = a.len() >= 2 && a.len() <= 4 && a.chars().all(|c| c.is_ascii_uppercase());
                let digits = b.len() == 3 && b.chars().all(|c| c.is_ascii_digit());
                if alpha && digits { codes.insert(format!("{a} {b}")); }
            }
        }
    }
    (events, codes.len())
}

/// Fetch it once and say what is in it. `fetch` is a parameter so the tests never touch a network;
/// production passes `knowlu_engine::calfeed::fetch_ics`, which is **the same fetcher `ingest` uses**
/// — a link that validates here is a link that works on the next slot, and that is the point.
pub fn validate(url: &str, fetch: &dyn Fn(&str) -> Result<String, String>) -> Result<IcsLink, CaptureError> {
    if !looks_like_ics(url) { return Err(CaptureError::NotACalendarLink(url.to_string())); }
    let text = fetch(url).map_err(CaptureError::Unreachable)?;
    let (events, courses) = summarise(&text);
    if events == 0 { return Err(CaptureError::Empty); }
    Ok(IcsLink { url: url.to_string(), events, courses })
}

/// **Which URL the sign-in window's cookies are read for** (R-C1-41, I1). The window is a full
/// browser on a campus SSO chain: when the panel asks for a capture the student may still be sitting
/// on the identity provider, on Duo, or anywhere else they wandered — and `cookies_for_url` answers
/// for whatever origin it is handed. Sending that jar to the LMS would hand one party's live session
/// to another, which is not a thing to do once, so **the window's own URL is used only when it is on
/// the feed endpoint's host**, and the feed URL itself otherwise: a completed SSO leaves the LMS
/// host's cookies at path `/`, which is exactly what the request needs.
///
/// Strings in and out, so the rule is pinned by a test with no window and no URL type at the call
/// site. A window URL that will not parse, or that has no host, chooses the feed.
pub fn cookie_url(window_url: Option<&str>, feed: &str) -> String {
    let host = |u: &str| u.parse::<tauri::Url>().ok().and_then(|p| p.host_str().map(str::to_lowercase));
    let Some(here) = window_url else { return feed.to_string() };
    let (Some(a), Some(b)) = (host(here), host(feed)) else { return feed.to_string() };
    if a == b { here.to_string() } else { feed.to_string() }
}

/// A capture that did not produce a link, in the **same keys `finish` uses** (R-C1-41, M4): a panel
/// reading `kind` or `note` must never find `undefined` on the paths where it is telling the student
/// something went wrong. A capture is always a school feed, so the kind is not in doubt.
fn capture_failed(error: impl std::fmt::Display) -> Value {
    json!({ "ok": false, "error": error.to_string(), "kind": "lms_ics", "link": Value::Null, "note": Value::Null, "stored": false })
}

/// **One agent, one shape, for every capture** (the calendar link, the course list, and phase 3's
/// registrar).
/// Thirty seconds global, because a campus behind SSO is slow and a student is watching; no jar of
/// its own, because the only cookies either capture sends are the ones the window hands it for that
/// one call. `put_source_at` builds its own on purpose — it talks to our API, not to a campus, and
/// it reads a status code rather than treating one as an error.
pub fn session_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into()
}

/// **Outcome B — cookie handover** (Task 13's spike, recorded and reviewed). The window is never
/// navigated: the endpoint below, fetched once with the student's own session cookies, answers 200
/// with the feed URL as the entire body.
#[tauri::command(async)]
pub fn capture_calendar_link(app: tauri::AppHandle, unitid: String) -> Value {
    let Some(steps) = capture_steps(&unitid) else {
        return capture_failed("no capture is known for that school yet — paste the link instead");
    };
    let Some(w) = app.get_webview_window(WINDOW) else {
        return capture_failed("the sign-in window is not open");
    };
    // The feed endpoint Task 13 recorded, fetched ONCE with the student's own session cookies. The
    // cookies live in this function's stack and nowhere else: never written, never logged, and gone
    // when it returns.
    let feed = &steps[steps.len() - 1];
    let Ok(url) = feed.parse::<tauri::Url>() else {
        return capture_failed("the campus feed URL is not a URL");
    };
    // The spike read them for the window's own current URL, having landed on the campus's
    // institution page after SSO; `cookie_url` keeps that read and adds the one rule it was missing.
    let here = w.url().ok();
    // Either the feed URL, which parsed above, or a window URL `cookie_url` only chose after parsing
    // it — so the fallback here is unreachable rather than load-bearing.
    let jar_url = cookie_url(here.as_ref().map(|u| u.as_str()), feed).parse::<tauri::Url>().unwrap_or(url);
    let jar: String = match w.cookies_for_url(jar_url) {
        Ok(cs) => cs.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; "),
        Err(e) => return capture_failed(format!("the sign-in could not be read ({e})")),
    };
    let agent = session_agent();
    let body = agent.get(feed.as_str()).header("cookie", &jar).call()
        .and_then(|mut r| r.body_mut().with_config().limit(1 << 22).read_to_string())
        .map_err(|e| e.to_string());
    match body {
        Ok(text) => match first_ics_link(&text) {
            Some(found) => finish("lms_ics", &found),
            // The commonest reason for a 200 with no link in it is a jar that does not authenticate
            // the feed — the student is still part-way through signing in — so say that rather than
            // "no calendar link was found", which reads as *your school has none*.
            None => capture_failed("finish signing in to your school first, then try again — or paste the link below"),
        },
        Err(e) => capture_failed(CaptureError::Unreachable(e)),
    }
}

/// The calendar-share path Task 13's spike recorded, on **this** school's host. A constant host per
/// LMS kind would send the third curated Blackboard school's student to the first one's LMS — which is
/// the assumption this whole task exists to stop making (`CLAUDE.md`'s first rule).
///
/// **Only the last element is Task 13's** (R-C1-41, M3). The spike recorded the feed endpoint and
/// nothing else — outcome B never navigates the window, so the calendar page URL is moot: it is where
/// a person would go to do this by hand, kept as orientation. The capture reads `steps[len - 1]`,
/// always. `None` for a school nobody has curated yet, which is what makes the panel show its paste
/// field instead of pretending.
fn capture_steps(unitid: &str) -> Option<Vec<String>> {
    let c = crate::scaffold::curated(unitid)?;
    // <Task 13 Outcome: the PATHS, verbatim — the host comes from the row>
    let paths: &[&str] = match c.lms_kind {
        "blackboard" => &["/ultra/calendar", "/webapps/calendar/calendarFeed/url"],
        "canvas" => &["/calendar", "/api/v1/users/self"],
        _ => return None,
    };
    Some(paths.iter().map(|path| format!("https://{}{path}", c.lms_host)).collect())
}

/// One course the student is enrolled in, as the LMS names it and as the vault will.
///
/// **The four fields `scaffold::CourseSeed` carries, by the same names.** The panel hands what
/// `capture_courses` returned straight back as the wizard plan's `courses:`, so these two structs are
/// one shape; `a_captured_course_is_exactly_what_the_wizard_plan_takes_back` pins the round trip, and
/// that nothing else rides along on it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Course {
    /// The LMS's own key — Blackboard's `courseId`, Canvas's `course_code`.
    pub code: String,
    /// What the student sees in their LMS.
    pub name: String,
    /// `cs-100`. The vault's own name for it: the note's stem, and every task's `course:` field.
    pub slug: String,
    /// R-C1c-plan-2: the human course code, or empty. The one place a code is read out of an id or
    /// a name is `courses_from_json`; every reader downstream takes it from here.
    #[serde(default)]
    pub label: String,
}

/// Both shapes, one reader (spec §11a R-OB-2). Blackboard Ultra answers
/// `{"results":[{"courseId":…,"course":{"name":…}}]}` — Task 13's second answer, run against a real
/// enrolment. **Canvas is unverified**: no Canvas login existed to spike against, so its bare array of
/// `{"course_code":…,"name":…}` is written from Canvas's documented API and has never been run, exactly
/// as its curated row's Canvas path is. **Never panics and never guesses**: a body it does not
/// recognise is an empty list, and an empty list is what puts the typed-codes fallback on screen.
pub fn courses_from_json(body: &str) -> Vec<Course> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else { return Vec::new() };
    let items: Vec<&serde_json::Value> = match (&v, v.get("results")) {
        (serde_json::Value::Array(a), _) => a.iter().collect(),
        (_, Some(serde_json::Value::Array(a))) => a.iter().collect(),
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for it in items {
        let code = it
            .get("courseId")
            .or_else(|| it.get("course_code"))
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string();
        let name = it
            .get("course")
            .and_then(|c| c.get("name"))
            .or_else(|| it.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or_default()
            .to_string();
        if code.is_empty() && name.is_empty() {
            continue;
        }
        // The human code comes from the SUGGESTED code where the LMS's own id carries one —
        // `ua-cs-100-fall-2026` is nobody's idea of a course — and from the NAME otherwise (D4:
        // Blackboard hands back an opaque `courseId` and writes the code into the name).
        let label = crate::scaffold::suggest_course(&code)
            .or_else(|| crate::scaffold::course_code_in_name(&name));
        // The slug is that code's, and the name's when there was no code to read.
        let slug = label
            .as_deref()
            .map(knowlu_engine::ingest::slugify)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| knowlu_engine::ingest::slugify(if name.is_empty() { &code } else { &name }));
        out.push(Course { code, name, slug, label: label.unwrap_or_default() });
    }
    out
}

/// Likewise for the enrolled-course endpoint: one path per LMS kind, **this school's own host**. A
/// constant host per LMS kind would send the third curated Blackboard school's student to the first
/// one's course list.
///
/// `?expand=course` is load-bearing on Blackboard's path — without it a membership carries a
/// `courseId` and no `course` object at all, so every course would arrive nameless — recorded verbatim
/// by Task 13's second answer: 200, with `{"results":[…]}` as the body. `None` for a school nobody has
/// curated, which is what makes the panel ask instead of pretending.
fn course_list_url(unitid: &str) -> Option<String> {
    let c = crate::scaffold::curated(unitid)?;
    // <Task 13 step 4a: the PATHS that answered, verbatim>
    let path = match c.lms_kind {
        "blackboard" => "/learn/api/public/v1/users/me/courses?expand=course",
        "canvas" => "/api/v1/courses",
        _ => return None,
    };
    Some(format!("https://{}{path}", c.lms_host))
}

/// Which LMS a school runs, when nobody curated it. **Guessed from where the sign-in window landed**,
/// never from the school's name: `blackboard.com` and `instructure.com` are in the URL of every one of
/// their tenants, and a guess from a hostname is a fact. `None` means the panel's two-button question.
pub fn lms_kind_from_url(url: &str) -> Option<&'static str> {
    let u = url.to_ascii_lowercase();
    if u.contains("blackboard.com") || u.contains("/ultra/") { return Some("blackboard"); }
    if u.contains("instructure.com") || u.contains("/api/v1/courses") { return Some("canvas"); }
    None
}

/// The one answer a course capture that could not read a list gives — the **ruled outcome C, for the
/// course list only**: an empty list, and the panel showing its typed-codes field. Never an `error`,
/// because there is nothing here for a student to act on, and never anything to do with the calendar
/// link, which proves out or fails on its own.
fn no_courses() -> Value {
    json!({ "ok": true, "error": Value::Null, "courses": [], "typed": true })
}

/// The window's second job (Task 13's second go/no-go, answered **GO**), and the same cookie handover
/// the calendar capture uses: the signed-in window's own cookies, handed to one request this app
/// makes — **the student's own enrolment, out of the student's own session**.
///
/// **The window is never navigated.** The student may still be part-way through Duo when the panel
/// asks, and taking their page out from under them would lose the session the whole flow depends on.
///
/// Every failure is the same quiet answer: a campus nobody has walked, a window that is not open, a
/// non-2xx, a body this reader does not recognise, and an enrolment of none all end at
/// [`no_courses`], and the student types their course codes instead.
#[tauri::command(async)]
pub fn capture_courses(app: tauri::AppHandle, unitid: String) -> Value {
    let Some(url) = course_list_url(&unitid) else { return no_courses() };
    let Some(w) = app.get_webview_window(WINDOW) else { return no_courses() };
    // The jar rule is Task 14's, not a second one (R-C1-41, I1): the window's own URL only while it
    // is on the endpoint's host, and the endpoint's otherwise — one host's live session is never
    // handed to another. The cookies live on this function's stack: never written, never logged,
    // never formatted into an error, and gone when it returns.
    let here = w.url().ok();
    let Ok(jar_url) = cookie_url(here.as_ref().map(|u| u.as_str()), &url).parse::<tauri::Url>() else {
        return no_courses();
    };
    let Ok(cookies) = w.cookies_for_url(jar_url) else { return no_courses() };
    let jar: String = cookies.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; ");
    let agent = session_agent();
    let read = agent
        .get(&url)
        .header("cookie", &jar)
        .header("accept", "application/json")
        .call()
        .and_then(|mut r| r.body_mut().with_config().limit(1 << 22).read_to_string());
    let Ok(body) = read else { return no_courses() };
    let courses = courses_from_json(&body);
    if courses.is_empty() {
        return no_courses();
    }
    json!({ "ok": true, "error": Value::Null, "courses": courses, "typed": false })
}

/// **The account is the only writer of this URL from C3′ on** (amendment 2026-09-17, ruling 2, and
/// §9's SPII line). `scaffold::ingest_yaml` writes `ics_url: ''` into a vault that has an account;
/// a vault with no account still keeps its own copy, because it has nowhere else to keep it.
///
/// It is also the only place C1 exercises `requireActiveEntitlement` in production, which is how the
/// 402 contract C2 imports gets proved by something that ships.
///
/// **A failure here does not fail the panel, and it is not final** (Task 11 review, I1;
/// `R-C3'-exec-40`). `onboarding::create_vault_in` retries this same call once more at Finish, under
/// the session it has just moved onto the new profile — for a feed that validated and did not land
/// (its `stored` flag, [`finish_with`], R-C3'-exec-41), never for one that did or one that was
/// rejected; only if that retry also fails does the vault
/// gain a local copy of the URL (`scaffold::restore_capability_url`), exactly as it did before this
/// task, so the feed is never silently lost. The wizard is not the place to relitigate a subscription
/// either way, so a 402 becomes one sentence beside the link and everything else becomes a quieter
/// one — and both sentences now say what actually happens next rather than claiming the vault already
/// has it. The account copy is also back-filled by Task 18's `attach_in` for a vault that was
/// onboarded before it worked.
pub fn put_source_at(api_base: &str, token: &str, kind: &str, url: &str) -> Result<(), String> {
    crate::account::check_api_base(api_base)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    // Serialized compact and sent as a plain string, not through `send_json`: ureq's `json` feature
    // is not enabled in `app/Cargo.toml`, and `send_json` pretty-prints besides. `account.rs`'s
    // `post_json` documents both halves of that decision — this is the same one.
    let body = serde_json::to_string(&json!({ "kind": kind, "url": url })).map_err(|e| e.to_string())?;
    let mut res = agent
        .put(&format!("{}/account/sources", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .header("content-type", "application/json")
        .send(body)
        .map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let _ = res.body_mut().with_config().limit(1 << 16).read_to_string();
    match status {
        200..=299 => Ok(()),
        // Task 11 re-review N2 (R-C3'-exec-41): nothing moves a link the vault kept into the account
        // later, so this says what the other refusal below says rather than promise that it will.
        402 => Err("finish subscribing first — Knowlu will try again when you finish setup, and keeps it on this machine if it still can't".to_string()),
        other => Err(format!("we could not save your calendar link to your account yet ({other}) — Knowlu will try again when you finish setup, and keeps it on this machine if it still can't")),
    }
}

/// Send the validated link to the account, using whichever session this window has. The wizard has
/// only the **pending** one (the vault does not exist yet); an adopted or finished profile has its
/// own, and Task 18 passes that target instead.
pub fn store_source(api_base: &str, session_target: &str, kind: &str, url: &str) -> Result<(), String> {
    let auth = crate::account::auth_base(api_base)?;
    let token = crate::account::valid_access_token_at(&auth, &crate::account::anon_key(), session_target, jiff::Timestamp::now().as_second())?;
    put_source_at(api_base, &token, kind, url)
}

/// The whole `sources.kind` vocabulary, matching the check constraint in
/// `cloud/supabase/migrations/20260910000100_accounts.sql` and `SOURCE_KINDS` in
/// `cloud/supabase/functions/account/handler.ts`. Three copies, pinned to each other by
/// `app/tests/lms_link.rs::the_source_kind_vocabulary_is_one_list_in_three_places`.
pub const SOURCE_KINDS: [&str; 3] = ["lms_ics", "calendar_ics", "google_calendar"];

/// The two the **device** may write (spec §11a: one panel, both calendars). `google_calendar` is
/// a reserved value nobody writes (R-X-9) — a Google grant has no URL and lives in C2's
/// `google_accounts`, from a token this machine never sees — so `validate_for` refuses it here
/// rather than letting a page invent one.
pub const DEVICE_KINDS: [&str; 2] = ["lms_ics", "calendar_ics"];

/// `validate`, plus the one rule that differs between the two kinds a device may write.
///
/// **An empty school feed is a failure** — "an empty parse is a failure, never an empty semester" is
/// the rule `coursework` has always followed, and a student whose LMS feed returns nothing has a
/// problem worth hearing about. **An empty personal calendar is not**: a correct secret address with
/// nothing inside the fetched window is a student who has not put anything in their calendar yet, and
/// refusing it would be a false negative on the one field they had to go and find.
pub fn validate_for(
    kind: &str,
    url: &str,
    fetch: &dyn Fn(&str) -> Result<String, String>,
) -> Result<IcsLink, CaptureError> {
    if !DEVICE_KINDS.contains(&kind) {
        return Err(CaptureError::NotACalendarLink(format!("{kind} is not a calendar this app connects")));
    }
    match validate(url, fetch) {
        Err(CaptureError::Empty) if kind == "calendar_ics" => Ok(IcsLink { url: url.to_string(), events: 0, courses: 0 }),
        other => other,
    }
}

/// **The rewrite belongs here, not only in the paste command** (R-C1-41, I2): a capture that ever
/// returns a `webcal://` link is covered by the same line, and the link that reaches the vault and
/// `PUT /account/sources` is the `https://` one either way.
fn finish(kind: &str, url: &str) -> Value {
    finish_with(kind, url, &|u| knowlu_engine::calfeed::fetch_ics(u), &|kind, url| {
        store_source(&crate::account::api_base(), crate::account::PENDING_TARGET, kind, url)
    })
}

/// [`finish`], with the fetch and the account save handed in — the seam a test drives without a
/// network or a session.
///
/// **`stored` says whether the account now holds the link** (Task 11 re-review N1,
/// R-C3'-exec-41). The page carries it into the plan per feed, and `onboarding::create_vault_in`
/// retries at Finish only a feed that validated here and did not land: a second, needless save of a
/// link the account already holds could only fail transiently and put the link in the vault too.
/// **Only a link that passed validation is ever sent**: a rejected one returns before `store` is
/// called, here and — because its flag says so — at Finish.
pub fn finish_with(
    kind: &str,
    url: &str,
    fetch: &dyn Fn(&str) -> Result<String, String>,
    store: &dyn Fn(&str, &str) -> Result<(), String>,
) -> Value {
    let url = &https_from_webcal(url);
    match validate_for(kind, url, fetch) {
        Ok(link) => {
            // The account copy. A failure is a `note`, never an `error`: the panel goes on.
            let (stored, note) = match store(kind, &link.url) {
                Ok(()) => (true, Value::Null),
                Err(e) => (false, json!(e)),
            };
            json!({ "ok": true, "error": Value::Null, "kind": kind, "link": link, "note": note, "stored": stored })
        }
        Err(e) => json!({ "ok": false, "error": e.to_string(), "kind": kind, "link": Value::Null, "note": Value::Null, "stored": false }),
    }
}

/// **The fallback, always present** (spec §11a: "the paste-a-link path as the fallback for a campus
/// whose pages defeat it"). It is also the whole of the flow for a student whose school is not in
/// `CAMPUSES` yet — and it validates exactly as the captured path does, so the two produce the same
/// sentence and the same failure.
#[tauri::command(async)]
pub fn paste_calendar_link(kind: String, url: String) -> Value {
    finish(&kind, &url)
}
