//! The email lane's app half (spec `2026-09-30-email-forwarding-design.md`). Nothing here opens a
//! window or reaches past the loopback.
//!
//! **T9.1: the kept profiles and `mail.json`** (spec test 32's session half, §5.4, D15). One kept
//! WebView2 profile per mailbox, `<data_dir>\mail-session-<address_id>`, and one throwaway
//! `<data_dir>\mail-setup` when the saved sign-in is off; both behind `is_session_dir`, the grades
//! rule. `mail.json` sits beside `settings.json`, never in it; its `turned_off` only grows (§13 Q11).
//! Every address id here is synthetic.
//!
//! **T11.1: the service client** (plan P15, spec §6.3, test 16b's device half) runs against a loopback
//! server this file stands up on `127.0.0.1:0`; nothing leaves the machine. Every address, hint and
//! token in it is synthetic.
use knowlu::mail_session::{
    forget, forget_all, is_session_dir, session_dir, setup_dir, start_up_sweep, turn_off, BackfillPace, MailState,
};
use serde_json::json;
use std::path::{Path, PathBuf};

const A: &str = "0b0e5b2c-1111-4aaa-8bbb-000000000001";
const B: &str = "0b0e5b2c-2222-4aaa-8bbb-000000000002";

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("knowlu-mail-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A synthetic kept profile: a cookie file two folders down, as WebView2 lays one out.
fn profile(d: &Path, name: &str) {
    let net = d.join(name).join("Default").join("Network");
    std::fs::create_dir_all(&net).unwrap();
    std::fs::write(net.join("Cookies"), "synthetic").unwrap();
}

#[test]
fn is_session_dir_accepts_only_mail_session_ids_and_mail_setup_under_data_dir() {
    let d = temp("guard");
    assert_eq!(session_dir(&d, A), Some(d.join(format!("mail-session-{A}"))));
    assert_eq!(setup_dir(&d), d.join("mail-setup"));
    assert!(is_session_dir(&d, &session_dir(&d, A).unwrap()));
    assert!(is_session_dir(&d, &setup_dir(&d)));
    let long = "a".repeat(65);
    for bad in ["", "..", "../x", "a/b", "a\\b", "A-UPPER", "a.b", "a b", "x:y", long.as_str()] {
        assert_eq!(session_dir(&d, bad), None, "{bad:?} is not an address id");
    }
    let other = temp("guard-other");
    for wrong in [
        d.clone(), d.parent().unwrap().to_path_buf(), d.join("mail-session-"), d.join("mail-session"),
        d.join(format!("MAIL-SESSION-{A}")), d.join("MAIL-SETUP"), d.join("mail-setup-old"), d.join("lms-session"),
        d.join(format!("mail-session-{A}")).join("Default"), d.join("mail-setup").join("..").join("mail-setup"),
        d.join("x").join("..").join(format!("mail-session-{A}")), other.join(format!("mail-session-{A}")),
        other.join("mail-setup"),
    ] {
        assert!(!is_session_dir(&d, &wrong), "{}", wrong.display());
    }
    let rel = Path::new("relative");
    assert!(!is_session_dir(rel, &rel.join("mail-setup")), "a relative profile is refused");
    let up = d.join("..").join(d.file_name().unwrap());
    assert!(!is_session_dir(&up, &up.join("mail-setup")), "a data_dir with .. is refused");
}

#[test]
fn forget_deletes_only_that_mailboxs_profile() {
    let d = temp("forget");
    profile(&d, &format!("mail-session-{A}"));
    profile(&d, &format!("mail-session-{B}"));
    profile(&d, "mail-setup");
    profile(&d, "lms-session");
    std::fs::write(d.join("settings.json"), "{}").unwrap();
    assert_eq!(forget(&d, A), Ok(true));
    assert!(!d.join(format!("mail-session-{A}")).exists());
    for kept in [format!("mail-session-{B}"), "mail-setup".into(), "lms-session".into(), "settings.json".into()] {
        assert!(d.join(&kept).exists(), "{kept} survives forget");
    }
    assert_eq!(forget(&d, A), Ok(true), "nothing to forget is done, not a failure");
    assert!(forget(&d, "../lms-session").is_err(), "a bad id is refused");
    assert!(d.join("lms-session").exists());
    assert!(forget(Path::new("relative"), A).is_err(), "a relative profile is refused");
    assert!(MailState::load(&d).unwrap().pending_deletes.is_empty());
}

/// Holds a file inside `dir` open with no sharing, so Windows refuses to delete it.
#[cfg(windows)]
fn lock(dir: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new().read(true).share_mode(0).open(dir.join("Default").join("Network").join("Cookies")).unwrap()
}

#[cfg(windows)]
#[test]
fn a_failed_delete_is_recorded_and_retried_at_start_up() {
    let d = temp("retry");
    let name = format!("mail-session-{A}");
    profile(&d, &name);
    let held = lock(&d.join(&name));
    assert_eq!(forget(&d, A), Ok(false), "a held profile is not gone, and is recorded");
    assert_eq!(MailState::load(&d).unwrap().pending_deletes, vec![name.clone()]);
    assert_eq!(start_up_sweep(&d), Ok(vec![name.clone()]), "still held: still pending");
    assert_eq!(MailState::load(&d).unwrap().pending_deletes, vec![name.clone()]);
    drop(held);
    assert_eq!(start_up_sweep(&d), Ok(vec![]), "released: the retry deletes it");
    assert!(!d.join(&name).exists());
    assert!(MailState::load(&d).unwrap().pending_deletes.is_empty());
}

/// D15: a Settings write made from an older read (here, before sign-out's delete failed) never
/// drops the pending delete, and never switches the saved sign-in back.
#[cfg(windows)]
#[test]
fn a_settings_write_from_an_older_read_keeps_a_pending_delete() {
    let d = temp("stale");
    let name = format!("mail-session-{A}");
    profile(&d, &name);
    let older = MailState::load(&d).unwrap();
    let held = lock(&d.join(&name));
    assert_eq!(forget_all(&d), Ok(false), "a held profile is recorded");
    MailState::update(&d, |s| {
        s.keep_session = !older.keep_session;
        true
    })
    .unwrap();
    let s = MailState::load(&d).unwrap();
    assert_eq!(s.pending_deletes, vec![name.clone()], "the failed delete is still named");
    assert!(!s.keep_session);
    drop(held);
    assert_eq!(start_up_sweep(&d), Ok(vec![]));
}

/// `mail.json` has one write path, `MailState::update`, which reloads under the lock: no public
/// call can write a stale copy over `pending_deletes` or `keep_session`.
#[test]
fn mail_json_is_written_only_through_update() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("mail_session.rs")).unwrap();
    assert!(!src.contains("pub fn save"), "a public save writes whatever copy its caller holds");
}

/// D15, §5.4: an unreadable `mail.json` is a named state, never "nothing pending". A delete that
/// fails and cannot be recorded is an error, and the sweep says it cannot read the file.
#[cfg(windows)]
#[test]
fn an_unreadable_mail_json_is_named_never_read_as_clean() {
    let d = temp("corrupt");
    let name = format!("mail-session-{A}");
    profile(&d, &name);
    std::fs::write(d.join("mail.json"), "{").unwrap();
    let held = lock(&d.join(&name));
    assert!(forget(&d, A).is_err(), "held and unrecordable: an error, never a silent false");
    assert!(forget_all(&d).is_err());
    assert!(d.join(&name).exists());
    assert!(start_up_sweep(&d).is_err(), "an unreadable mail.json is never a clean sweep");
    drop(held);
    assert_eq!(forget(&d, A), Ok(true), "the delete itself stands");
    assert!(start_up_sweep(&d).is_err(), "still unreadable: still named");
    assert_eq!(std::fs::read_to_string(d.join("mail.json")).unwrap(), "{", "never overwritten");
}

#[test]
fn the_start_up_sweep_removes_a_leftover_mail_setup() {
    let d = temp("sweep");
    profile(&d, "mail-setup");
    profile(&d, &format!("mail-session-{A}"));
    profile(&d, "lms-session");
    assert_eq!(start_up_sweep(&d), Ok(vec![]));
    assert!(!d.join("mail-setup").exists());
    assert!(d.join(format!("mail-session-{A}")).exists(), "a kept profile is kept");
    assert!(d.join("lms-session").exists());
    assert_eq!(start_up_sweep(&d), Ok(vec![]), "nothing left is done");
}

#[test]
fn forget_all_removes_every_mail_profile_and_nothing_else() {
    let d = temp("forget-all");
    for name in [format!("mail-session-{A}"), format!("mail-session-{B}"), "mail-setup".into()] {
        profile(&d, &name);
    }
    let kept = ["lms-session", "mail-setup-old", "mail-sessions", "mail-session-Not.An.Id", "lu-session"];
    for name in kept {
        profile(&d, name);
    }
    for file in ["settings.json", "grades.json", "account.json"] {
        std::fs::write(d.join(file), "{}").unwrap();
    }
    turn_off(&d, A).unwrap();
    assert_eq!(forget_all(&d), Ok(true));
    for gone in [format!("mail-session-{A}"), format!("mail-session-{B}"), "mail-setup".into()] {
        assert!(!d.join(&gone).exists(), "{gone} is removed");
    }
    for name in kept.iter().copied().chain(["settings.json", "grades.json", "account.json", "mail.json"]) {
        assert!(d.join(name).exists(), "{name} survives forget_all");
    }
    assert_eq!(MailState::load(&d).unwrap().turned_off, vec![A.to_string()], "turned_off survives");
    assert!(forget_all(Path::new("relative")).is_err());
}

