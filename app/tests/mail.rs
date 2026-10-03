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
const NONCE: &str = "bk7f3a9c2e41";
const ANA: &str = "ana.lopez@school.example.invalid";
const BO: &str = "bo.chen@school.example.invalid";

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
                    .map(|(who, at)| json!({ "sender": who, "subject": m.subject, "received": received(*at), "received_at": iso(*at) }))
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
                json!({ "result": "ok", "deleted": self.forwards.borrow().len() })
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
    Job { provider, address_id: A, knowlu_address: KNOWLU, phase, window, cap }
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
