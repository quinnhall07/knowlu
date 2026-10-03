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
    forget, forget_all, is_session_dir, session_dir, setup_dir, start_up_sweep, turn_off, MailState,
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