#[test]
fn turned_off_only_grows() {
    let d = temp("turned-off");
    turn_off(&d, A).unwrap();
    turn_off(&d, B).unwrap();
    turn_off(&d, A).unwrap();
    assert_eq!(MailState::load(&d).unwrap().turned_off, vec![A.to_string(), B.to_string()]);
    // Every write reloads under the lock, so a Settings change made from an older read drops neither.
    MailState::update(&d, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    let s = MailState::load(&d).unwrap();
    assert_eq!(s.turned_off, vec![A.to_string(), B.to_string()]);
    assert!(!s.keep_session, "the rest of the save lands");
    assert!(s.is_turned_off(A) && s.is_turned_off(B) && !s.is_turned_off("0b0e5b2c"));
    assert!(turn_off(&d, "../x").is_err(), "a bad id is refused, never written");
    // An unreadable mail.json is never read as "nothing turned off", and never overwritten.
    std::fs::write(d.join("mail.json"), "{").unwrap();
    assert!(MailState::load(&d).is_err());
    assert!(turn_off(&d, A).is_err());
    assert!(MailState::update(&d, |s| {
        *s = MailState::default();
        true
    })
    .is_err());
    assert_eq!(std::fs::read_to_string(d.join("mail.json")).unwrap(), "{");
}

#[test]
fn mail_json_is_its_own_file_and_settings_json_is_untouched() {
    let d = temp("json");
    std::fs::write(d.join("settings.json"), "SETTINGS, UNTOUCHED").unwrap();
    let fresh = MailState::load(&d).unwrap();
    assert_eq!(fresh, MailState::default(), "missing is the default");
    assert!(fresh.keep_session, "the saved sign-in is on by default (D15)");
    MailState::update(&d, |s| s.last_reconcile.insert(A.into(), "2026-10-02T12:00:00Z".into()).is_none()).unwrap();
    turn_off(&d, B).unwrap();
    let text = std::fs::read_to_string(d.join("mail.json")).unwrap();
    let want = json!({
        "keep_session": true, "last_reconcile": { A: "2026-10-02T12:00:00Z" }, "pending_deletes": [], "turned_off": [B],
    });
    assert_eq!(text, knowlu_engine::ledger::dumps_value(&want));
    assert_eq!(std::fs::read_to_string(d.join("settings.json")).unwrap(), "SETTINGS, UNTOUCHED");
    // A file written before a field existed reads that field as its default.
    std::fs::write(d.join("mail.json"), r#"{"turned_off": []}"#).unwrap();
    assert_eq!(MailState::load(&d).unwrap(), MailState::default());
    for bad in ["", "[]", "null", r#"{"keep_session": "yes"}"#, r#"{"turned_off": "x"}"#] {
        std::fs::write(d.join("mail.json"), bad).unwrap();
        assert!(MailState::load(&d).is_err(), "{bad:?}");
    }
}

// ---- T11.1: the service client over loopback ------------------------------------------------

use knowlu::mail_client::{MailClient, MailError, Phase, State};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// A token and an address no real account holds; every error string below is searched for both.
const TOKEN: &str = "synthetic-bearer-0123456789";
const HINT: &str = "student.one@school.example";
const MAILBOX: &str = "abcdefghijklmnopqrstuvwxyz@in.example";

/// What the loopback server saw of one request.
#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    bearer: Option<String>,
    body: String,
}

/// A loopback service on `127.0.0.1:0` that answers every request with `reply(&Seen)`, records it,
/// and stops when dropped. `stall` makes it read the request and then say nothing.
struct Service {
    base: String,
    seen: Arc<Mutex<Vec<Seen>>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Service {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Service {
    fn start(stall: bool, reply: impl Fn(&Seen) -> (u16, String) + Send + 'static) -> Service {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        listener.set_nonblocking(true).expect("nonblocking");
        let base = format!("http://127.0.0.1:{}/functions/v1", listener.local_addr().unwrap().port());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (seen2, stop2) = (seen.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            while !stop2.load(std::sync::atomic::Ordering::SeqCst) {
                let mut stream = match listener.accept() {
                    Ok((s, _)) => s,
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let Some(req) = read_request(&mut stream) else { continue };
                seen2.lock().unwrap().push(req.clone());
                if stall {
                    let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
                    while !stop2.load(std::sync::atomic::Ordering::SeqCst) && std::time::Instant::now() < until {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    continue;
                }
                let (status, body) = reply(&req);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Service { base, seen, stop, thread: Some(thread) }
    }

    fn requests(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> Option<Seen> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024];
    let head_end = loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let want: usize = head
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1)?.trim().parse().ok())
        .unwrap_or(0);
    while buf.len() < head_end + want {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let mut first = head.lines().next()?.split(' ');
    let (method, path) = (first.next()?.to_string(), first.next()?.to_string());
    let bearer = head
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("authorization:"))
        .map(|l| l.split_once(':').unwrap().1.trim().to_string());
    Some(Seen { method, path, bearer, body: String::from_utf8_lossy(&buf[head_end..]).to_string() })
}

fn client(svc: &Service) -> MailClient {
    MailClient::with_token_fn(&svc.base, Box::new(|| Ok(TOKEN.to_string()))).expect("a loopback base")
}

#[test]
fn every_route_sends_the_session_bearer_and_maps_failures_to_named_outcomes() {
    let svc = Service::start(false, |_| (200, r#"{"ok":true,"addresses":[]}"#.to_string()));
    let c = client(&svc);
    let id = A;
    c.list_addresses().expect("list");
    c.mint_address("google").expect("mint");
    c.set_hint(id, HINT).expect("hint");
    c.rotate_address(id).expect("rotate");
    c.delete_address(id).expect("delete");
    c.canary(id).expect("canary");
    c.open_batch(id, Phase::Backfill, "2026-06-04", 12).expect("backfill batch");
    c.open_batch(id, Phase::Recover, "2026-10-01T09:00:00Z", 3).expect("recover batch");
    c.batch_sent(B, 7).expect("sent");
    c.reconcile(id, "2026-09-29T00:00:00Z", "2026-10-02T00:00:00Z").expect("reconcile");
    c.set_state(id, State::Reenabled).expect("state");
    c.provider("school.example").expect("provider");
    let seen = svc.requests();
    let m = "/functions/v1/mail";
    let want: Vec<(&str, String, Option<serde_json::Value>)> = vec![
        ("GET", format!("{m}/addresses"), None),
        ("POST", format!("{m}/addresses"), Some(json!({"provider": "google"}))),
        ("POST", format!("{m}/addresses/{id}/hint"), Some(json!({"hint": HINT}))),
        ("POST", format!("{m}/addresses/{id}/rotate"), None),
        ("DELETE", format!("{m}/addresses/{id}"), None),
        ("POST", format!("{m}/canary"), Some(json!({"address_id": id}))),
        (
            "POST",
            format!("{m}/batches"),
            Some(json!({"address_id": id, "phase": "backfill", "since": "2026-06-04", "expected": 12})),
        ),
        (
            "POST",
            format!("{m}/batches"),
            Some(json!({"address_id": id, "phase": "recover", "since": "2026-10-01T09:00:00Z", "expected": 3})),
        ),
        ("POST", format!("{m}/batches/{B}/sent"), Some(json!({"sent": 7}))),
        (
            "POST",
            format!("{m}/reconcile"),
            Some(json!({"address_id": id, "since": "2026-09-29T00:00:00Z", "until": "2026-10-02T00:00:00Z"})),
        ),
        ("POST", format!("{m}/addresses/{id}/state"), Some(json!({"state": "reenabled"}))),
        ("GET", format!("{m}/provider?domain=school.example"), None),
    ];
    assert_eq!(seen.len(), want.len());
    for (got, (method, path, body)) in seen.iter().zip(want) {
        assert_eq!((got.method.as_str(), &got.path), (method, &path));
        assert_eq!(got.bearer.as_deref(), Some(format!("Bearer {TOKEN}").as_str()), "{path}");
        match body {
            // The body is exactly these keys: a reconcile or a batch carries nothing else (§4.11 step 3).
            Some(b) => assert_eq!(serde_json::from_str::<serde_json::Value>(&got.body).unwrap(), b, "{path}"),
            None => assert!(got.body.is_empty(), "{path}: {}", got.body),
        }
    }
    for (s, state) in [("off", State::Off), ("elsewhere", State::Elsewhere), ("confirming", State::Confirming)] {
        c.set_state(id, state).unwrap();
        let last = svc.requests().pop().unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&last.body).unwrap(), json!({"state": s}));
    }
    // The failures, one status each, by name.
    let named: [(u16, &str); 8] = [
        (400, "refused"),
        (401, "sign-in"),
        (402, "lapsed"),
        (404, "not-found"),
        (409, "refused"),
        (429, "refused"),
        (503, "server"),
        (200, "bad-reply"),
    ];
    for (status, name) in named {
        let body = if status == 200 { "not json".to_string() } else { r#"{"error":"a fixed sentence"}"#.to_string() };
        let svc = Service::start(false, move |_| (status, body.clone()));
        let e = client(&svc).rotate_address(A).expect_err("a failure");
        assert_eq!(e.name(), name, "{status}: {e}");
    }
    // A refusal carries the service's sentence (the caps are shown as they are), never more.
    let cap = "this account has made six mail addresses in the last 30 days";
    let svc = Service::start(false, move |_| (429, format!(r#"{{"error":"{cap}"}}"#)));
    match client(&svc).mint_address("google").expect_err("429") {
        MailError::Refused { status, reason } => assert_eq!((status, reason.as_str()), (429, cap)),
        e => panic!("{e:?}"),
    }
    // A port nobody listens on is unreachable, by name.
    assert_eq!(client_on_a_closed_port().list_addresses().expect_err("closed port").name(), "unreachable");
}

fn client_on_a_closed_port() -> MailClient {
    let dead = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://127.0.0.1:{}/functions/v1", dead.local_addr().unwrap().port());
    drop(dead);
    MailClient::with_token_fn(&base, Box::new(|| Ok(TOKEN.into()))).unwrap()
}

#[test]
fn a_402_on_an_entitled_route_reads_lapsed_while_get_and_delete_still_work() {
    // 16b, the device half: the service answers 402 to every route but the list and the delete.
    let svc = Service::start(false, |r| {
        let ungated = (r.method == "GET" && r.path.ends_with("/mail/addresses")) || r.method == "DELETE";
        if ungated {
            (200, r#"{"addresses":[{"address_id":"x"}],"deleted":true}"#.to_string())
        } else {
            (402, r#"{"error":"this account has no active subscription"}"#.to_string())
        }
    });
    let c = client(&svc);
    assert_eq!(c.list_addresses().expect("a canceled account sees its mailboxes").len(), 1);
    c.delete_address(A).expect("and can disconnect one");
    let lapsed = [
        c.mint_address("m365").map(drop),
        c.set_hint(A, HINT).map(drop),
        c.rotate_address(A).map(drop),
        c.canary(A).map(drop),
        c.open_batch(A, Phase::Backfill, "2026-06-04", 0).map(drop),
        c.batch_sent(B, 0).map(drop),
        c.reconcile(A, "2026-09-29T00:00:00Z", "2026-10-02T00:00:00Z").map(drop),
        c.set_state(A, State::Off).map(drop),
        c.provider("school.example").map(drop),
    ];
    for r in lapsed {
        assert!(matches!(r, Err(MailError::Lapsed)), "{r:?}");
    }
    // On the two routes that never need an entitlement a 402 is not "lapsed": it is a refusal.
    let svc = Service::start(false, |_| (402, r#"{"error":"odd"}"#.to_string()));
    let c = client(&svc);
    assert_eq!(c.list_addresses().expect_err("402").name(), "refused");
    assert_eq!(c.delete_address(A).expect_err("402").name(), "refused");
}

#[test]
fn no_token_or_address_reaches_an_error_string() {
    // A service that echoes the bearer, the mailbox address and the hint back in its error.
    let svc = Service::start(false, |r| {
        let bearer = r.bearer.clone().unwrap_or_default();
        (409, format!(r#"{{"error":"bad {bearer} for {MAILBOX} and {HINT}"}}"#))
    });
    let c = client(&svc);
    let mut errs = vec![c.set_hint(A, HINT).expect_err("409"), c.mint_address("google").expect_err("409")];
    // A sentence that names an address but not the bearer is dropped too: the `@` alone is enough.
    let svc2 = Service::start(false, |_| (409, format!(r#"{{"error":"{HINT} is held by {MAILBOX}"}}"#)));
    errs.push(client(&svc2).set_hint(A, HINT).expect_err("409"));
    assert_eq!(errs.last().unwrap().to_string(), "the account service refused the request (409)");
    // A transport failure: the URL, the token and the address stay out of it.
    errs.push(client_on_a_closed_port().set_hint(A, HINT).expect_err("closed port"));
    let no_session = MailClient::with_token_fn(&svc.base, Box::new(|| Err(format!("the store said {TOKEN}"))));
    errs.push(no_session.unwrap().list_addresses().expect_err("no session"));
    for e in &errs {
        for text in [e.to_string(), format!("{e:?}")] {
            for secret in [TOKEN, MAILBOX, HINT, "student.one", "school.example", "127.0.0.1"] {
                assert!(!text.contains(secret), "{secret} reached: {text}");
            }
            assert!(!text.contains('@'), "{text}");
        }
    }
    // The client's own Debug redacts what it holds.
    let shown = format!("{c:?}");
    assert!(!shown.contains(TOKEN) && !shown.contains("127.0.0.1"), "{shown}");
    // A request that could not be a route (a path injected through an id) is refused before it is sent.
    let before = svc.requests().len();
    assert_eq!(c.rotate_address("../../addresses").expect_err("bad id").name(), "invalid");
    assert_eq!(c.provider("school.example/../x").expect_err("bad domain").name(), "invalid");
    assert_eq!(svc.requests().len(), before, "nothing was sent");
}

/// T11.1 review: a sentence with no `@` that still carries the bearer, and one too long to be a fixed
/// sentence, are both dropped to an empty reason (the Display then names only the status).
#[test]
fn a_refusal_reason_is_dropped_when_it_holds_the_bearer_or_is_too_long() {
    let svc = Service::start(false, |r| {
        let bearer = r.bearer.clone().unwrap_or_default();
        (409, format!(r#"{{"error":"bad {bearer}"}}"#))
    });
    match client(&svc).set_hint(A, HINT).expect_err("409") {
        MailError::Refused { status, reason } => assert_eq!((status, reason.as_str()), (409, ""), "the bearer, no @"),
        e => panic!("{e:?}"),
    }
    let long = "a fixed sentence ".repeat(13);
    assert!(long.len() > 200 && !long.contains('@'));
    let svc = Service::start(false, move |_| (409, format!(r#"{{"error":"{long}"}}"#)));
    let e = client(&svc).set_hint(A, HINT).expect_err("409");
    match &e {
        MailError::Refused { status, reason } => assert_eq!((*status, reason.as_str()), (409, ""), "201+ chars, no @"),
        e => panic!("{e:?}"),
    }
    assert_eq!(e.to_string(), "the account service refused the request (409)");
    // Exactly 200 characters is still kept.
    let edge = "x".repeat(200);
    let kept = edge.clone();
    let svc = Service::start(false, move |_| (409, format!(r#"{{"error":"{edge}"}}"#)));
    match client(&svc).set_hint(A, HINT).expect_err("409") {
        MailError::Refused { reason, .. } => assert_eq!(reason, kept),
        e => panic!("{e:?}"),
    }
}

/// T11.1 review: the session is read on every request, never kept: each call asks the token function
/// again, so a token that changes between calls is the one the service sees.
#[test]
fn the_token_function_is_called_on_every_request() {
    let svc = Service::start(false, |_| (200, r#"{"addresses":[]}"#.to_string()));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = calls.clone();
    let c = MailClient::with_token_fn(
        &svc.base,
        Box::new(move || Ok(format!("token-{}", counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)))),
    )
    .expect("a loopback base");
    for _ in 0..3 {
        c.list_addresses().expect("list");
    }
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3, "once per request");
    let bearers: Vec<_> = svc.requests().into_iter().map(|r| r.bearer.unwrap_or_default()).collect();
    assert_eq!(bearers, ["Bearer token-0", "Bearer token-1", "Bearer token-2"]);
    // A request refused before it is built (a bad id) never asks for the session.
    assert_eq!(c.rotate_address("../x").expect_err("bad id").name(), "invalid");
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
fn a_timeout_is_a_named_outcome() {
    let svc = Service::start(true, |_| (200, "{}".into()));
    let c = client(&svc).with_timeout(std::time::Duration::from_millis(300));
    let started = std::time::Instant::now();
    let e = c.list_addresses().expect_err("the service never answers");
    assert_eq!(e.name(), "timed-out", "{e}");
    assert!(started.elapsed() < std::time::Duration::from_secs(3), "the timeout is the client's own");
    assert!(!e.to_string().contains("127.0.0.1"));
}

#[test]
fn no_session_is_a_named_outcome_and_nothing_is_sent() {
    let svc = Service::start(false, |_| (200, "{}".into()));
    let c = MailClient::with_token_fn(&svc.base, Box::new(|| Err("nothing stored".into()))).unwrap();
    assert_eq!(c.list_addresses().expect_err("no session").name(), "no-session");
    assert!(svc.requests().is_empty());
    // An api_base that is not https:// (or the test loopback) is refused at construction.
    assert!(MailClient::with_token_fn("http://example.test/functions/v1", Box::new(|| Ok("t".into()))).is_err());
}

// ---- T11.2: reconcile and recover -------------------------------------------------------------
//
// The routine runs against a fake mailbox (the compiled-in scripts' JSON, answered from a model of the
// mailbox), the real client over the loopback service above, and a fake clock that only advances when
// the routine waits. Every sender, subject and address is synthetic.

use knowlu::mail::recover::{
    backfill_window, fingerprint, run_pass, window, Clock, Held, Job, Mailbox, Place, Stop, Window, SLOT_CAP,
};
use knowlu::mail::Refusal;
use knowlu::mail_steps::{Paused, Provider, Routine};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const DAY: i64 = 86_400;
const NOW: i64 = 1_790_000_000;
const FPKEY: &[u8] = b"0123456789abcdef0123456789abcdef";
const FPKEY_B64: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";
const KNOWLU: &str = "box-1@in.example.invalid";
const KNOWLU_NEW: &str = "box-2@in.example.invalid";
const NONCE: &str = "bk7f3a9c2e41";
const ANA: &str = "ana.lopez@school.example.invalid";
const BO: &str = "bo.chen@school.example.invalid";
/// The mailbox's own address: the page flags what it sent.
const ME: &str = "me.student@school.example.invalid";

fn iso(secs: i64) -> String {
    jiff::Timestamp::from_second(secs).unwrap().to_string()
}

/// A clock that moves only when the routine waits.
struct Clk(Cell<i64>);

impl Clock for Clk {
    fn now(&self) -> i64 {
        self.0.get()
    }
    fn sleep(&self, secs: i64) {
        self.0.set(self.0.get() + secs);
    }
}

#[derive(Clone)]
struct Mail {
    sender: String,
    subject: String,
    at: i64,
    /// Gmail: the count the row shows, and the conversation's messages (sender, instant), oldest first.
    count: Option<u32>,
    convo: Vec<(String, i64)>,
}

fn mail(sender: &str, subject: &str, at: i64) -> Mail {
    Mail { sender: sender.into(), subject: subject.into(), at, count: None, convo: vec![] }
}

fn received(at: i64) -> String {
    format!("at-{at}")
}

/// A model of one mailbox that answers the scripts the routine runs, and records what it was asked.
struct Fake {
    provider: Provider,
    msgs: Vec<Mail>,
    /// Microsoft 365's virtualised list draws this many rows; Gmail's pager shows `size` to a page.
    drawn: usize,
    size: usize,
    clk: Rc<Clk>,
    log: RefCell<Vec<(String, Value)>>,
    /// Each forward: the clock when it ran, and its args.
    forwards: RefCell<Vec<(i64, Value)>>,
    current: RefCell<Vec<usize>>,
    page: Cell<usize>,
    places: RefCell<Vec<Place>>,
    /// The nth forward (from 0) is refused by the page.
    refuse_on: Cell<Option<usize>>,
    /// "Forward as attachment" attaches fewer than a conversation's messages.
    mismatch: bool,
    /// A Gmail send of more rows than this is "over 25 MB".
    over_rows: Option<usize>,
    cleanup_fails: bool,
    /// Sent copies the cleanup has deleted so far: each call deletes at most its `max`.
    cleaned: Cell<usize>,
}

fn fake(provider: Provider, msgs: Vec<Mail>) -> Fake {
    Fake {
        provider,
        msgs,
        drawn: 1000,
        size: 50,
        clk: Rc::new(Clk(Cell::new(NOW))),
        log: Default::default(),
        forwards: Default::default(),
        current: Default::default(),
        page: Cell::new(0),
        places: Default::default(),
        refuse_on: Cell::new(None),
        mismatch: false,
        over_rows: None,
        cleanup_fails: false,
        cleaned: Cell::new(0),
    }
}

impl Fake {
    fn row(&self, i: usize, with_thread: bool) -> Value {
        let m = &self.msgs[i];
        let mut v = json!({ "sender": m.sender, "subject": m.subject, "received": received(m.at), "received_at": iso(m.at) });
        if with_thread {
            let thread = m.count.is_some_and(|c| c > 1) || !m.convo.is_empty();
            v["thread"] = json!(thread);
            if thread {
                v["messages"] = json!(m.count);
            }
        }
        v
    }

    fn find(&self, sender: &str, subject: &str, received_text: &str) -> usize {
        (0..self.msgs.len())
            .find(|&i| {
                let m = &self.msgs[i];
                m.sender == sender && m.subject == subject && received(m.at) == received_text
            })
            .expect("the routine asked for a row the page does not show")
    }

    fn calls(&self, name: &str) -> Vec<Value> {
        self.log.borrow().iter().filter(|(p, _)| p.ends_with(name)).map(|(_, a)| a.clone()).collect()
    }

    /// The received texts of the forwards, in the order they ran (Microsoft 365: one row each).
    fn forwarded(&self) -> Vec<String> {
        self.forwards.borrow().iter().flat_map(|(_, a)| row_texts(a)).collect()
    }
}

fn row_texts(args: &Value) -> Vec<String> {
    match args.get("rows").and_then(Value::as_array) {
        Some(rows) => rows.iter().map(|r| r["received"].as_str().unwrap().to_string()).collect(),
        None => vec![args["received"].as_str().unwrap().to_string()],
    }
}

impl Mailbox for Fake {
    fn open(&self, place: Place) -> Result<(), Refusal> {
        self.places.borrow_mut().push(place);
        self.log.borrow_mut().push((format!("open {place:?}"), Value::Null));
        Ok(())
    }

    fn script(&self, path: &str, args: &Value) -> Result<Value, Refusal> {
        self.log.borrow_mut().push((path.to_string(), args.clone()));
        let (dir, name) = path.split_once('/').unwrap();
        assert_eq!(dir, if self.provider == Provider::Gmail { "gmail" } else { "m365" }, "{path}");
        let m365 = self.provider == Provider::M365;
        Ok(match name {
            "search.js" => {
                let since = args["since"].as_str().unwrap();
                let before = args.get("until").and_then(Value::as_str);
                assert!(m365 || before.is_none(), "Gmail's search takes no until");
                let mut idx: Vec<usize> = (0..self.msgs.len())
                    .filter(|&i| {
                        let day = iso(self.msgs[i].at)[..10].to_string();
                        day.as_str() >= since && before.is_none_or(|u| day.as_str() < u)
                    })
                    .collect();
                idx.sort_by_key(|&i| self.msgs[i].at);
                if m365 {
                    idx.reverse();
                }
                self.page.set(0);
                let n = idx.len();
                *self.current.borrow_mut() = idx;
                json!({ "result": "ok", "count": n })
            }
            "list.js" if m365 => {
                let cur = self.current.borrow();
                let rows: Vec<Value> = cur.iter().take(self.drawn).map(|&i| self.row(i, false)).collect();
                json!({ "result": "ok", "total": cur.len(), "rows": rows })
            }
            "list.js" => {
                let cur = self.current.borrow();
                let pages = cur.len().div_ceil(self.size).max(1);
                let before = self.page.get();
                match args["go"].as_str() {
                    Some("oldest") => self.page.set(0),
                    Some("newer") if before + 1 < pages => self.page.set(before + 1),
                    _ => {}
                }
                let at = self.page.get();
                let (start, end) = (at * self.size, ((at + 1) * self.size).min(cur.len()));
                let rows: Vec<Value> = cur[start..end].iter().map(|&i| self.row(i, true)).collect();
                let page = json!({ "from": cur.len() - end + 1, "to": cur.len() - start, "total": cur.len() });
                json!({ "result": "ok", "page": page, "moved": at != before, "rows": rows })
            }
            "thread.js" => {
                let i = self.find(args["sender"].as_str().unwrap(), args["subject"].as_str().unwrap(), args["received"].as_str().unwrap());
                let m = &self.msgs[i];
                let messages: Vec<Value> = m
                    .convo
                    .iter()
                    .map(|(who, at)| {
                        let mut v = json!({ "sender": who, "subject": m.subject, "received": received(*at), "received_at": iso(*at) });
                        if who == ME {
                            v["own"] = json!(true);
                        }
                        v
                    })
                    .collect();
                json!({ "result": "ok", "messages": messages })
            }
            "forward.js" => {
                assert_eq!(args["address"], KNOWLU);
                let n = self.forwards.borrow().len();
                let rows: Vec<Value> = args.get("rows").and_then(Value::as_array).cloned().unwrap_or_else(|| vec![args.clone()]);
                for r in &rows {
                    self.find(r["sender"].as_str().unwrap(), r["subject"].as_str().unwrap(), r["received"].as_str().unwrap());
                }
                self.forwards.borrow_mut().push((self.clk.now(), args.clone()));
                if self.refuse_on.get() == Some(n) {
                    return Ok(json!({ "result": "refused", "reason": "send failure" }));
                }
                if m365 {
                    return Ok(json!({ "result": "ok", "sent": true }));
                }
                let want: u64 = rows.iter().map(|r| r["messages"].as_u64().unwrap_or(1)).sum();
                if self.over_rows.is_some_and(|k| rows.len() > k) {
                    return Ok(json!({ "result": "failed", "reason": "over 25 MB" }));
                }
                if self.mismatch && want > rows.len() as u64 {
                    return Ok(json!({ "result": "failed", "reason": "attachments do not match the rows" }));
                }
                json!({ "result": "ok", "sent": true, "count": want })
            }
            "sent_cleanup.js" => {
                assert_eq!(self.places.borrow().last(), Some(&Place::Sent), "Sent is open for the cleanup");
                if self.cleanup_fails {
                    return Err(Refusal::TimedOut);
                }
                let max = args["max"].as_u64().expect("the cleanup names its most") as usize;
                let left = self.forwards.borrow().len() - self.cleaned.get();
                let deleted = left.min(max);
                self.cleaned.set(self.cleaned.get() + deleted);
                json!({ "result": "ok", "deleted": deleted })
            }
            other => panic!("the routine ran {other}"),
        })
    }
}

/// A loopback service that holds `held` (sender, subject, instant) for the account, opens batches and
/// takes their counts.
fn holding(held: &[(&str, &str, i64)]) -> Service {
    let entries: Vec<Value> =
        held.iter().map(|(s, t, at)| json!({ "fp": fingerprint(FPKEY, s, t), "minute": iso(*at) })).collect();
    Service::start(false, move |r| {
        if r.path.ends_with("/mail/reconcile") {
            (200, json!({ "address_id": A, "key": FPKEY_B64, "held": entries }).to_string())
        } else if r.path.ends_with("/mail/batches") {
            (201, json!({ "batch_id": "batch-1", "nonce": NONCE, "phase": "recover" }).to_string())
        } else if r.path.ends_with("/mail/batches/batch-1/sent") {
            (200, r#"{"ok":true}"#.to_string())
        } else {
            (404, r#"{"error":"no such route"}"#.to_string())
        }
    })
}

fn routine(provider: Provider) -> Routine {
    Routine::new(provider, NOW.div_euclid(DAY), 0)
}

fn job(provider: Provider, phase: Phase, window: Window, cap: Option<u32>) -> Job<'static> {
    Job { provider, address_id: A, knowlu_address: KNOWLU, phase, window, cap, watch: None }
}

fn pass(fake: &Fake, svc: &Service, routine: &mut Routine, job: &Job) -> knowlu::mail::recover::Outcome {
    run_pass(fake, &client(svc), &*fake.clk, routine, job)
}

/// `n` unheld messages in the saved session's window, one every ten minutes, in a scrambled order.
fn unheld(n: usize) -> Vec<Mail> {
    let since = window(NOW, None, None).since;
    let mut v: Vec<Mail> = (0..n).map(|i| mail(ANA, &format!("Notice {i}"), since + 3_600 + i as i64 * 600)).collect();
    v.reverse();
    v.rotate_left(n / 3);
    v
}

/// What the loopback saw, as (method, path suffix after /mail, body) with the server's own prefix cut.
fn seen(svc: &Service) -> Vec<(String, String, String)> {
    svc.requests()
        .into_iter()
        .map(|r| (r.method, r.path.split("/mail").nth(1).unwrap_or("").to_string(), r.body))
        .collect()
}

#[test]
fn a_listed_message_is_held_when_its_hmac_matches_within_fifteen_minutes() {
    let t = NOW - 3 * 3_600;
    let answer = |held: Value| json!({ "address_id": A, "key": FPKEY_B64, "held": held });
    let fp = fingerprint(FPKEY, ANA, "Quiz 3 moved");
    let held = Held::from_answer(&answer(json!([{ "fp": fp, "minute": iso(t) }]))).expect("a good answer");
    for (at, want) in [(t, true), (t + 899, true), (t + 900, true), (t - 900, true), (t + 901, false), (t - 901, false)] {
        assert_eq!(held.holds(ANA, "Quiz 3 moved", at), want, "received {} s from the held minute", at - t);
    }
    // The same fold the service uses: case, runs of white space (including no-break spaces and tabs).
    assert!(held.holds("Ana.Lopez@School.Example.Invalid", "  QUIZ 3 \u{a0}moved\t", t));
    assert!(!held.holds(BO, "Quiz 3 moved", t), "another sender");
    assert!(!held.holds(ANA, "Quiz 4 moved", t), "another subject");
    // The bytes are the service's: these two hex strings come from `mail-inbound/keys.ts`'s fingerprintInput
    // and WebCrypto (Deno), over the same key, for inputs that fold, escape and lower-case non-ASCII.
    assert_eq!(
        fingerprint(FPKEY, "Ana.Lopez@school.example.invalid", "  Quiz 3\u{a0} MOVED \t"),
        "707d922dc24e62a2cea61c942922c6d110e3145504d56f1d5c2a7366b8a8d6eb"
    );
    assert_eq!(
        fingerprint(FPKEY, "reg@school.example.invalid", "Re: \"Quoted\" \u{e9}\u{130} tab\there"),
        "64c61ccb2ffe7256542ad407ebadd015699c082d9db217480b792a1ab1e75442"
    );
    // A key that is not 32 bytes of base64, or no list of held entries, is no answer.
    for bad in [json!({ "key": "AAAA", "held": [] }), json!({ "key": "not base64!", "held": [] }), json!({ "key": FPKEY_B64 })] {
        assert!(Held::from_answer(&bad).is_none(), "{bad}");
    }
}

#[test]
fn nothing_about_an_unheld_message_is_sent_to_the_service() {
    let since = window(NOW, None, None).since;
    let (held_at, gone_at) = (since + 7_200, since + 9_000);
    let msgs = vec![
        mail("kept.sender@school.example.invalid", "Held subject words", held_at),
        mail("zed.unheld@school.example.invalid", "Secret syllabus change", gone_at),
    ];
    let svc = holding(&[("kept.sender@school.example.invalid", "Held subject words", held_at + 120)]);
    let mb = fake(Provider::M365, msgs);
    let out = pass(&mb, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    // Only the unheld message was forwarded, to Knowlu, through the mailbox's own script.
    assert_eq!((out.stop.clone(), out.held, out.sent), (Stop::Done, 1, 1));
    assert_eq!(mb.forwarded(), vec![received(gone_at)]);
    // The service saw a window, a batch and a count: no sender, subject or time of any message.
    let log = seen(&svc);
    assert_eq!(log.iter().map(|(m, p, _)| format!("{m} {p}")).collect::<Vec<_>>(), ["POST /reconcile", "POST /batches", "POST /batches/batch-1/sent"]);
    let everything = svc.requests().iter().map(|r| format!("{} {} {}", r.method, r.path, r.body)).collect::<String>();
    for secret in ["kept.sender", "zed.unheld", "school.example", "syllabus", "Held subject", &received(gone_at), &received(held_at)] {
        assert!(!everything.contains(secret), "{secret} reached the service");
    }
    let reconcile: Value = serde_json::from_str(&log[0].2).unwrap();
    let mut keys: Vec<&str> = reconcile.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, ["address_id", "since", "until"], "a reconcile carries the window and nothing else");
}

#[test]
fn recover_forwards_oldest_first_and_at_most_30_a_slot_when_no_backfill_is_open() {
    let msgs = unheld(40);
    let mut oldest: Vec<i64> = msgs.iter().map(|m| m.at).collect();
    oldest.sort();
    let svc = holding(&[]);
    let mb = fake(Provider::M365, msgs);
    let out = pass(&mb, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!(SLOT_CAP, 30);
    assert_eq!((out.stop.clone(), out.sent, out.expected), (Stop::SlotCap, 30, 40));
    let want: Vec<String> = oldest[..30].iter().map(|at| received(*at)).collect();
    assert_eq!(mb.forwarded(), want, "the 30 oldest, oldest first");
    let sent = seen(&svc).into_iter().find(|(_, p, _)| p.ends_with("/sent")).expect("the batch is closed");
    assert_eq!(sent.2, r#"{"sent":30}"#, "with the count sent");
    // A recover batch opens as a recover batch, on an instant.
    let open: Value = serde_json::from_str(&seen(&svc).iter().find(|(_, p, _)| p == "/batches").unwrap().2).unwrap();
    assert_eq!((open["phase"].as_str(), open["since"].as_str()), (Some("recover"), Some(iso(window(NOW, None, None).since).as_str())));
}

#[test]
fn an_open_backfill_ignores_the_slot_cap_and_keeps_microsofts_pace() {
    let since_day = NOW.div_euclid(DAY) - 6;
    let w = backfill_window(since_day, NOW);
    let msgs: Vec<Mail> = (0..35).map(|i| mail(ANA, &format!("Term mail {i}"), w.since + 5 * DAY + i * 900)).collect();
    let svc = holding(&[]);
    let mb = fake(Provider::M365, msgs);
    let out = pass(&mb, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Backfill, w, None));
    assert_eq!((out.stop.clone(), out.sent, out.handled, out.expected), (Stop::Done, 35, 35, 35), "no 30 cap");
    // Never more than ten in any minute: the 11th send waits for the 1st to age out.
    let times: Vec<i64> = mb.forwards.borrow().iter().map(|(t, _)| *t).collect();
    for i in 10..times.len() {
        assert!(times[i] - times[i - 10] >= 60, "forward {i} came {} s after forward {}", times[i] - times[i - 10], i - 10);
    }
    assert!(times.last().unwrap() - times[0] >= 180, "35 at ten a minute is at least three minutes");
    // The backfill's batch opened before the list, on a date, with the search's count.
    let open: Value = serde_json::from_str(&seen(&svc).iter().find(|(_, p, _)| p == "/batches").unwrap().2).unwrap();
    assert_eq!(
        (open["phase"].as_str(), open["since"].as_str(), open["expected"].as_u64()),
        (Some("backfill"), Some(iso(w.since)[..10].to_string().as_str()), Some(35))
    );
    // The day's 600th forward is the last: the next day starts at a named second.
    let mb = fake(Provider::M365, unheld(5));
    let mut r = Routine::new(Provider::M365, NOW.div_euclid(DAY), 599);
    let out = pass(&mb, &holding(&[]), &mut r, &job(Provider::M365, Phase::Recover, window(NOW, None, None), None));
    assert_eq!((out.stop.clone(), out.sent), (Stop::DayLimit { next_day_at: (NOW.div_euclid(DAY) + 1) * DAY }, 1));
    // The first refused send pauses the routine for the day: the pass says so, and a second pass sends nothing.
    let mb = fake(Provider::M365, unheld(5));
    mb.refuse_on.set(Some(1));
    let mut r = routine(Provider::M365);
    let svc = holding(&[]);
    let out = pass(&mb, &svc, &mut r, &job(Provider::M365, Phase::Recover, window(NOW, None, None), None));
    assert_eq!((out.stop.clone(), out.sent), (Stop::Paused(Paused::new(Provider::M365)), 1));
    let again = pass(&mb, &svc, &mut r, &job(Provider::M365, Phase::Recover, window(NOW, None, None), None));
    assert_eq!((again.stop.clone(), mb.forwards.borrow().len()), (Stop::Paused(Paused::new(Provider::M365)), 2), "no third forward");
}

#[test]
fn the_window_is_three_days_or_back_to_lapsed_since_at_most_30_or_to_when_last_known_on() {
    let lag = 15 * 60;
    assert_eq!(window(NOW, None, None), Window { since: NOW - 3 * DAY, until: NOW - lag });
    // After a lapse, back to when it began, never past 30 days; a lapse inside the three days changes nothing.
    assert_eq!(window(NOW, Some(NOW - 10 * DAY), None).since, NOW - 10 * DAY);
    assert_eq!(window(NOW, Some(NOW - 90 * DAY), None).since, NOW - 30 * DAY);
    assert_eq!(window(NOW, Some(NOW - DAY), None).since, NOW - 3 * DAY);
    // After forwarding was found off, to when it was last known on (at most as far as the backfill window).
    assert_eq!(window(NOW, None, Some(NOW - 20 * DAY)).since, NOW - 20 * DAY);
    assert_eq!(window(NOW, None, Some(NOW - 400 * DAY)).since, NOW - 120 * DAY);
    assert_eq!(window(NOW, None, Some(NOW - 2 * DAY)).since, NOW - 3 * DAY);
    // Both reach back: the earlier wins.
    assert_eq!(window(NOW, Some(NOW - 10 * DAY), Some(NOW - 20 * DAY)).since, NOW - 20 * DAY);
    // A backfill is a date to the same end.
    assert_eq!(backfill_window(20_000, NOW), Window { since: 20_000 * DAY, until: NOW - lag });
}

#[test]
fn the_batch_closes_with_the_count_sent_and_the_sent_copies_are_cleaned() {
    let w = window(NOW, None, None);
    let svc = holding(&[]);
    let mb = fake(Provider::M365, unheld(3));
    let out = pass(&mb, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, w, Some(SLOT_CAP)));
    assert_eq!((out.stop.clone(), out.sent, out.cleaned, out.closed), (Stop::Done, 3, 3, true));
    let log = seen(&svc);
    let close = log.iter().find(|(_, p, _)| p == "/batches/batch-1/sent").expect("the batch is closed");
    assert_eq!(close.2, r#"{"sent":3}"#);
    // Sent is opened after the last forward and before the cleanup, which names the batch's nonce and
    // Knowlu's address, and nothing wider.
    let names: Vec<String> = mb.log.borrow().iter().map(|(p, _)| p.clone()).collect();
    let at = |what: &str| names.iter().position(|n| n == what).unwrap_or_else(|| panic!("no {what} in {names:?}"));
    assert!(at("m365/forward.js") < at("open Sent") && at("open Sent") < at("m365/sent_cleanup.js"));
    assert_eq!(names.iter().rposition(|n| n == "m365/forward.js").unwrap() + 1, at("open Sent"));
    let cleanup = &mb.calls("sent_cleanup.js")[0];
    assert_eq!((cleanup["address"].as_str(), cleanup["nonce"].as_str()), (Some(KNOWLU), Some(NONCE)));
    assert!(cleanup["max"].as_u64().unwrap() >= 3);
    // A cleanup that fails changes nothing about what was sent or the batch's count.
    let mut broken = fake(Provider::M365, unheld(3));
    broken.cleanup_fails = true;
    let svc = holding(&[]);
    let out = pass(&broken, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, w, Some(SLOT_CAP)));
    assert_eq!((out.sent, out.cleaned, out.closed), (3, 0, true));
    // Nothing missing: a recover opens no batch and touches no Sent; a backfill's batch (opened before the
    // list) closes at zero, with no Sent visit.
    let msgs = unheld(2);
    let held: Vec<(String, String, i64)> = msgs.iter().map(|m| (m.sender.clone(), m.subject.clone(), m.at)).collect();
    let held: Vec<(&str, &str, i64)> = held.iter().map(|(a, b, c)| (a.as_str(), b.as_str(), *c)).collect();
    let svc = holding(&held);
    let quiet = fake(Provider::M365, msgs.clone());
    let out = pass(&quiet, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, w, Some(SLOT_CAP)));
    assert_eq!((out.stop.clone(), out.held, out.sent, out.closed), (Stop::Done, 2, 0, false));
    assert_eq!(seen(&svc).iter().map(|(_, p, _)| p.as_str()).collect::<Vec<_>>(), ["/reconcile"]);
    assert!(quiet.places.borrow().iter().all(|p| *p == Place::Inbox) && quiet.calls("sent_cleanup.js").is_empty());
    let svc = holding(&held);
    let quiet = fake(Provider::M365, msgs);
    let out = pass(&quiet, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Backfill, backfill_window(NOW.div_euclid(DAY) - 5, NOW), None));
    assert_eq!((out.sent, out.closed), (0, true));
    let paths: Vec<_> = seen(&svc).into_iter().map(|(_, p, b)| (p, b)).collect();
    assert_eq!(paths.last().unwrap(), &("/batches/batch-1/sent".to_string(), r#"{"sent":0}"#.to_string()));
    assert!(quiet.calls("sent_cleanup.js").is_empty(), "nothing was forwarded, so nothing to clean");
}

#[test]
fn each_sent_cleanup_call_fits_inside_one_step() {
    // Checkpoint C: a cleanup still running in the page after the runner's step gives up survives Gmail's
    // hash navigation and unticks the next script's rows. Each call is asked for no more than its worst
    // case (two waits a deleted row, one for each of the five skips it allows) can do inside the step's
    // 90 s, and the cleanup is called again while a call deleted all it was asked for.
    let w = window(NOW, None, None);
    let svc = holding(&[]);
    let mb = fake(Provider::M365, unheld(10));
    let out = pass(&mb, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, w, Some(SLOT_CAP)));
    assert_eq!((out.stop.clone(), out.sent, out.cleaned), (Stop::Done, 10, 10));
    let calls = mb.calls("sent_cleanup.js");
    assert!(calls.len() > 1, "ten copies take more than one call: {calls:?}");
    for c in &calls {
        let (max, wait) = (c["max"].as_u64().unwrap(), c["wait_ms"].as_u64().expect("the call names its wait"));
        assert!(wait * (2 * max + 5) < 90_000, "a call that can outlast its step: {c}");
        assert_eq!((c["address"].as_str(), c["nonce"].as_str()), (Some(KNOWLU), Some(NONCE)));
    }
    let asked: u64 = calls.iter().map(|c| c["max"].as_u64().unwrap()).sum();
    assert!(asked <= 10 + 5, "never more than the attempts and five: {asked}");
}

#[test]
fn a_microsoft_window_too_large_to_draw_is_narrowed_and_one_day_too_large_is_left() {
    // Four days of two messages each, and a list that draws three rows: the whole window does not draw, so
    // the routine searches [since, until) spans, halving, until each draws whole. Every message is sent, in
    // order, and each search past the first carries an until.
    let w = backfill_window(NOW.div_euclid(DAY) - 5, NOW);
    let day = |d: i64, k: i64| (NOW.div_euclid(DAY) - 4 + d) * DAY + 3_600 * (k + 1);
    let msgs: Vec<Mail> = (0..4).flat_map(|d| (0..2).map(move |k| (d, k))).map(|(d, k)| mail(ANA, &format!("Day {d} mail {k}"), day(d, k))).collect();
    let mut mb = fake(Provider::M365, msgs.clone());
    mb.drawn = 3;
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::M365), &job(Provider::M365, Phase::Backfill, w, None));
    assert_eq!((out.stop.clone(), out.sent, out.left, out.handled, out.expected), (Stop::Done, 8, 0, 8, 8));
    let mut want: Vec<i64> = msgs.iter().map(|m| m.at).collect();
    want.sort();
    assert_eq!(mb.forwarded(), want.iter().map(|at| received(*at)).collect::<Vec<_>>(), "oldest first across spans");
    let searches = mb.calls("search.js");
    assert!(searches[0].get("until").is_none() && searches.iter().skip(1).all(|s| s.get("until").is_some()), "{searches:?}");
    // A single day with more messages than the list draws: what shows is sent, the rest is left, counted.
    let one: Vec<Mail> = (0..5).map(|k| mail(ANA, &format!("Crowded {k}"), day(3, 0) + k * 60)).collect();
    let mut crowded = fake(Provider::M365, one);
    crowded.drawn = 3;
    let out = pass(&crowded, &holding(&[]), &mut routine(Provider::M365), &job(Provider::M365, Phase::Backfill, w, None));
    assert_eq!((out.stop.clone(), out.sent, out.left, out.handled, out.expected), (Stop::Done, 3, 2, 5, 5));
}

#[test]
fn a_gmail_thread_row_is_opened_and_every_message_in_it_is_fingerprinted() {
    let since = window(NOW, None, None).since;
    let t = |h: i64| since + h * 3_600;
    // Oldest first: a single missing message (m3), a conversation of three with one message missing, and a
    // conversation of two whose messages are both held (the second under its "Re: " subject).
    let single = mail(BO, "Lab report due", t(2));
    let mut one = mail(ANA, "Group project", t(10));
    one.count = Some(3);
    one.convo = vec![(ANA.into(), t(4)), (BO.into(), t(7)), (ANA.into(), t(10))];
    let mut two = mail(BO, "Seminar room", t(20));
    two.count = Some(2);
    two.convo = vec![(ANA.into(), t(15)), (BO.into(), t(20))];
    let svc = holding(&[
        (ANA, "Group project", t(4)),
        (ANA, "Group project", t(10) + 300),
        (ANA, "Seminar room", t(15)),
        (BO, "Re: Seminar room", t(20)),
    ]);
    let mb = fake(Provider::Gmail, vec![two.clone(), single.clone(), one.clone()]);
    let out = pass(&mb, &svc, &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!(out.stop, Stop::Done);
    // Only the conversation rows were opened; the single message was fingerprinted from its row.
    let opened: Vec<String> = mb.calls("thread.js").iter().map(|a| a["subject"].as_str().unwrap().to_string()).collect();
    assert_eq!(opened, ["Group project", "Seminar room"]);
    assert_eq!(mb.calls("thread.js")[0]["messages"], 3, "the row's count goes with it");
    // Oldest first: the single message alone, then the conversation with its count, never the held one.
    let sends: Vec<Value> = mb.forwards.borrow().iter().map(|(_, a)| a["rows"].clone()).collect();
    assert_eq!(sends.len(), 2);
    assert_eq!(sends[0].as_array().unwrap().len(), 1);
    assert_eq!(sends[0][0]["subject"], "Lab report due");
    assert!(sends[0][0].get("messages").is_none());
    assert_eq!((sends[1][0]["subject"].as_str(), sends[1][0]["messages"].as_u64()), (Some("Group project"), Some(3)));
    assert_eq!((out.sent, out.held, out.left, out.handled, out.expected), (4, 1, 0, 3, 3));
    // If "Forward as attachment" attaches fewer messages than a conversation has, the page refuses and the
    // conversation is left, once, never retried and never counted as sent.
    let mut thin = fake(Provider::Gmail, vec![one.clone()]);
    thin.mismatch = true;
    let out = pass(&thin, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!((out.stop.clone(), out.sent, out.left, out.handled), (Stop::Done, 0, 1, 1));
    assert_eq!(thin.forwards.borrow().len(), 1, "one attempt");
}

#[test]
fn a_conversation_of_a_held_message_and_the_students_own_reply_is_held_and_sends_nothing() {
    let since = window(NOW, None, None).since;
    let t = |h: i64| since + h * 3_600;
    // A professor writes (held), the student replies (never received by the service, so never held).
    let mut talk = mail(ME, "Office hours", t(6));
    talk.count = Some(2);
    talk.convo = vec![(ANA.into(), t(4)), (ME.into(), t(6))];
    let svc = holding(&[(ANA, "Office hours", t(4))]);
    let mb = fake(Provider::Gmail, vec![talk.clone()]);
    let w = window(NOW, None, None);
    let out = pass(&mb, &svc, &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, w, Some(SLOT_CAP)));
    assert_eq!((out.stop.clone(), out.sent, out.held, out.left, out.handled), (Stop::Done, 0, 1, 0, 1));
    assert!(mb.forwards.borrow().is_empty(), "the student's reply alone never makes a conversation missing");
    // The same conversation with the professor's message not held is forwarded whole (Gmail attaches every
    // message in it, the reply included), expecting one attachment each.
    let mb = fake(Provider::Gmail, vec![talk]);
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, w, Some(SLOT_CAP)));
    assert_eq!((out.stop.clone(), out.sent, out.held), (Stop::Done, 2, 0));
    assert_eq!(mb.forwards.borrow()[0].1["rows"][0]["messages"], 2);
}

#[test]
fn on_gmail_expected_and_the_progress_count_are_conversations() {
    let since = window(NOW, None, None).since;
    let t = |h: i64| since + h * 3_600;
    // Three conversations over two pages of two: a single missing message, a conversation of four with all
    // four held, and a conversation of four with one missing.
    let single = mail(BO, "Lab report due", t(2));
    let mut held4 = mail(ANA, "Reading list", t(9));
    held4.count = Some(4);
    held4.convo = (0..4).map(|i| (ANA.to_string(), t(6 + i))).collect();
    let mut gap4 = mail(ANA, "Group project", t(14));
    gap4.count = Some(4);
    gap4.convo = (0..4).map(|i| (ANA.to_string(), t(11 + i))).collect();
    let mut held: Vec<(&str, &str, i64)> = (0..4).map(|i| (ANA, "Reading list", t(6 + i))).collect();
    held.extend((0..3).map(|i| (ANA, "Group project", t(11 + i))));
    let svc = holding(&held);
    let mut mb = fake(Provider::Gmail, vec![gap4, single, held4]);
    mb.size = 2;
    let out = pass(&mb, &svc, &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Backfill, backfill_window(NOW.div_euclid(DAY) - 5, NOW), None));
    assert_eq!(out.stop, Stop::Done);
    // The search's count is conversations, and is the batch's expected; progress is conversations handled.
    let open: Value = serde_json::from_str(&seen(&svc).iter().find(|(_, p, _)| p == "/batches").unwrap().2).unwrap();
    assert_eq!(open["expected"], 3);
    assert_eq!((out.expected, out.handled), (3, 3));
    // Messages sent are not conversations: one single message and the conversation of four.
    assert_eq!(out.sent, 5);
    let close = seen(&svc).into_iter().find(|(_, p, _)| p.ends_with("/sent")).unwrap();
    assert_eq!(close.2, r#"{"sent":5}"#, "the batch closes with messages, not conversations");
    // Two pages: the oldest first, then the next newer; no page is read twice.
    let go: Vec<_> = mb.calls("list.js").iter().map(|a| a["go"].as_str().unwrap().to_string()).collect();
    assert_eq!(go, ["oldest", "newer"]);
}

#[test]
fn a_guided_mailbox_runs_nothing() {
    let mb = fake(Provider::Guided, vec![]);
    let svc = holding(&[]);
    let out = pass(&mb, &svc, &mut routine(Provider::Guided), &job(Provider::Guided, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!(out.stop, Stop::Guided);
    assert!(mb.log.borrow().is_empty() && svc.requests().is_empty());
}

// ---- T11.3: the setup runner ------------------------------------------------------------------------
//
// The runner drives T10.1's step machine through a fake window host (the compiled-in scripts' JSON,
// answered from a model of the mailbox), the real client over the loopback service, and a clock that
// moves only when the runner waits. Every address is synthetic. The answers a student gives arrive from
// the test thread; the runner is on its own thread, as it is in the app.

use knowlu::mail::host::Host;
use knowlu::mail::setup::{Answer, Ask, Env, Kind, Plan, Progress, Setups, Shared};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::time::Duration;

/// What the page shows as the mailbox's own address.
const HINT_PAGE: &str = "student.one@school.example.invalid";
/// What the student types when asked for the school address.
const TYPED: &str = "typed.person@school.example.invalid";

type Events = Arc<Mutex<Vec<String>>>;

struct SClk(AtomicI64);

impl SClk {
    fn at(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}

struct ClkRef(Arc<SClk>);

impl Clock for ClkRef {
    fn now(&self) -> i64 {
        self.0.at()
    }
    fn sleep(&self, secs: i64) {
        self.0 .0.fetch_add(secs, Ordering::SeqCst);
    }
}

fn today() -> jiff::civil::Date {
    jiff::Timestamp::from_second(NOW).unwrap().to_zoned(jiff::tz::TimeZone::UTC).date()
}

/// The first day of the backfill window for a setup today: a fixed 120 days (BW).
fn since_day() -> String {
    knowlu_engine::scheduling::add_days(today(), -120).to_string()
}

#[derive(Default)]
struct HState {
    opens: Vec<(PathBuf, bool)>,
    window: bool,
    student_closed: bool,
    signed_in: bool,
    title_address: Option<String>,
    popup_polls: u32,
    /// The visible window is where the student signs in: opening it signs them in.
    sign_in_on_visible: bool,
    aside: u32,
    back: u32,
    closed: u32,
    log: Vec<(String, Value)>,
    /// Scripted answers by script name; the last one repeats.
    queued: HashMap<String, VecDeque<Value>>,
    /// The mailbox: (sender, subject, instant).
    msgs: Vec<(String, String, i64)>,
    on_script: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

struct SHost {
    st: Arc<Mutex<HState>>,
    ev: Events,
    provider: Mutex<Provider>,
}

impl SHost {
    fn rows(s: &HState) -> Vec<Value> {
        s.msgs
            .iter()
            .map(|(who, subject, at)| json!({ "sender": who, "subject": subject, "received": received(*at), "received_at": iso(*at) }))
            .collect()
    }
}

impl Mailbox for SHost {
    fn open(&self, place: Place) -> Result<(), Refusal> {
        self.st.lock().unwrap().log.push((format!("open {place:?}"), Value::Null));
        self.ev.lock().unwrap().push(format!("open {place:?}"));
        Ok(())
    }

    fn script(&self, path: &str, args: &Value) -> Result<Value, Refusal> {
        let hook = {
            let mut s = self.st.lock().unwrap();
            s.log.push((path.to_string(), args.clone()));
            s.on_script.clone()
        };
        self.ev.lock().unwrap().push(format!("script {path}"));
        if let Some(h) = hook {
            h(path);
        }
        let mut s = self.st.lock().unwrap();
        if let Some(q) = s.queued.get_mut(path) {
            let v = if q.len() > 1 { q.pop_front().unwrap() } else { q.front().cloned().unwrap() };
            return Ok(v);
        }
        let ours = json!({ "result": "ok", "forwarding": "on", "to": "ours", "keep_copy": true, "changed": true });
        Ok(match path {
            "m365/signin.js" => json!({ "result": "ok", "signed_in": true, "mailbox_hint": HINT_PAGE }),
            "m365/forwarding_set.js" | "gmail/forwarding_set.js" | "m365/forwarding_read.js" | "gmail/forwarding_read.js" => ours,
            "gmail/forwarding_add.js" => json!({ "result": "ok", "stage": "asked", "changed": true }),
            "gmail/forwarding_add_done.js" => json!({ "result": "ok", "sent": true }),
            "m365/search.js" | "gmail/search.js" => json!({ "result": "ok", "count": s.msgs.len() }),
            "m365/list.js" => json!({ "result": "ok", "total": s.msgs.len(), "rows": SHost::rows(&s) }),
            "gmail/list.js" => {
                let n = s.msgs.len();
                json!({ "result": "ok", "page": { "from": 1, "to": n, "total": n }, "moved": false, "rows": SHost::rows(&s) })
            }
            "m365/forward.js" => json!({ "result": "ok", "sent": true }),
            "gmail/forward.js" => {
                let n = args["rows"].as_array().map_or(1, |r| r.len());
                json!({ "result": "ok", "sent": true, "count": n })
            }
            "m365/sent_cleanup.js" | "gmail/sent_cleanup.js" => json!({ "result": "ok", "deleted": 1 }),
            "m365/forwarding_off.js" | "gmail/forwarding_off.js" => json!({ "result": "ok", "forwarding": "off", "changed": true }),
            "gmail/forwarding_remove_address.js" => json!({ "result": "ok", "removed": true, "changed": true }),
            other => panic!("the runner ran {other}"),
        })
    }
}

impl Host for SHost {
    fn use_provider(&self, p: Provider) {
        *self.provider.lock().unwrap() = p;
    }
    fn open_window(&self, profile: &Path, visible: bool) -> Result<(), String> {
        std::fs::create_dir_all(profile.join("Default")).unwrap();
        std::fs::write(profile.join("Default").join("Cookies"), "synthetic").unwrap();
        let mut s = self.st.lock().unwrap();
        s.opens.push((profile.to_path_buf(), visible));
        s.window = true;
        if visible && s.sign_in_on_visible {
            s.signed_in = true;
        }
        self.ev.lock().unwrap().push(format!("window {}", if visible { "visible" } else { "hidden" }));
        Ok(())
    }
    fn window_open(&self) -> bool {
        let s = self.st.lock().unwrap();
        s.window && !s.student_closed
    }
    fn signed_in(&self) -> bool {
        self.st.lock().unwrap().signed_in
    }
    fn title_address(&self) -> Option<String> {
        self.st.lock().unwrap().title_address.clone()
    }
    fn popup_open(&self) -> bool {
        let mut s = self.st.lock().unwrap();
        let open = s.popup_polls > 0;
        s.popup_polls = s.popup_polls.saturating_sub(1);
        open
    }
    fn step_aside(&self) -> Result<(), String> {
        self.st.lock().unwrap().aside += 1;
        self.ev.lock().unwrap().push("aside".to_string());
        Ok(())
    }
    fn come_back(&self) -> Result<(), String> {
        self.st.lock().unwrap().back += 1;
        Ok(())
    }
    fn close_window(&self) {
        let mut s = self.st.lock().unwrap();
        s.window = false;
        s.closed += 1;
    }
}

#[derive(Default)]
struct Sv {
    /// What `GET /provider` answers for any domain.
    provider_of: String,
    /// The setup canary comes back through forwarding.
    canary_arrives: bool,
    canary_sent: bool,
    confirming: bool,
    hint: Option<String>,
    /// The `since` of each backfill batch opened.
    batches: Vec<String>,
    /// Set by a resume test: what the row says about the backfill.
    backfill_since: Option<String>,
    provider: String,
    /// `POST /addresses/<id>/rotate` answers 429 (the mint limit).
    rotate_refused: bool,
}

fn setup_service(sv: &Arc<Mutex<Sv>>, ev: &Events) -> Service {
    let (sv, ev) = (sv.clone(), ev.clone());
    Service::start(false, move |r| {
        let path = r.path.split("/mail").nth(1).unwrap_or("").to_string();
        let body: Value = serde_json::from_str(&r.body).unwrap_or(Value::Null);
        let mut s = sv.lock().unwrap();
        let note = |what: &str| ev.lock().unwrap().push(format!("svc {what}"));
        match (r.method.as_str(), path.as_str()) {
            ("POST", "/addresses") => {
                note("mint");
                s.provider = body["provider"].as_str().unwrap_or("").to_string();
                (201, json!({ "address_id": A, "address": KNOWLU, "provider": s.provider, "status": "pending" }).to_string())
            }
            ("GET", "/addresses") => {
                let ok_at = (s.canary_sent && s.canary_arrives).then(|| "2026-10-02T10:00:40Z");
                let status = if ok_at.is_some() { "active" } else if s.confirming { "confirmed" } else { "pending" };
                let row = json!({
                    "address_id": A, "provider": s.provider, "address": KNOWLU, "mailbox_hint": s.hint,
                    "status": status, "backfill_since": s.backfill_since, "last_canary_ok_at": ok_at,
                });
                (200, json!({ "addresses": [row] }).to_string())
            }
            ("POST", p) if p.ends_with("/rotate") => {
                note("rotate");
                if s.rotate_refused {
                    return (429, r#"{"error":"limit"}"#.to_string());
                }
                (200, json!({ "address_id": A, "address": KNOWLU_NEW, "provider": s.provider, "status": "pending" }).to_string())
            }
            ("POST", p) if p.ends_with("/hint") => {
                note("hint");
                s.hint = body["hint"].as_str().map(str::to_string);
                (200, json!({ "address_id": A, "mailbox_hint": s.hint }).to_string())
            }
            ("POST", p) if p.ends_with("/state") => {
                let state = body["state"].as_str().unwrap_or("");
                note(&format!("state {state}"));
                s.confirming |= state == "confirming";
                (200, json!({ "address_id": A, "state": state, "status": "pending" }).to_string())
            }
            ("POST", "/canary") => {
                note("canary");
                s.canary_sent = true;
                (201, json!({ "canary_id": "c-1", "address_id": A, "sent_at": "2026-10-02T10:00:00Z" }).to_string())
            }
            ("POST", "/batches") => {
                note("batch");
                s.batches.push(body["since"].as_str().unwrap_or("").to_string());
                (201, json!({ "batch_id": "batch-1", "nonce": NONCE, "phase": body["phase"], "since": body["since"] }).to_string())
            }
            ("POST", p) if p.starts_with("/batches/") && p.ends_with("/sent") => {
                note("batch sent");
                (200, r#"{"ok":true}"#.to_string())
            }
            ("POST", "/reconcile") => (200, json!({ "address_id": A, "key": FPKEY_B64, "held": [] }).to_string()),
            ("DELETE", p) if p.starts_with("/addresses/") => {
                note("delete address");
                (200, r#"{"deleted":true}"#.to_string())
            }
            ("GET", p) if p.starts_with("/provider") => (200, json!({ "domain": "x", "provider": s.provider_of }).to_string()),
            _ => (404, r#"{"error":"no such route"}"#.to_string()),
        }
    })
}

struct Rig {
    dir: PathBuf,
    host: Arc<Mutex<HState>>,
    ev: Events,
    sv: Arc<Mutex<Sv>>,
    svc: Service,
    clk: Arc<SClk>,
    closed: Arc<AtomicU32>,
    setups: Arc<Setups>,
}

fn rig(tag: &str) -> Rig {
    let host = Arc::new(Mutex::new(HState { signed_in: true, ..Default::default() }));
    let ev = Events::default();
    let sv = Arc::new(Mutex::new(Sv { provider_of: "m365".into(), canary_arrives: true, ..Default::default() }));
    Rig {
        dir: temp(tag),
        svc: setup_service(&sv, &ev),
        host,
        ev,
        sv,
        clk: Arc::new(SClk(AtomicI64::new(NOW))),
        closed: Default::default(),
        setups: Arc::new(Setups::new()),
    }
}

impl Rig {
    fn env(&self) -> Env {
        let (closed, ev) = (self.closed.clone(), self.ev.clone());
        Env {
            host: Box::new(SHost { st: self.host.clone(), ev: self.ev.clone(), provider: Mutex::new(Provider::Guided) }),
            client: MailClient::with_token_fn(&self.svc.base, Box::new(|| Ok(TOKEN.to_string()))).expect("a loopback base"),
            clock: Box::new(ClkRef(self.clk.clone())),
            on_closed: Box::new(move || {
                closed.fetch_add(1, Ordering::SeqCst);
                ev.lock().unwrap().push("slot now".to_string());
            }),
            guard: None,
        }
    }

    fn plan(&self, kind: Kind, keep: bool) -> Plan {
        Plan { kind, data_dir: self.dir.clone(), keep_session: keep, setup_day: today() }
    }

    fn start(&self, kind: Kind, keep: bool) -> Arc<Shared> {
        self.setups.start(self.plan(kind, keep), self.env()).expect("a setup started")
    }

    fn events(&self) -> Vec<String> {
        self.ev.lock().unwrap().clone()
    }

    fn scripts(&self) -> Vec<String> {
        self.host.lock().unwrap().log.iter().filter(|(p, _)| p.contains(".js")).map(|(p, _)| p.clone()).collect()
    }

    fn calls(&self, name: &str) -> Vec<Value> {
        self.host.lock().unwrap().log.iter().filter(|(p, _)| p == name).map(|(_, a)| a.clone()).collect()
    }
}

/// A curated school whose mail is Microsoft 365 (the table is data; nothing here names one).
fn curated_m365() -> Kind {
    let row = knowlu::scaffold::CAMPUSES.iter().find(|c| c.mail_provider == "m365").expect("a curated m365 row");
    Kind::School { unitid: Some(row.unitid.to_string()), address: None }
}

/// A school no curated row names: the setup asks once for its address.
fn uncurated() -> Kind {
    Kind::School { unitid: Some("0".to_string()), address: None }
}

fn wait(shared: &Shared) -> Progress {
    assert!(shared.wait_done(Duration::from_secs(30)), "the setup did not end");
    shared.progress()
}

fn until(what: &str, f: impl Fn() -> bool) {
    let end = std::time::Instant::now() + Duration::from_secs(10);
    while !f() {
        assert!(std::time::Instant::now() < end, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn row<'a>(p: &'a Progress, step: &str) -> &'a knowlu::mail::setup::Row {
    p.rows.iter().find(|r| r.step == step).unwrap_or_else(|| panic!("no row {step}"))
}

/// Whether a step's row reads `state` now (false while the rows are not yet published).
fn has(shared: &Shared, step: &str, state: &str) -> bool {
    shared.progress().rows.iter().any(|r| r.step == step && r.state == state)
}

fn ana_mail(n: usize) -> Vec<(String, String, i64)> {
    (0..n).map(|i| (ANA.to_string(), format!("Notice {i}"), NOW - 5 * DAY + i as i64 * 600)).collect()
}

#[test]
fn one_setup_at_a_time() {
    let r = rig("t113-one");
    let first = r.start(uncurated(), true);
    until("the first setup's question", || first.progress().ask == Some("school_address"));
    assert!(r.setups.start(r.plan(uncurated(), true), r.env()).is_err(), "a second setup while the first runs");
    assert!(r.setups.current().is_some_and(|c| Arc::ptr_eq(&c, &first)));
    first.stop();
    assert_eq!(wait(&first).ended, Some("stopped"));
    let again = r.setups.start(r.plan(uncurated(), true), r.env()).expect("a new one once the first has ended");
    again.stop();
    wait(&again);
    assert!(!r.events().contains(&"svc mint".to_string()), "it was asked, never minted");
}

#[test]
fn progress_carries_the_mailbox_id_and_never_the_address() {
    let r = rig("t151-addr-id");
    let shared = r.start(curated_m365(), true);
    until("the mailbox id", || shared.progress().address_id.is_some());
    shared.stop();
    let p = wait(&shared);
    assert_eq!(p.address_id.as_deref(), Some(A));
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains(KNOWLU) && !json.contains("in.example"), "no address in the progress");
}

#[test]
fn a_closed_sign_in_window_is_setup_cancelled_and_nothing_changed() {
    let r = rig("t113-cancel");
    {
        let mut h = r.host.lock().unwrap();
        h.signed_in = false;
        h.student_closed = true;
    }
    let p = wait(&r.start(Kind::Gmail, true));
    assert_eq!(p.ended, Some("cancelled"));
    assert_eq!(p.line, Some("setup cancelled"));
    assert_eq!(row(&p, "sign_in").state, "skipped");
    let ev = r.events();
    for never in ["svc hint", "svc canary", "svc batch", "svc state confirming"] {
        assert!(!ev.contains(&never.to_string()), "{never}");
    }
    assert!(r.scripts().is_empty(), "no script ran: {:?}", r.scripts());
    assert!(ev.contains(&"svc delete address".to_string()), "the address minted for it is revoked");
    assert!(!session_dir(&r.dir, A).unwrap().exists(), "its profile is gone");
    assert!(MailState::load(&r.dir).unwrap().open_backfills.is_empty());
}

#[test]
fn stop_ends_after_the_current_step_and_names_what_was_done() {
    let r = rig("t113-stop");
    let setups = r.setups.clone();
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "m365/forwarding_set.js" {
            setups.current().expect("a setup is running").stop();
        }
    }));
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!(p.ended, Some("stopped"));
    assert_eq!(p.line, Some("setup stopped"));
    assert_eq!(p.done, ["sign_in", "step_aside", "forwarding"], "the step in hand finished");
    assert_eq!(p.not_brought_in_before, Some(since_day()), "what the stop says was not brought in");
    for later in ["canary", "backfill", "finish"] {
        assert_eq!(row(&p, later).state, "skipped", "{later}");
    }
    let ev = r.events();
    assert!(!ev.contains(&"svc canary".to_string()) && !ev.contains(&"svc batch".to_string()));
    assert!(r.host.lock().unwrap().closed >= 1, "the window is closed");
    assert!(session_dir(&r.dir, A).unwrap().exists(), "nothing the student kept is undone");
}

#[test]
fn a_canary_not_back_in_three_minutes_leaves_blocked_with_retry_and_offers_what_remains() {
    let r = rig("t113-blocked");
    r.sv.lock().unwrap().canary_arrives = false;
    r.host.lock().unwrap().msgs = ana_mail(2);
    let shared = r.start(curated_m365(), true);
    until("the blocked state", || shared.progress().blocked);
    let p = shared.progress();
    assert_eq!(row(&p, "canary").state, "failed");
    assert_eq!(row(&p, "backfill").state, "failed");
    assert_eq!(row(&p, "backfill").reason, Some("blocked"));
    assert!(row(&p, "backfill").guided);
    for offer in ["retry", "personal_gmail", "guided_steps"] {
        assert!(p.offers.contains(&offer), "{offer}: {:?}", p.offers);
    }
    let waited = r.clk.at() - NOW;
    assert!((180..240).contains(&waited), "it waited the three minutes and no longer: {waited}");
    assert!(!r.scripts().iter().any(|s| s.ends_with("search.js")), "no backfill is read while forwarding does not arrive");
    assert!(!r.events().contains(&"svc batch".to_string()));
    // Retry: the school's setting is fixed and the canary comes back.
    r.sv.lock().unwrap().canary_arrives = true;
    assert!(shared.answer(Answer::Retry));
    until("the canary done by hand", || has(&shared, "canary", "done_by_hand"));
    // The canary arriving re-arms the backfill and it runs on this run (§4.2 step 4), not at some later launch.
    let p = wait(&shared);
    assert_eq!(p.ended, Some("finished"));
    assert_eq!(row(&p, "backfill").state, "done");
    assert!(!p.blocked && p.not_brought_in_before.is_none(), "{p:?}");
    assert_eq!(r.sv.lock().unwrap().batches, [since_day()], "the backfill ran after the Retry");
    assert_eq!(r.calls("m365/forward.js").len(), 2);
    assert!(MailState::load(&r.dir).unwrap().open_backfills.is_empty());
}

fn answer_address(shared: &Shared) {
    until("the question", || shared.progress().ask == Some("school_address"));
    // An answer to a different question is not taken.
    assert!(!shared.answer(Answer::Replace(true)));
    assert!(shared.answer(Answer::SchoolAddress(TYPED.to_string())));
}

#[test]
fn an_uncurated_school_asks_once_for_the_address_and_the_service_decides_the_provider() {
    let r = rig("t113-ask");
    let shared = r.start(uncurated(), true);
    answer_address(&shared);
    let p = wait(&shared);
    assert_eq!((p.ended, p.provider), (Some("finished"), "m365"));
    assert_eq!(shared.asked(), ["school_address"], "asked once");
    let all = seen(&r.svc);
    assert!(all.iter().any(|(m, path, _)| m == "GET" && path == "/provider?domain=school.example.invalid"), "{all:?}");
    assert!(all.iter().any(|(m, path, b)| m == "POST" && path == "/addresses" && b == r#"{"provider":"m365"}"#), "{all:?}");
    assert!(
        all.iter().any(|(_, path, b)| path.ends_with("/hint") && b.contains(HINT_PAGE)),
        "the hint is the address the page shows"
    );
    assert!(!format!("{all:?}").contains("typed.person"), "only the typed address's domain left the device");

    // A curated school is never asked and no lookup is made.
    let c = rig("t113-ask-curated");
    let p = wait(&c.start(curated_m365(), true));
    assert_eq!(p.ended, Some("finished"));
    let all = seen(&c.svc);
    assert!(!all.iter().any(|(_, path, _)| path.starts_with("/provider")), "no lookup for a curated row");
    assert!(all.iter().any(|(_, path, b)| path == "/addresses" && b == r#"{"provider":"m365"}"#));

    // An unknown provider is guided: the typed address is the mailbox's own, since no page can show it.
    let g = rig("t113-ask-other");
    g.sv.lock().unwrap().provider_of = "other".into();
    let shared = g.start(uncurated(), true);
    answer_address(&shared);
    until("the guided step", || has(&shared, "forwarding", "needs_you"));
    let p = shared.progress();
    assert_eq!(p.provider, "other");
    assert!(row(&p, "forwarding").guided);
    assert!(g.scripts().is_empty(), "a guided mailbox runs no script");
    assert!(seen(&g.svc).iter().any(|(_, path, b)| path.ends_with("/hint") && b.contains(TYPED)));
    shared.stop();
    wait(&shared);
}

#[test]
fn a_google_workspace_school_is_offered_forwarding_through_gmails_steps() {
    let r = rig("t113-workspace");
    r.sv.lock().unwrap().provider_of = "google".into();
    r.host.lock().unwrap().title_address = Some(HINT_PAGE.to_string());
    let shared = r.start(uncurated(), true);
    answer_address(&shared);
    let p = wait(&shared);
    assert_eq!((p.ended, p.provider), (Some("finished"), "google"));
    assert_eq!(row(&p, "add_address").state, "done");
    assert!(seen(&r.svc).iter().any(|(_, path, b)| path.ends_with("/hint") && b.contains(HINT_PAGE)));
}

#[test]
fn no_date_is_asked_and_every_school_reads_the_same_120_days() {
    for (tag, kind, typed) in [("t113-date-a", curated_m365(), false), ("t113-date-b", uncurated(), true)] {
        let r = rig(tag);
        r.host.lock().unwrap().msgs = ana_mail(1);
        let shared = r.start(kind, true);
        if typed {
            answer_address(&shared);
        }
        let p = wait(&shared);
        assert_eq!(p.ended, Some("finished"), "{tag}");
        assert_eq!(r.sv.lock().unwrap().batches, [since_day()], "{tag}: the batch opens on the fixed window start");
        assert_eq!(p.backfill.as_ref().map(|b| b.since.clone()), Some(since_day()), "{tag}");
        assert!(shared.asked().iter().all(|a| *a == "school_address"), "{tag}: only the address is ever asked");
    }
    assert_eq!(Ask::ALL.map(Ask::name), ["school_address", "forwarding_elsewhere"], "no ask is about a date");
}

/// Whether `text` holds `word` as a whole word (not inside a longer run of letters or digits).
fn names(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(i, w)| {
        let before = text[..i].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let after = text[i + w.len()..].chars().next().is_none_or(|c| !c.is_alphanumeric());
        before && after
    })
}

fn files_under(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            files_under(&p, exts, out);
        } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| exts.contains(&x)) {
            out.push(p);
        }
    }
}

#[test]
fn the_mail_code_names_no_school() {
    let mut words: Vec<&str> = Vec::new();
    for c in knowlu::scaffold::CAMPUSES.iter() {
        words.extend([c.unitid, c.key]);
        words.extend(c.mail_domain);
    }
    assert!(words.len() >= 4, "the curated table was read");
    let mut files: Vec<PathBuf> = Vec::new();
    for e in std::fs::read_dir("src").unwrap().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with("mail") && name.ends_with(".rs") {
            files.push(e.path());
        }
    }
    files_under(Path::new("src/mail"), &["rs"], &mut files);
    files_under(Path::new("assets/mail"), &["js", "ts", "json"], &mut files);
    assert!(files.iter().any(|f| f.ends_with("setup.rs")), "src/mail/setup.rs was read");
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        for w in &words {
            assert!(!names(&text, w), "{} names a school: {w}", f.display());
        }
    }
}

#[test]
fn a_closed_backfill_batch_starts_a_slot_now() {
    let r = rig("t113-slot");
    r.host.lock().unwrap().msgs = ana_mail(2);
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!(p.ended, Some("finished"));
    let ev = r.events();
    let at = |e: &str| ev.iter().position(|x| x == e).unwrap_or_else(|| panic!("no {e}: {ev:?}"));
    assert!(at("svc batch sent") < at("slot now"), "the slot starts once the batch is closed");
    assert_eq!(r.closed.load(Ordering::SeqCst), 1, "one closed batch starts one slot");
    // The app's own hook runs the slot through the scheduler (§4.8 step 1).
    assert!(include_str!("../src/mail/live.rs").contains("scheduler::run_slot"));
}

#[test]
fn with_the_saved_sign_in_off_setup_uses_mail_setup_and_wipes_it_on_close() {
    let off = rig("t113-keep-off");
    let p = wait(&off.start(curated_m365(), false));
    assert_eq!(p.ended, Some("finished"));
    assert_eq!(off.host.lock().unwrap().opens, [(setup_dir(&off.dir), true)]);
    assert!(!setup_dir(&off.dir).exists(), "mail-setup is wiped when the window closes");
    assert!(!session_dir(&off.dir, A).unwrap().exists());
    assert!(MailState::load(&off.dir).unwrap().pending_deletes.is_empty());

    let on = rig("t113-keep-on");
    let p = wait(&on.start(curated_m365(), true));
    assert_eq!(p.ended, Some("finished"));
    assert_eq!(on.host.lock().unwrap().opens, [(session_dir(&on.dir, A).unwrap(), true)]);
    assert!(session_dir(&on.dir, A).unwrap().exists(), "the kept profile stays");
    assert!(!setup_dir(&on.dir).exists());
}

/// A mailbox whose first backfill began and was cut off: the service holds its window start, the kept
/// profile is on disk, and `mail.json` marks it open.
fn open_backfill_rig(tag: &str, n: usize) -> Rig {
    let r = rig(tag);
    {
        let mut s = r.sv.lock().unwrap();
        s.provider = "m365".into();
        s.hint = Some(HINT_PAGE.into());
        s.backfill_since = Some("2026-06-04".into());
        s.canary_sent = true;
    }
    r.host.lock().unwrap().msgs = ana_mail(n);
    profile(&r.dir, &format!("mail-session-{A}"));
    MailState::update(&r.dir, |s| {
        s.open_backfills.push(A.to_string());
        true
    })
    .unwrap();
    r
}

#[test]
fn an_open_backfill_resumes_at_launch() {
    let r = open_backfill_rig("t113-resume", 2);
    assert_eq!(open_backfills(&r.dir), [A]);
    let shared = r.setups.start_resume(r.dir.clone(), vec![A.to_string()], r.env()).expect("a resume started");
    let p = wait(&shared);
    assert_eq!(p.ended, Some("finished"));
    assert_eq!(r.host.lock().unwrap().opens, [(session_dir(&r.dir, A).unwrap(), false)], "hidden, on the kept profile");
    assert_eq!(r.sv.lock().unwrap().batches, ["2026-06-04"], "the window the first backfill fixed, never recomputed");
    assert_eq!(r.calls("m365/forward.js").len(), 2);
    assert!(MailState::load(&r.dir).unwrap().open_backfills.is_empty(), "a finished backfill is not resumed again");
    assert_eq!(r.closed.load(Ordering::SeqCst), 1);
    assert!(r.host.lock().unwrap().closed >= 1 && session_dir(&r.dir, A).unwrap().exists());

    // A mailbox the student turned off in Knowlu's Settings is never resumed.
    let t = open_backfill_rig("t113-resume-off", 1);
    turn_off(&t.dir, A).unwrap();
    assert!(open_backfills(&t.dir).is_empty());

    // Signed out: no window is shown, nothing is read, and the marker stays for the next launch.
    let s = open_backfill_rig("t113-resume-out", 1);
    s.host.lock().unwrap().signed_in = false;
    let shared = s.setups.start_resume(s.dir.clone(), vec![A.to_string()], s.env()).unwrap();
    let p = wait(&shared);
    assert_eq!(p.line, Some("needs sign-in"));
    assert!(s.host.lock().unwrap().opens.iter().all(|(_, visible)| !visible));
    assert!(s.sv.lock().unwrap().batches.is_empty());
    assert_eq!(open_backfills(&s.dir), [A]);
}

#[test]
fn an_open_backfill_resumes_again_as_its_pace_allows() {
    let r = open_backfill_rig("t113-pace", 3);
    let env = r.env();
    let shared = Shared::new();
    // 599 sent today: one more fits, then the day's 600 are spent.
    let mut routine = Routine::new(Provider::M365, NOW.div_euclid(DAY), 599);
    let next_day = (NOW.div_euclid(DAY) + 1) * DAY;
    let d = Drive {
        mb: &*env.host,
        client: &env.client,
        clock: &*env.clock,
        provider: Provider::M365,
        address_id: A,
        knowlu_address: KNOWLU,
        since: "2026-06-04".parse().unwrap(),
        shared: &shared,
        on_closed: &*env.on_closed,
        data_dir: &r.dir,
    };
    let end = drive_backfill(&d, &mut routine);
    assert_eq!(end, Drove::Done);
    assert!(r.clk.at() >= next_day, "it waited for the next UTC day");
    assert_eq!(r.calls("m365/forward.js").len(), 1 + 3, "one before the day's limit, the window again after it");
    assert_eq!(r.sv.lock().unwrap().batches, ["2026-06-04", "2026-06-04"], "each pass is a backfill batch on the same window");
    assert_eq!(r.closed.load(Ordering::SeqCst), 2, "each closed batch starts a slot");
    assert!(MailState::load(&r.dir).unwrap().open_backfills.is_empty());
}

use knowlu::mail::backfill::{drive_backfill, open_backfills, Drive, Drove};

/// Runs the driver on `routine` with the first forward stopping it (a quit mid-day), and returns how it ended.
fn quit_after_first_forward(r: &Rig, routine: &mut Routine) -> Drove {
    let env = r.env();
    let shared = Arc::new(Shared::new());
    let stopper = shared.clone();
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "m365/forward.js" {
            stopper.stop();
        }
    }));
    let d = Drive {
        mb: &*env.host,
        client: &env.client,
        clock: &*env.clock,
        provider: Provider::M365,
        address_id: A,
        knowlu_address: KNOWLU,
        since: "2026-06-04".parse().unwrap(),
        shared: &shared,
        on_closed: &*env.on_closed,
        data_dir: &r.dir,
    };
    drive_backfill(&d, routine)
}

/// Resumes the open backfill and returns the clock at each forward it sent.
fn resume_and_time_forwards(r: &Rig) -> Vec<i64> {
    let times = Arc::new(Mutex::new(Vec::new()));
    let (t, clk) = (times.clone(), r.clk.clone());
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "m365/forward.js" {
            t.lock().unwrap().push(clk.at());
        }
    }));
    MailState::update(&r.dir, |s| {
        if !s.open_backfills.iter().any(|i| i == A) {
            s.open_backfills.push(A.to_string());
        }
        true
    })
    .unwrap();
    let shared = r.setups.start_resume(r.dir.clone(), vec![A.to_string()], r.env()).expect("a resume started");
    wait(&shared);
    let out = times.lock().unwrap().clone();
    out
}

#[test]
fn a_relaunch_on_the_same_utc_day_never_sends_past_the_days_600() {
    // Spec 4.2 step 5: at most 600 a mailbox a UTC day. 599 were sent, one more went (600), the app quit;
    // the relaunch the same UTC day sends nothing until the next day.
    let r = open_backfill_rig("t113-relaunch-day", 3);
    let next_day = (NOW.div_euclid(DAY) + 1) * DAY;
    let mut routine = Routine::new(Provider::M365, NOW.div_euclid(DAY), 599);
    assert_eq!(quit_after_first_forward(&r, &mut routine), Drove::Stopped);
    assert_eq!(r.calls("m365/forward.js").len(), 1);
    r.clk.0.fetch_add(3_600, Ordering::SeqCst);
    let times = resume_and_time_forwards(&r);
    assert_eq!(times.len(), 3, "the window is forwarded once the day turns: {times:?}");
    assert!(times.iter().all(|t| *t >= next_day), "nothing went before the next UTC day: {times:?} vs {next_day}");
}

#[test]
fn a_resume_after_a_refused_send_waits_for_the_next_utc_day() {
    // The pause after a refused send (an NDR) survives a relaunch: nothing retries until the next day.
    let r = open_backfill_rig("t113-relaunch-paused", 2);
    let day = NOW.div_euclid(DAY);
    let next_day = (day + 1) * DAY;
    MailState::update(&r.dir, |s| {
        s.backfill_pace.insert(A.to_string(), BackfillPace { day, sent_today: 41, paused_until: Some(next_day) });
        true
    })
    .unwrap();
    let times = resume_and_time_forwards(&r);
    assert_eq!(times.len(), 2);
    assert!(times.iter().all(|t| *t >= next_day), "{times:?} vs {next_day}");
    // A pace from an earlier day counts for nothing.
    let old = open_backfill_rig("t113-relaunch-old-pace", 2);
    MailState::update(&old.dir, |s| {
        s.backfill_pace.insert(A.to_string(), BackfillPace { day: day - 1, sent_today: 600, paused_until: Some(day * DAY) });
        true
    })
    .unwrap();
    assert!(resume_and_time_forwards(&old).iter().all(|t| *t < next_day), "yesterday's limit does not hold today");
}

#[test]
fn the_pace_is_kept_beside_the_open_marker_and_costs_nothing_when_empty() {
    let r = open_backfill_rig("t113-pace-file", 3);
    let mut routine = Routine::new(Provider::M365, NOW.div_euclid(DAY), 599);
    quit_after_first_forward(&r, &mut routine);
    let pace = MailState::load(&r.dir).unwrap().backfill_pace;
    assert_eq!(pace.get(A), Some(&BackfillPace { day: NOW.div_euclid(DAY), sent_today: 600, paused_until: None }));
    // With none kept, mail.json is the spec's bytes: the field is not written.
    let clean = rig("t113-pace-empty");
    MailState::update(&clean.dir, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    let text = std::fs::read_to_string(clean.dir.join("mail.json")).unwrap();
    assert!(!text.contains("backfill_pace") && !text.contains("open_backfills"), "{text}");
}

#[test]
fn a_guided_backfill_the_student_says_is_done_never_clears_what_was_not_brought_in() {
    // No count route exists for a manual forward-as-attachment, so "I've done this" cannot pass the
    // backfill: the setup ends honestly, naming the day before which mail was not brought in.
    let g = rig("t113-guided-backfill");
    g.sv.lock().unwrap().provider_of = "other".into();
    let shared = g.start(uncurated(), true);
    answer_address(&shared);
    until("the forwarding step", || has(&shared, "forwarding", "needs_you"));
    assert!(shared.answer(Answer::Done));
    until("the backfill step", || has(&shared, "backfill", "needs_you"));
    assert!(shared.answer(Answer::Done));
    let p = wait(&shared);
    assert_eq!(p.not_brought_in_before, Some(since_day()));
    assert_ne!(row(&p, "backfill").state, "done_by_hand");
    assert!(!p.done.contains(&"backfill"), "{:?}", p.done);
}

#[test]
fn retry_runs_a_failed_scripted_backfill_again_and_the_marker_stays_open_until_then() {
    let r = rig("t113-failed-backfill");
    r.host.lock().unwrap().msgs = ana_mail(2);
    queue(&r, "m365/list.js", vec![json!({ "result": "not found", "control": "list" }), json!({ "result": "ok", "total": 2, "rows": SHost::rows(&r.host.lock().unwrap()) })]);
    let shared = r.start(curated_m365(), true);
    until("the backfill failed", || has(&shared, "backfill", "failed"));
    assert_eq!(MailState::load(&r.dir).unwrap().open_backfills, [A], "still open, so the next launch resumes it");
    assert!(shared.answer(Answer::Retry));
    let p = wait(&shared);
    assert_eq!((p.ended, row(&p, "backfill").state), (Some("finished"), "done"));
    assert!(p.not_brought_in_before.is_none());
}

fn position(ev: &[String], e: &str) -> usize {
    ev.iter().position(|x| x == e).unwrap_or_else(|| panic!("no {e}: {ev:?}"))
}

#[test]
fn a_school_microsoft_mailbox_sets_up_in_order() {
    let r = rig("t113-m365");
    r.host.lock().unwrap().msgs = ana_mail(2);
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!(p.ended, Some("finished"));
    assert_eq!(p.line, Some("setup finished"));
    for step in ["sign_in", "step_aside", "forwarding", "canary", "backfill", "finish"] {
        assert_eq!(row(&p, step).state, "done", "{step}");
    }
    let ev = r.events();
    let at = |e: &str| position(&ev, e);
    assert!(at("svc mint") < at("window visible") && at("window visible") < at("script m365/signin.js"));
    assert!(at("script m365/signin.js") < at("svc hint") && at("svc hint") < at("aside"));
    assert!(at("aside") < at("script m365/forwarding_set.js"));
    assert!(at("script m365/forwarding_set.js") < at("script m365/forwarding_read.js"), "read back after navigating away and back");
    assert!(at("script m365/forwarding_read.js") < at("svc canary") && at("svc canary") < at("svc batch"));
    assert!(at("svc batch") < at("svc batch sent") && at("svc batch sent") < at("slot now"));
    let set = &r.calls("m365/forwarding_set.js")[0];
    assert_eq!((set["address"].clone(), set["replace"].clone()), (json!(KNOWLU), json!(false)));
    let places: Vec<&String> = ev.iter().filter(|e| e.starts_with("open ")).collect();
    assert_eq!(places[..3], ["open Forwarding", "open Inbox", "open Forwarding"], "away and back: {places:?}");
    assert_eq!(r.calls("m365/sent_cleanup.js").len(), 1);
    let h = r.host.lock().unwrap();
    assert_eq!((h.aside, h.closed >= 1), (1, true));
}

#[test]
fn a_gmail_setup_opens_the_confirm_window_before_adding_the_address_and_waits_out_the_popup() {
    let r = rig("t113-gmail");
    {
        let mut h = r.host.lock().unwrap();
        h.title_address = Some(HINT_PAGE.to_string());
        h.popup_polls = 4;
        h.msgs = ana_mail(1);
    }
    let p = wait(&r.start(Kind::Gmail, true));
    assert_eq!((p.ended, p.provider), (Some("finished"), "google"));
    for step in ["sign_in", "step_aside", "add_address", "confirm_address", "turn_on", "canary", "backfill", "finish"] {
        assert_eq!(row(&p, step).state, "done", "{step}");
    }
    let ev = r.events();
    let at = |e: &str| position(&ev, e);
    assert!(at("svc hint") < at("svc state confirming"), "the mailbox's own address is set first");
    assert!(at("svc state confirming") < at("script gmail/forwarding_add.js"), "the 15-minute confirm window opens before Google mails");
    assert!(at("script gmail/forwarding_add.js") < at("script gmail/forwarding_add_done.js"));
    assert!(at("script gmail/forwarding_add_done.js") < at("script gmail/forwarding_set.js"));
    assert!(at("script gmail/forwarding_set.js") < at("svc canary") && at("svc canary") < at("svc batch"));
    let asides: Vec<usize> = ev.iter().enumerate().filter(|(_, e)| *e == "aside").map(|(i, _)| i).collect();
    assert_eq!(asides.len(), 2, "the window steps aside, and again once the popup has closed");
    assert!(at("script gmail/forwarding_add.js") < asides[1] && asides[1] < at("script gmail/forwarding_add_done.js"));
    assert!(r.host.lock().unwrap().popup_polls == 0, "the runner waited the popup out");
}

fn queue(r: &Rig, script: &str, answers: Vec<Value>) {
    r.host.lock().unwrap().queued.insert(script.to_string(), answers.into());
}

#[test]
fn forwarding_elsewhere_is_asked_and_only_a_yes_replaces_it() {
    let ours = json!({ "result": "ok", "forwarding": "on", "to": "ours", "keep_copy": true, "changed": true });
    let r = rig("t113-else");
    queue(&r, "m365/forwarding_set.js", vec![json!({ "result": "elsewhere" }), ours]);
    let shared = r.start(curated_m365(), true);
    until("the question", || shared.progress().ask == Some("forwarding_elsewhere"));
    assert_eq!(row(&shared.progress(), "forwarding").state, "needs_you");
    assert!(shared.answer(Answer::Replace(true)));
    assert_eq!(wait(&shared).ended, Some("finished"));
    let sets = r.calls("m365/forwarding_set.js");
    assert_eq!((sets.len(), sets[0]["replace"].clone(), sets[1]["replace"].clone()), (2, json!(false), json!(true)));
    assert_eq!(shared.asked(), ["forwarding_elsewhere"]);

    let n = rig("t113-else-no");
    queue(&n, "m365/forwarding_set.js", vec![json!({ "result": "elsewhere" })]);
    let shared = n.start(curated_m365(), true);
    until("the question", || shared.progress().ask == Some("forwarding_elsewhere"));
    assert!(shared.answer(Answer::Replace(false)));
    assert_eq!(wait(&shared).ended, Some("stopped"));
    assert_eq!(n.calls("m365/forwarding_set.js").len(), 1, "nothing replaces it without a yes");
    assert!(!n.events().contains(&"svc canary".to_string()));
}

#[test]
fn a_gmail_script_that_reloads_is_run_again_once_with_reloaded_set() {
    let ours = json!({ "result": "ok", "forwarding": "on", "to": "ours", "keep_copy": true, "changed": true });
    let reloading = json!({ "result": "reloading" });
    let r = rig("t113-reload");
    r.host.lock().unwrap().title_address = Some(HINT_PAGE.to_string());
    queue(&r, "gmail/forwarding_set.js", vec![reloading.clone(), ours]);
    let p = wait(&r.start(Kind::Gmail, true));
    assert_eq!(row(&p, "turn_on").state, "done");
    let sets = r.calls("gmail/forwarding_set.js");
    assert_eq!(sets.len(), 2);
    assert_ne!(sets[0]["reloaded"], json!(true));
    assert_eq!(sets[1]["reloaded"], json!(true));

    // A second miss is "not found", and the step falls to its guided version.
    let t = rig("t113-reload-twice");
    t.host.lock().unwrap().title_address = Some(HINT_PAGE.to_string());
    queue(&t, "gmail/forwarding_set.js", vec![reloading]);
    let shared = t.start(Kind::Gmail, true);
    until("the failed step", || has(&shared, "turn_on", "failed"));
    let p = shared.progress();
    assert_eq!((row(&p, "turn_on").reason, row(&p, "turn_on").guided), (Some("not_found"), true));
    assert_eq!(t.calls("gmail/forwarding_set.js").len(), 2, "run again once, never in a loop");
    shared.stop();
    wait(&shared);
}

#[test]
fn the_progress_never_holds_an_address_or_a_token() {
    let r = rig("t113-private");
    r.host.lock().unwrap().msgs = ana_mail(1);
    let shared = r.start(uncurated(), true);
    answer_address(&shared);
    let p = wait(&shared);
    assert_eq!(p.ended, Some("finished"));
    let text = serde_json::to_string(&p).unwrap();
    for secret in [KNOWLU, HINT_PAGE, TYPED, TOKEN, ANA, "school.example", "@"] {
        assert!(!text.contains(secret), "the progress holds {secret}: {text}");
    }
}

// ---- H4: the launch hook ------------------------------------------------------------------------

/// `main.rs`'s `.setup` closures, as source text: `[0]` is the vault-less shell's, `[1]` the console's.
/// (The same stand-in `handler_lists.rs` uses: a call left out compiles and shows only at a launch.)
fn setup_bodies(src: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find(".setup(move |app|") {
        let body = &rest[at..];
        let end = body[1..].find(".setup(").or_else(|| body[1..].find(".run(")).map_or(body.len(), |e| e + 1);
        out.push(&body[..end]);
        rest = &body[end..];
    }
    out
}

#[test]
fn the_console_setup_calls_the_mail_launch_hook_after_it_manages_the_console_state() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("main.rs");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let bodies = setup_bodies(&src);
    assert_eq!(bodies.len(), 2, "main.rs has the wizard's setup and the console's");
    let console = bodies.iter().find(|b| b.contains("app.manage(cs)")).expect("the console's setup manages ConsoleState");
    let wizard = bodies.iter().find(|b| b.contains("onboarding::Onboarding")).expect("the wizard's setup");

    let hook = console.find("knowlu::mail::live::on_start(app.handle())").expect("the console's setup calls mail::live::on_start");
    assert!(hook > console.find("app.manage(cs)").unwrap(), "on_start reads ConsoleState with try_state, so it comes after the manage");
    assert!(hook < console.find("scheduler::spawn").unwrap(), "the sweep starts before the scheduler can run a slot");
    assert!(!wizard.contains("on_start"), "the vault-less shell has no vault to resume a backfill in");
}

// ---- T11.4: the ten commands' seams (spec §6.2, §4.12) -------------------------------------------------
//
// Each command is a thin wrapper over a seam in `mail::commands`; these tests drive the seams with the
// setup runner's fake window and a loopback service. Test 37 (the console's list names the ten, the
// wizard's none) is `handler_lists.rs`'s and waits for H5 (plan §10, R5).
use knowlu::mail::commands::{
    answer_with, applied_argv, applied_with, disconnect_with, keep_session_with, progress_of, setup_start_with,
    sign_in_with, status_for, stop_with, Disconnect,
};

fn row_for(provider: &str) -> Value {
    json!({ "address_id": A, "provider": provider, "address": KNOWLU, "status": "active", "mailbox_hint": HINT_PAGE })
}

#[test]
fn mail_status_reports_offer_pending_until_setup_starts_or_not_now() {
    let d = temp("t114-offer");
    let marker = knowlu::onboarding::mail_offer_marker(&d);
    // Not now: the wizard wrote no marker, so there is nothing to offer.
    assert_eq!(status_for(Ok(vec![]), &d, false)["offer_pending"], json!(false));
    // The wizard's announcement left its marker: the console's first open offers setup.
    std::fs::write(&marker, b"").unwrap();
    assert_eq!(status_for(Ok(vec![]), &d, false)["offer_pending"], json!(true));
    // A setup that did not start leaves the offer standing, and a name the page cannot start is refused.
    let busy = setup_start_with(&d, "school", Some("1".into()), |_| Err("a mail setup is already running"));
    assert_eq!((busy["ok"].clone(), busy["error"].clone()), (json!(false), json!("a mail setup is already running")));
    let bad = setup_start_with(&d, "yahoo", None, |_| panic!("nothing starts for an unknown provider"));
    assert_eq!(bad["ok"], json!(false));
    assert_eq!(status_for(Ok(vec![]), &d, false)["offer_pending"], json!(true));
    // Starting the setup is what answers the offer.
    let got = RefCell::new(None);
    let ok = setup_start_with(&d, "school", Some("1".into()), |k| {
        *got.borrow_mut() = Some(k);
        Ok(())
    });
    assert_eq!(ok["ok"], json!(true));
    assert_eq!(got.into_inner(), Some(Kind::School { unitid: Some("1".into()), address: None }));
    assert!(!marker.exists());
    assert_eq!(status_for(Ok(vec![]), &d, false)["offer_pending"], json!(false));
    let gmail = RefCell::new(None);
    setup_start_with(&d, "gmail", None, |k| {
        *gmail.borrow_mut() = Some(k);
        Ok(())
    });
    assert_eq!(gmail.into_inner(), Some(Kind::Gmail));
}

#[test]
fn mail_status_lists_each_mailbox_with_what_this_device_knows_and_names_a_failed_read() {
    let d = temp("t114-status");
    profile(&d, &format!("mail-session-{A}"));
    turn_off(&d, B).unwrap();
    MailState::update(&d, |s| {
        s.open_backfills.push(A.to_string());
        true
    })
    .unwrap();
    let mut other = row_for("google");
    other["address_id"] = json!(B);
    let s = status_for(Ok(vec![row_for("m365"), other]), &d, true);
    assert_eq!((s["ok"].clone(), s["keep_session"].clone(), s["setup_running"].clone()), (json!(true), json!(true), json!(true)));
    let rows = s["mailboxes"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (rows[0]["state"].clone(), rows[0]["open_backfill"].clone(), rows[0]["saved_sign_in"].clone()),
        (json!("active"), json!(true), json!(true))
    );
    assert_eq!((rows[1]["turned_off"].clone(), rows[1]["saved_sign_in"].clone()), (json!(true), json!(false)));
    // The service lapsed or was unreachable: the device's own facts still come, and the reason is a name.
    let down = status_for(Err("unreachable".into()), &d, false);
    assert_eq!((down["ok"].clone(), down["error"].clone(), down["mailboxes"].clone()), (json!(false), json!("unreachable"), json!([])));
    assert_eq!(down["keep_session"], json!(true));
}

#[test]
fn mail_keep_session_off_deletes_every_mail_profile() {
    let d = temp("t114-keep");
    for name in [format!("mail-session-{A}"), format!("mail-session-{B}"), "mail-setup".to_string(), "grades-session".to_string(), "other".to_string()] {
        profile(&d, &name);
    }
    let closed = Cell::new(0);
    let on = keep_session_with(&d, true, || closed.set(closed.get() + 1));
    assert_eq!((on["ok"].clone(), on["keep_session"].clone()), (json!(true), json!(true)));
    assert!(d.join(format!("mail-session-{A}")).exists(), "turning it on deletes nothing");
    assert_eq!(closed.get(), 0, "and closes nothing");

    let off = keep_session_with(&d, false, || closed.set(closed.get() + 1));
    assert_eq!((off["ok"].clone(), off["keep_session"].clone(), off["profiles_left"].clone()), (json!(true), json!(false), json!(false)));
    assert_eq!(closed.get(), 1, "the mail window closes before its profiles are deleted");
    assert!(!MailState::load(&d).unwrap().keep_session);
    for gone in [format!("mail-session-{A}"), format!("mail-session-{B}"), "mail-setup".to_string()] {
        assert!(!d.join(&gone).exists(), "{gone} is deleted");
    }
    for kept in ["grades-session", "other"] {
        assert!(d.join(kept).exists(), "{kept} is not a mail profile");
    }
    // An unreadable mail.json is named and deletes nothing: the switch cannot be recorded.
    let bad = temp("t114-keep-bad");
    profile(&bad, &format!("mail-session-{A}"));
    std::fs::write(bad.join("mail.json"), "not json").unwrap();
    let out = keep_session_with(&bad, false, || {});
    assert_eq!(out["ok"], json!(false));
    assert!(bad.join(format!("mail-session-{A}")).exists());
}

#[test]
fn mail_applied_passes_the_engines_json_through_unchanged() {
    let raw = r#"{"since": "2026-06-01", "courses": [{"course": "CS 101", "created": [], "changed": [{"old": 1, "new": 2.50, "note": "n"}], "archived": {"count": 0, "items": []}, "notices": []}]}"#;
    let vault = Path::new("V");
    let ran = RefCell::new(Vec::new());
    let got = applied_with(vault, Some("2026-06-01T00:00:00Z"), |argv| {
        *ran.borrow_mut() = argv.to_vec();
        Ok(raw.to_string())
    });
    assert_eq!(got, serde_json::from_str::<Value>(raw).unwrap(), "no key added, none dropped, none computed");
    assert_eq!(ran.into_inner(), ["mail-applied", "--vault", "V", "--since", "2026-06-01T00:00:00Z"]);
    assert_eq!(applied_argv(vault, None), ["mail-applied", "--vault", "V"]);
    for failed in [Err("could not run the engine".to_string()), Ok("not json".to_string()), Ok("[1]".to_string())] {
        let out = applied_with(vault, None, |_| failed);
        assert_eq!(out["ok"], json!(false));
        assert!(out["error"].as_str().is_some_and(|e| !e.is_empty()));
    }
}

#[test]
fn the_progress_the_answer_and_stop_read_the_run_that_is_there_and_say_so_when_none_is() {
    assert_eq!(progress_of(None)["active"], json!(false));
    assert_eq!(stop_with(None)["ok"], json!(false));
    assert_eq!(answer_with(None, &json!("replace"))["ok"], json!(false));
    let r = rig("t114-answer");
    let shared = r.start(uncurated(), true);
    until("the question", || shared.progress().ask == Some("school_address"));
    let p = progress_of(r.setups.current());
    assert_eq!((p["active"].clone(), p["progress"]["ask"].clone()), (json!(true), json!("school_address")));
    // A choice the open question cannot take is refused and changes nothing.
    assert_eq!(answer_with(r.setups.current(), &json!("replace"))["ok"], json!(false));
    assert_eq!(answer_with(r.setups.current(), &json!("nonsense"))["ok"], json!(false));
    assert_eq!(answer_with(r.setups.current(), &json!({ "school_address": TYPED }))["ok"], json!(true));
    assert_eq!(stop_with(r.setups.current())["ok"], json!(true));
    wait(&shared);
}

// ---- §4.12 Disconnect and the repair sign-in ------------------------------------------------------------

/// A service for Disconnect: it lists one mailbox of `provider` and answers the delete with
/// `delete_status`. Each request lands in the rig's events with whether `mail.json` already listed the
/// mailbox as turned off when it arrived (the service's thread reads the file as the request comes in).
fn disc_service(r: &Rig, provider: &str, delete_status: u16) -> Service {
    let (ev, dir, provider) = (r.ev.clone(), r.dir.clone(), provider.to_string());
    Service::start(false, move |req| {
        let path = req.path.split("/mail").nth(1).unwrap_or("").to_string();
        let off = MailState::load(&dir).is_ok_and(|s| s.is_turned_off(A));
        ev.lock().unwrap().push(format!("svc {} {} turned_off={off}", req.method, path.split('/').nth(1).unwrap_or("")));
        match (req.method.as_str(), path.as_str()) {
            ("GET", "/addresses") => (200, json!({ "addresses": [row_for(&provider)] }).to_string()),
            ("DELETE", p) if p.starts_with("/addresses/") => (delete_status, r#"{"deleted":true}"#.to_string()),
            _ => (404, r#"{"error":"no such route"}"#.to_string()),
        }
    })
}

fn disconnect(r: &Rig, svc: &Service) -> Value {
    let host = SHost { st: r.host.clone(), ev: r.ev.clone(), provider: Mutex::new(Provider::Guided) };
    let client = MailClient::with_token_fn(&svc.base, Box::new(|| Ok(TOKEN.to_string()))).expect("a loopback base");
    let clock = ClkRef(r.clk.clone());
    disconnect_with(&Disconnect { host: &host, client: &client, clock: &clock, data_dir: &r.dir }, A)
}

fn step<'a>(out: &'a Value, name: &str) -> &'a Value {
    out["steps"].as_array().and_then(|s| s.iter().find(|x| x["step"] == name)).unwrap_or_else(|| panic!("no step {name}: {out}"))
}

#[test]
fn mail_disconnect_writes_turned_off_before_anything_else() {
    let r = rig("t114-order");
    profile(&r.dir, &format!("mail-session-{A}"));
    let svc = disc_service(&r, "m365", 200);
    let out = disconnect(&r, &svc);
    assert_eq!(out["ok"], json!(true), "{out}");
    let ev = r.events();
    assert!(ev[0].starts_with("svc GET addresses"), "the first thing is the read of the mailbox: {ev:?}");
    for e in ev.iter().filter(|e| e.starts_with("svc ")) {
        assert!(e.ends_with("turned_off=true"), "{e}: no request left before the mailbox was recorded as turned off");
    }
    let at = |e: &str| position(&ev, e);
    assert!(at("svc GET addresses turned_off=true") < at("window hidden"));
    assert!(at("window hidden") < at("script m365/forwarding_off.js"));
    assert!(at("script m365/forwarding_off.js") < at("svc DELETE addresses turned_off=true"));
    assert_eq!(r.scripts(), ["m365/forwarding_off.js"], "Disconnect never turns forwarding on, and a Microsoft mailbox has no address to remove");
    assert_eq!(r.calls("m365/forwarding_off.js")[0]["address"], json!(KNOWLU));
    assert_eq!((step(&out, "forwarding_off")["state"].clone(), step(&out, "revoke")["state"].clone()), (json!("done"), json!("done")));
    assert_eq!(out["needs_you"], json!(false));
    assert!(!r.dir.join(format!("mail-session-{A}")).exists(), "step 3: the mailbox's profile is deleted");
    assert!(r.host.lock().unwrap().closed >= 1, "and its window closed first");
    assert!(MailState::load(&r.dir).unwrap().is_turned_off(A), "turned_off only grows");

    // A server call that fails leaves it recorded, leaves the profile to try again, and says so.
    let f = rig("t114-order-fail");
    profile(&f.dir, &format!("mail-session-{A}"));
    let svc = disc_service(&f, "m365", 500);
    let out = disconnect(&f, &svc);
    assert_eq!((out["ok"].clone(), step(&out, "revoke")["state"].clone()), (json!(false), json!("failed")));
    assert!(MailState::load(&f.dir).unwrap().is_turned_off(A));
    assert!(f.dir.join(format!("mail-session-{A}")).exists());
    assert_eq!(f.calls("m365/forwarding_set.js").len(), 0);

    // mail.json that cannot be read cannot record it: nothing at all is done.
    let u = rig("t114-order-bad");
    std::fs::write(u.dir.join("mail.json"), "not json").unwrap();
    let svc = disc_service(&u, "m365", 200);
    let out = disconnect(&u, &svc);
    assert_eq!(out["ok"], json!(false));
    assert!(u.events().is_empty(), "no window, script or call: {:?}", u.events());
}

#[test]
fn mail_disconnect_gmail_removal_waits_for_the_live_remove_check() {
    let r = rig("t114-gmail");
    profile(&r.dir, &format!("mail-session-{A}"));
    let needs_you = json!({ "result": "needs you", "reason": "another address is selected" });
    queue(&r, "gmail/forwarding_remove_address.js", vec![needs_you]);
    let svc = disc_service(&r, "google", 200);
    let out = disconnect(&r, &svc);
    assert_eq!(r.scripts(), ["gmail/forwarding_off.js", "gmail/forwarding_remove_address.js"], "one remove, no retry, no other click");
    assert_eq!(r.calls("gmail/forwarding_remove_address.js")[0]["address"], json!(KNOWLU));
    let s = step(&out, "remove_address");
    assert_eq!(s["state"], json!("needs_you"), "never done: {out}");
    assert!(s["message"].as_str().is_some_and(|m| m.contains("Gmail")), "it says the address is still in Gmail: {s}");
    assert_eq!((out["needs_you"].clone(), out["address"].clone()), (json!(true), json!(KNOWLU)), "the address is shown so it can be found");
    // The step before it did its work, and the rest of Disconnect still runs.
    assert_eq!(step(&out, "forwarding_off")["state"], json!("done"));
    assert_eq!((out["ok"].clone(), step(&out, "revoke")["state"].clone()), (json!(true), json!("done")));

    // Removed for real: done, and nothing is asked of the student.
    let ok = rig("t114-gmail-ok");
    profile(&ok.dir, &format!("mail-session-{A}"));
    let svc = disc_service(&ok, "google", 200);
    let out = disconnect(&ok, &svc);
    assert_eq!((step(&out, "remove_address")["state"].clone(), out["needs_you"].clone(), out["address"].clone()), (json!("done"), json!(false), Value::Null));

    // A page that will not give the section up falls to the guided steps, with the address shown.
    let g = rig("t114-gmail-guided");
    profile(&g.dir, &format!("mail-session-{A}"));
    queue(&g, "gmail/forwarding_off.js", vec![json!({ "result": "not found", "control": "section" })]);
    let svc = disc_service(&g, "google", 200);
    let out = disconnect(&g, &svc);
    assert_eq!((step(&out, "forwarding_off")["state"].clone(), out["address"].clone()), (json!("guided"), json!(KNOWLU)));
    assert!(g.calls("gmail/forwarding_remove_address.js").is_empty(), "nothing is removed after a step that did not run");
    assert_eq!(step(&out, "revoke")["state"], json!("done"));
}

#[test]
fn a_signed_out_mailbox_asks_for_a_visible_sign_in_and_a_declined_one_falls_to_the_guided_steps() {
    let r = rig("t114-signin");
    profile(&r.dir, &format!("mail-session-{A}"));
    r.host.lock().unwrap().signed_in = false;
    // The student signs in as soon as the visible window is up.
    r.host.lock().unwrap().sign_in_on_visible = true;
    let svc = disc_service(&r, "m365", 200);
    let out = disconnect(&r, &svc);
    let opens: Vec<bool> = r.host.lock().unwrap().opens.iter().map(|(_, v)| *v).collect();
    assert_eq!(opens, [false, true], "hidden first, then the visible window for the sign-in");
    assert_eq!(step(&out, "forwarding_off")["state"], json!("done"));

    // Declined: the student closes the visible window. The guided steps, and the rest still runs.
    let d = rig("t114-signin-no");
    profile(&d.dir, &format!("mail-session-{A}"));
    {
        let mut h = d.host.lock().unwrap();
        h.signed_in = false;
        h.student_closed = true;
    }
    let svc = disc_service(&d, "m365", 200);
    let out = disconnect(&d, &svc);
    assert_eq!((step(&out, "forwarding_off")["state"].clone(), out["address"].clone()), (json!("guided"), json!(KNOWLU)));
    assert!(d.scripts().is_empty());
    assert_eq!(step(&out, "revoke")["state"], json!("done"));
}

#[test]
fn the_repair_sign_in_opens_the_visible_window_on_the_kept_profile_and_leaves_it_kept() {
    let r = rig("t114-repair");
    profile(&r.dir, &format!("mail-session-{A}"));
    r.host.lock().unwrap().signed_in = true;
    let host = SHost { st: r.host.clone(), ev: r.ev.clone(), provider: Mutex::new(Provider::Guided) };
    let clock = ClkRef(r.clk.clone());
    let out = sign_in_with(&host, &clock, &r.dir, Provider::M365, A);
    assert_eq!((out["ok"].clone(), out["signed_in"].clone()), (json!(true), json!(true)));
    let h = r.host.lock().unwrap();
    assert_eq!(h.opens, [(r.dir.join(format!("mail-session-{A}")), true)]);
    assert!(h.closed >= 1, "the window closes once the sign-in has taken");
    assert!(r.dir.join(format!("mail-session-{A}")).exists(), "the profile is the kept sign-in");
    drop(h);

    // A window the student closes is a sign-in that did not happen, named, and the profile stays.
    let c = rig("t114-repair-closed");
    profile(&c.dir, &format!("mail-session-{A}"));
    {
        let mut h = c.host.lock().unwrap();
        h.signed_in = false;
        h.student_closed = true;
    }
    let host = SHost { st: c.host.clone(), ev: c.ev.clone(), provider: Mutex::new(Provider::Guided) };
    let out = sign_in_with(&host, &ClkRef(c.clk.clone()), &c.dir, Provider::M365, A);
    assert_eq!((out["ok"].clone(), out["signed_in"].clone()), (json!(false), json!(false)));
    assert!(c.dir.join(format!("mail-session-{A}")).exists());
    // A guided mailbox has no window to open.
    let out = sign_in_with(&host, &ClkRef(c.clk.clone()), &c.dir, Provider::Guided, A);
    assert_eq!(out["ok"], json!(false));
}

#[test]
fn a_rotate_asks_the_service_for_a_new_address_and_runs_the_forwarding_step_with_it() {
    let r = rig("t114-rotate");
    profile(&r.dir, &format!("mail-session-{A}"));
    r.sv.lock().unwrap().provider = "m365".into();
    let kind = Kind::Rotate { address_id: A.to_string(), provider: "m365".to_string() };
    let p = wait(&r.start(kind.clone(), true));
    assert_eq!((p.ended, p.line), (Some("finished"), Some("setup finished")));
    let ev = r.events();
    let at = |e: &str| position(&ev, e);
    assert!(at("window visible") < at("aside") && at("aside") < at("svc rotate"), "the address is rotated only once the sign-in has taken");
    assert!(at("svc rotate") < at("script m365/forwarding_set.js") && at("script m365/forwarding_set.js") < at("svc canary"));
    assert_eq!(r.calls("m365/forwarding_set.js")[0]["address"], json!(KNOWLU_NEW), "the step carries the new address");
    for never in ["svc mint", "svc hint", "svc delete address", "svc batch"] {
        assert!(!ev.contains(&never.to_string()), "a rotate never does `{never}`: {ev:?}");
    }
    assert_eq!(r.calls("m365/signin.js").len(), 0, "the mailbox's own address is set once, at its setup");
    assert!(p.backfill.is_none(), "mail already brought in is not brought in again");
    assert!(r.dir.join(format!("mail-session-{A}")).exists(), "the kept profile stays");

    // A window the student closes cancels the rotate and never revokes the mailbox.
    let c = rig("t114-rotate-closed");
    profile(&c.dir, &format!("mail-session-{A}"));
    c.sv.lock().unwrap().provider = "m365".into();
    {
        let mut h = c.host.lock().unwrap();
        h.signed_in = false;
        h.student_closed = true;
    }
    let p = wait(&c.start(kind, true));
    assert_eq!(p.ended, Some("cancelled"));
    assert!(!c.events().contains(&"svc delete address".to_string()), "the mailbox is not deleted");
    assert!(!c.events().contains(&"svc rotate".to_string()), "a rotate that never signed in changed nothing on the service: {:?}", c.events());
    assert!(c.dir.join(format!("mail-session-{A}")).exists(), "and its profile is not either");
}

#[test]
fn a_rotate_that_stops_after_rotating_ends_needs_you_naming_the_old_addresss_grace() {
    let r = rig("t114-rotate-stop");
    profile(&r.dir, &format!("mail-session-{A}"));
    r.sv.lock().unwrap().provider = "m365".into();
    let setups = r.setups.clone();
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "m365/forwarding_set.js" {
            setups.current().expect("a setup is running").stop();
        }
    }));
    let kind = Kind::Rotate { address_id: A.to_string(), provider: "m365".to_string() };
    let p = wait(&r.start(kind, true));
    assert_eq!(p.ended, Some("needs_you"), "the address was rotated: that is never a plain stop or cancel");
    assert!(p.line.is_some_and(|l| l.contains("7 days")), "the line names the old address's grace: {:?}", p.line);
    assert!(r.events().contains(&"svc rotate".to_string()));
    assert!(!r.events().contains(&"svc delete address".to_string()), "the mailbox is not deleted");
    assert!(r.dir.join(format!("mail-session-{A}")).exists(), "the kept profile stays");

    // The service refusing the rotate changes nothing: a named failure, not a needs-you.
    let f = rig("t114-rotate-refused");
    profile(&f.dir, &format!("mail-session-{A}"));
    f.sv.lock().unwrap().provider = "m365".into();
    f.sv.lock().unwrap().rotate_refused = true;
    let kind = Kind::Rotate { address_id: A.to_string(), provider: "m365".to_string() };
    let p = wait(&f.start(kind, true));
    assert_eq!(p.ended, Some("failed"));
    assert!(f.scripts().is_empty(), "no forwarding step ran on an address that was never rotated: {:?}", f.scripts());
}
