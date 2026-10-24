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
    forget, forget_all, is_session_dir, session_dir, setup_dir, start_up_sweep, turn_off, BackfillPace, LoneStall, MailState,
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

/// Spec §4.11 step 3 (amended 2026-10-19): `covered` carries the two stamps exactly as they were read (a reparse
/// would lose the microseconds and the offset), and nothing else; its answer is whether the service cleared.
#[test]
fn covered_posts_both_stamps_verbatim_and_answers_whether_the_service_cleared() {
    let since = "2026-09-11T09:13:20.123456-05:00";
    let last = "2026-09-13T00:00:00.654321+00:00";
    let svc = Service::start(false, |r| {
        let cleared = r.body.contains("123456");
        (200, json!({ "address_id": A, "state": "covered", "status": "quiet", "cleared": cleared }).to_string())
    });
    let c = client(&svc);
    assert_eq!(c.covered(A, since, last), Ok(true));
    assert_eq!(c.covered(A, "2026-09-11T09:13:20-05:00", last), Ok(false), "cleared: false is an answer, not an error");
    let seen = svc.requests();
    assert_eq!(seen.len(), 2);
    assert_eq!((seen[0].method.as_str(), seen[0].path.as_str()), ("POST", format!("/functions/v1/mail/addresses/{A}/state").as_str()));
    assert_eq!(seen[0].bearer.as_deref(), Some(format!("Bearer {TOKEN}").as_str()));
    let body: serde_json::Value = serde_json::from_str(&seen[0].body).unwrap();
    assert_eq!(body, json!({ "state": "covered", "lapsed_since": since, "last_lapsed_at": last }), "exactly these keys");
    assert!(seen[0].body.contains(since) && seen[0].body.contains(last), "byte for byte: {}", seen[0].body);
    // A malformed stamp is the service's 400; a reply with no `cleared` is unreadable; the route needs an entitlement.
    let bad = Service::start(false, |_| (400, r#"{"error":"lapsed_since and last_lapsed_at must be the mailbox's own, each an instant with its zone"}"#.to_string()));
    assert!(matches!(client(&bad).covered(A, "x", last), Err(MailError::Refused { status: 400, .. })));
    let odd = Service::start(false, |_| (200, r#"{"ok":true}"#.to_string()));
    assert_eq!(client(&odd).covered(A, since, last), Err(MailError::BadReply));
    let lapsed = Service::start(false, |_| (402, r#"{"error":"this account has no active subscription"}"#.to_string()));
    assert_eq!(client(&lapsed).covered(A, since, last), Err(MailError::Lapsed));
    // An id that could change the route is never sent.
    let quiet = Service::start(false, |_| (200, "{}".to_string()));
    assert_eq!(client(&quiet).covered("../x", since, last), Err(MailError::Invalid));
    assert!(quiet.requests().is_empty());
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
    backfill_window, fingerprint, run_pass, window, Clock, Held, Job, Mailbox, Place, Stop, Stuck, Window, SLOT_CAP,
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
    /// Microsoft 365: a non-empty `convo` is a conversation row, which `list.js` opens (T18c).
    count: Option<u32>,
    convo: Vec<(String, i64)>,
    /// Microsoft 365: `list.js` could not read this conversation whole, and counts it `left`.
    unreadable: bool,
}

fn mail(sender: &str, subject: &str, at: i64) -> Mail {
    Mail { sender: sender.into(), subject: subject.into(), at, count: None, convo: vec![], unreadable: false }
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
    /// Gmail: the search lands on the newest page and each `go: "oldest"` walks at most this many pages
    /// toward the oldest, saying `at_oldest` (a search has no Oldest button; `list.js` walks in a budget).
    oldest_walk: Option<usize>,
    /// Gmail: every forward's attachments fail to upload.
    attach_fails: Option<&'static str>,
    /// Gmail: this many forwards from now draw no recipient ("recipient not accepted", none read; live ef28).
    recipient_misses: Cell<u32>,
    /// Gmail: a `recipient_misses` forward read another address drawn (recipients 1, no second try in place).
    other_recipient: bool,
    /// Gmail: the Inbox holds a non-delivery report for the batch once this many forwards have run.
    bounce_after: Option<usize>,
    /// Gmail: the Inbox draws no list (`bounce.js` says `not found`).
    inbox_unreadable: bool,
    /// Gmail: the single message with this subject never attaches, so any send carrying it is one short.
    no_chip: Option<&'static str>,
    /// Gmail: the single messages with these subjects never attach either, as `no_chip`.
    no_chips: Vec<&'static str>,
    /// Gmail: the single message with this subject's chip never finishes uploading, so any send carrying it
    /// answers "attachment still uploading" (live: one huge message).
    slow_chip: Option<&'static str>,
    /// Gmail: `slow_chip` does attach when `forward.js` is given at least this `upload_ms` (it waits 30 s
    /// without one): a slow but real upload. `None`: it never finishes.
    slow_chip_ms: Option<u64>,
    /// Gmail: no chip finishes uploading anywhere (a Gmail-wide stall): every forward answers "attachment still
    /// uploading".
    stall: bool,
    /// Gmail: a non-delivery report landed in the Inbox at this instant; a look whose `since` is at or before it
    /// finds it (a look after it does not: it was seen).
    report_at: Cell<Option<i64>>,
    /// Gmail: the clock at each look that read the Inbox.
    looked_at: RefCell<Vec<i64>>,
    /// Gmail: every page of the results has a newer one (a result that never ends).
    endless: bool,
    /// Gmail: a search has run in this document.
    searched: Cell<bool>,
    /// Gmail (live 2026-10-19): the Inbox was opened again in the same document after a search, so Gmail
    /// restores its cached view of the search, "of many", past the real end: a search answers a wrong count and
    /// the list will not move. Only a fresh load clears it.
    stale: Cell<bool>,
    /// The fresh Inbox cannot be opened after this many opens of it.
    fresh_refuses_after: Option<usize>,
    /// The clock at each open of the fresh Inbox.
    fresh_at: RefCell<Vec<i64>>,
    /// Every open, with the clock when it ran.
    opened_at: RefCell<Vec<(Place, i64)>>,
    /// Gmail: the account's zone, in seconds east of UTC: `after:` and `before:` compare the local day (live
    /// 2026-10-19: `before:` cuts at the account's local midnight).
    zone: i64,
    /// Gmail: the pager presses the scripts reported.
    presses: Cell<u64>,
    /// Gmail: the next `go: "older"` finds no range and no button to say (list.js: "page range unreadable").
    range_unreadable: Cell<bool>,
    /// Gmail: the next `go: "newer"` (or the next conversation read) runs out its step with no answer.
    newer_times_out: Cell<bool>,
    thread_times_out: Cell<bool>,
    /// `mail.json`'s folder: each `forward.js` call records the day's Gmail messages it holds then, in `paces`.
    pace_dir: Option<PathBuf>,
    paces: RefCell<Vec<u32>>,
    /// Gmail: each original a send that went carries is added here as a held entry (`learning`'s service holds them).
    learn: Option<Arc<Mutex<Vec<Value>>>>,
    /// Sent copies an earlier pass left, as (nonce, the one recipient): `sent_cleanup.js` for any other nonce deletes
    /// the ones matching its nonce and address, up to its `max`.
    old_copies: RefCell<Vec<(String, String)>>,
    /// The next this many opens of Sent are refused.
    sent_opens_fail: Cell<u32>,
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
        oldest_walk: None,
        attach_fails: None,
        recipient_misses: Cell::new(0),
        other_recipient: false,
        bounce_after: None,
        inbox_unreadable: false,
        no_chip: None,
        no_chips: Vec::new(),
        slow_chip: None,
        slow_chip_ms: None,
        stall: false,
        report_at: Cell::new(None),
        looked_at: Default::default(),
        endless: false,
        searched: Cell::new(false),
        stale: Cell::new(false),
        fresh_refuses_after: None,
        fresh_at: Default::default(),
        opened_at: Default::default(),
        zone: 0,
        presses: Cell::new(0),
        range_unreadable: Cell::new(false),
        newer_times_out: Cell::new(false),
        thread_times_out: Cell::new(false),
        pace_dir: None,
        paces: Default::default(),
        learn: None,
        old_copies: Default::default(),
        sent_opens_fail: Cell::new(0),
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
        self.opened_at.borrow_mut().push((place, self.clk.now()));
        self.log.borrow_mut().push((format!("open {place:?}"), Value::Null));
        if place == Place::FreshInbox {
            self.fresh_at.borrow_mut().push(self.clk.now());
        }
        if place == Place::Sent && self.sent_opens_fail.get() > 0 {
            self.sent_opens_fail.set(self.sent_opens_fail.get() - 1);
            return Err(Refusal::TimedOut);
        }
        match place {
            Place::FreshInbox if self.fresh_refuses_after.is_some_and(|n| self.fresh_at.borrow().len() > n) => {
                return Err(Refusal::TimedOut)
            }
            Place::FreshInbox => {
                self.stale.set(false);
                self.page.set(0);
            }
            Place::Inbox if self.searched.get() => self.stale.set(true),
            _ => {}
        }
        Ok(())
    }

    fn script(&self, path: &str, args: &Value) -> Result<Value, Refusal> {
        self.log.borrow_mut().push((path.to_string(), args.clone()));
        let timed_out = |flag: &Cell<bool>| flag.replace(false);
        if (path.ends_with("list.js") && args["go"] == "newer" && timed_out(&self.newer_times_out))
            || (path.ends_with("thread.js") && timed_out(&self.thread_times_out))
        {
            return Err(Refusal::TimedOut);
        }
        let v = self.answer(path, args)?;
        self.presses.set(self.presses.get() + v.get("presses").and_then(Value::as_u64).unwrap_or(0));
        Ok(v)
    }
}

impl Fake {
    /// Page loads as the routine counts them: every open (a fresh Inbox is two, the navigation and the reload),
    /// every search and every press a script reported (the pager's, and a conversation's open and back).
    fn loads(&self) -> u64 {
        let fresh = self.opened_at.borrow().iter().filter(|(p, _)| *p == Place::FreshInbox).count() as u64;
        self.opened_at.borrow().len() as u64 + fresh + self.calls("search.js").len() as u64 + self.presses.get()
    }

    /// The local day of an instant, as Gmail's `after:` and `before:` compare it.
    fn local_day(&self, at: i64) -> String {
        iso(at + if self.provider == Provider::Gmail { self.zone } else { 0 })[..10].to_string()
    }

    fn answer(&self, path: &str, args: &Value) -> Result<Value, Refusal> {
        let (dir, name) = path.split_once('/').unwrap();
        assert_eq!(dir, if self.provider == Provider::Gmail { "gmail" } else { "m365" }, "{path}");
        let m365 = self.provider == Provider::M365;
        Ok(match name {
            "search.js" if self.stale.get() => json!({ "result": "ok", "count": 5130, "presses": 0 }),
            "search.js" => {
                self.searched.set(true);
                let since = args["since"].as_str().unwrap();
                let before = args.get("until").and_then(Value::as_str);
                // A conversation is found by any message in it (Microsoft 365's conversation view).
                let mut idx: Vec<usize> = (0..self.msgs.len())
                    .filter(|&i| {
                        let m = &self.msgs[i];
                        std::iter::once(m.at).chain(m.convo.iter().map(|c| c.1)).any(|at| {
                            let day = self.local_day(at);
                            day.as_str() >= since && before.is_none_or(|u| day.as_str() < u)
                        })
                    })
                    .collect();
                idx.sort_by_key(|&i| self.msgs[i].at);
                if m365 {
                    idx.reverse();
                }
                let n = idx.len();
                *self.current.borrow_mut() = idx;
                if m365 {
                    self.page.set(0);
                    return Ok(json!({ "result": "ok", "count": n }));
                }
                // Gmail lands on page 1, the newest; an "of many" result is walked to the oldest page within
                // `max_presses` (and `oldest_walk`, its time), unless `walk` is false.
                let newest = n.div_ceil(self.size).max(1) - 1;
                let walk = args.get("walk") != Some(&json!(false));
                let max = args.get("max_presses").and_then(Value::as_u64).unwrap_or(40) as usize;
                let on = |page: usize| (page * self.size).min(n)..((page + 1) * self.size).min(n);
                if newest == 0 {
                    self.page.set(0);
                    json!({ "result": "ok", "count": n, "exact": true, "presses": 0 })
                } else if !walk {
                    self.page.set(newest);
                    json!({ "result": "ok", "count": on(newest).len(), "exact": false, "at_oldest": false, "presses": 0 })
                } else {
                    let can = max.min(self.oldest_walk.unwrap_or(usize::MAX));
                    if can >= newest {
                        self.page.set(0);
                        json!({ "result": "ok", "count": n, "exact": true, "at_oldest": true, "presses": newest })
                    } else {
                        self.page.set(newest - can);
                        let counted: usize = (newest - can..=newest).map(|p| on(p).len()).sum();
                        json!({ "result": "ok", "count": counted, "exact": false, "at_oldest": false, "presses": can })
                    }
                }
            }
            "list.js" if m365 => {
                // A conversation row is opened and each message in it read (T18c), numbered by conversation.
                let cur = self.current.borrow();
                let (mut rows, mut convos, mut left) = (Vec::new(), 0, 0);
                for &i in cur.iter().take(self.drawn) {
                    let m = &self.msgs[i];
                    if m.convo.is_empty() {
                        rows.push(self.row(i, false));
                        continue;
                    }
                    convos += 1;
                    if m.unreadable {
                        left += 1;
                        continue;
                    }
                    for (who, at) in &m.convo {
                        let mut v = json!({ "sender": who, "subject": m.subject, "received": received(*at), "received_at": iso(*at), "conversation": convos });
                        if who == ME {
                            v["own"] = json!(true);
                        }
                        rows.push(v);
                    }
                }
                let drawn = cur.len().min(self.drawn);
                json!({ "result": "ok", "total": cur.len(), "drawn": drawn, "left": left, "rows": rows })
            }
            "list.js" if self.stale.get() => json!({ "result": "failed", "reason": "page did not change" }),
            "list.js" if args["go"] == "older" && self.range_unreadable.replace(false) => {
                json!({ "result": "failed", "reason": "page range unreadable", "presses": 0 })
            }
            "list.js" if self.endless && args["go"] == "newer" => {
                if args["max_presses"] == 0 {
                    return Ok(json!({ "result": "failed", "reason": "no presses left", "presses": 0 }));
                }
                json!({ "result": "ok", "page": { "from": 9, "to": 9, "total": 9 }, "moved": true, "rows": [], "presses": 1 })
            }
            "list.js" => {
                let cur = self.current.borrow();
                let pages = cur.len().div_ceil(self.size).max(1);
                let before = self.page.get();
                let max = args.get("max_presses").and_then(Value::as_u64).unwrap_or(40) as usize;
                let to = match args["go"].as_str() {
                    Some("oldest") => before.saturating_sub(max.min(self.oldest_walk.unwrap_or(usize::MAX))),
                    Some("newer") if before + 1 < pages => before + 1,
                    Some("older") if before > 0 => before - 1,
                    _ => before,
                };
                let needed = match args["go"].as_str() {
                    Some("oldest" | "older") => before > 0,
                    Some("newer") => before + 1 < pages,
                    _ => false,
                };
                if needed && max == 0 {
                    return Ok(json!({ "result": "failed", "reason": "no presses left", "presses": 0 }));
                }
                self.page.set(to);
                let at = self.page.get();
                let (start, end) = (at * self.size, ((at + 1) * self.size).min(cur.len()));
                let rows: Vec<Value> = cur[start..end].iter().map(|&i| self.row(i, true)).collect();
                let page = json!({ "from": cur.len() - end + 1 + usize::from(self.endless), "to": cur.len() - start, "total": cur.len() });
                let mut v = json!({ "result": "ok", "page": page, "moved": at != before, "rows": rows, "presses": at.abs_diff(before) });
                if args["go"] == "oldest" {
                    v["at_oldest"] = json!(at == 0);
                }
                v
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
                // The row's open and the back press, each a page load (thread.js counts them).
                json!({ "result": "ok", "messages": messages, "presses": 2 })
            }
            "forward.js" => {
                assert_eq!(args["address"], KNOWLU);
                if let Some(dir) = &self.pace_dir {
                    let kept = MailState::load(dir).ok().and_then(|s| s.backfill_pace.get(A).map(|p| p.messages_today));
                    self.paces.borrow_mut().push(kept.unwrap_or(0));
                }
                let n = self.forwards.borrow().len();
                let rows: Vec<Value> = args.get("rows").and_then(Value::as_array).cloned().unwrap_or_else(|| vec![args.clone()]);
                for r in &rows {
                    let (who, subject, text) = (r["sender"].as_str().unwrap(), r["subject"].as_str().unwrap(), r["received"].as_str().unwrap());
                    // A message read inside a Microsoft 365 conversation is asked for as one (forward.js opens them).
                    if m365 && r.get("in_conversation") == Some(&json!(true)) {
                        let held = self.msgs.iter().any(|m| {
                            m.subject == subject && m.convo.iter().any(|(s, at)| s == who && received(*at) == text)
                        });
                        assert!(held, "the routine asked for a conversation message the page does not show");
                    } else {
                        self.find(who, subject, text);
                    }
                }
                self.forwards.borrow_mut().push((self.clk.now(), args.clone()));
                if self.refuse_on.get() == Some(n) {
                    return Ok(json!({ "result": "refused", "reason": "send failure" }));
                }
                if m365 {
                    return Ok(json!({ "result": "ok", "sent": true }));
                }
                if let Some(reason) = self.attach_fails {
                    return Ok(json!({ "result": "failed", "reason": reason }));
                }
                if self.recipient_misses.get() > 0 {
                    self.recipient_misses.set(self.recipient_misses.get() - 1);
                    if self.other_recipient {
                        return Ok(json!({ "result": "failed", "reason": "recipient not accepted", "recipients": 1 }));
                    }
                    return Ok(json!({ "result": "failed", "reason": "recipient not accepted", "recipients": 0, "retried": "kept" }));
                }
                let want: u64 = rows.iter().map(|r| r["messages"].as_u64().unwrap_or(1)).sum();
                if self.over_rows.is_some_and(|k| rows.len() > k) {
                    return Ok(json!({ "result": "failed", "reason": "over 25 MB" }));
                }
                if self.mismatch && want > rows.len() as u64 {
                    // G1 (live, ef25 2026-10-19): a conversation's "Forward as attachment" draws one chip of its many.
                    let reason = "attachments do not match the rows";
                    return Ok(json!({ "result": "failed", "reason": reason, "attached": 1, "expected": want }));
                }
                if rows.iter().any(|r| self.no_chip.is_some_and(|s| r["subject"] == s) || self.no_chips.iter().any(|s| r["subject"] == *s)) {
                    // One row whose chips never draw (a message, or a conversation's) is a compose with no
                    // attachment (forward.js checks that before the count); beside others the group is short.
                    if rows.len() == 1 {
                        return Ok(json!({ "result": "failed", "reason": "no attachment" }));
                    }
                    let reason = "attachments do not match the rows";
                    return Ok(json!({ "result": "failed", "reason": reason, "attached": want - 1, "expected": want }));
                }
                let upload_ms = args.get("upload_ms").map(|u| u.as_u64().expect("upload_ms is a number")).unwrap_or(30_000);
                let slow = self.slow_chip.is_some_and(|s| rows.iter().any(|r| r["subject"] == s))
                    && self.slow_chip_ms.is_none_or(|need| upload_ms < need);
                if self.stall || slow {
                    let reason = "attachment still uploading";
                    return Ok(json!({ "result": "failed", "reason": reason, "attached": want - 1, "expected": want }));
                }
                if let Some(learn) = &self.learn {
                    for r in &rows {
                        let at: i64 = r["received"].as_str().unwrap().trim_start_matches("at-").parse().unwrap();
                        let fp = fingerprint(FPKEY, r["sender"].as_str().unwrap(), r["subject"].as_str().unwrap());
                        learn.lock().unwrap().push(json!({ "fp": fp, "minute": iso(at) }));
                    }
                }
                json!({ "result": "ok", "sent": true, "count": want })
            }
            "bounce.js" => {
                assert!(!m365, "only Gmail looks in the Inbox");
                assert_eq!(self.places.borrow().last(), Some(&Place::Inbox), "the Inbox is open for the look");
                assert_eq!((args["address"].as_str(), args["nonce"].as_str()), (Some(KNOWLU), Some(NONCE)));
                assert!(args["since"].as_str().is_some_and(|s| s.parse::<jiff::Timestamp>().is_ok()), "{args}");
                assert!(args["wait_ms"].as_u64().is_some_and(|w| w > 0), "{args}");
                let since: jiff::Timestamp = args["since"].as_str().unwrap().parse().unwrap();
                let found = self.report_at.get().is_some_and(|t| t >= since.as_second());
                if self.inbox_unreadable {
                    json!({ "result": "not found", "control": "inbox" })
                } else if found || self.bounce_after.is_some_and(|k| self.forwards.borrow().len() >= k) {
                    self.looked_at.borrow_mut().push(self.clk.now());
                    json!({ "result": "refused", "reason": "non-delivery report" })
                } else {
                    self.looked_at.borrow_mut().push(self.clk.now());
                    json!({ "result": "ok", "rows": 7 })
                }
            }
            "sent_cleanup.js" => {
                assert_eq!(self.places.borrow().last(), Some(&Place::Sent), "Sent is open for the cleanup");
                if args["nonce"] != NONCE {
                    let (nonce, to) = (args["nonce"].as_str().unwrap(), args["address"].as_str().unwrap());
                    let max = args["max"].as_u64().expect("the cleanup names its most") as usize;
                    let mut deleted = 0;
                    self.old_copies.borrow_mut().retain(|(n, t)| {
                        let go = deleted < max && n == nonce && t == to;
                        deleted += usize::from(go);
                        !go
                    });
                    return Ok(json!({ "result": "ok", "deleted": deleted }));
                }
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

/// A loopback service like [`holding`] whose held entries are what `learned` holds when a pass asks: a [`Fake`]
/// whose `learn` is the same list adds each original it sends, so the next pass finds it held.
fn learning(learned: Arc<Mutex<Vec<Value>>>) -> Service {
    Service::start(false, move |r| {
        if r.path.ends_with("/mail/reconcile") {
            let held = learned.lock().unwrap().clone();
            (200, json!({ "address_id": A, "key": FPKEY_B64, "held": held }).to_string())
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
    Job { provider, address_id: A, knowlu_address: KNOWLU, phase, window, cap, watch: None, on_look: None, on_load: None, frontier: None, stuck: None, left: Vec::new(), leftovers: Vec::new(), on_batch: None }
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
    // The calls together are asked for at least the three copies the pass left.
    assert!(mb.calls("sent_cleanup.js").iter().map(|c| c["max"].as_u64().unwrap()).sum::<u64>() >= 3);
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
    // case (one wait for the first row, three a deleted row, two for each of the five skips it allows) can do inside the step's
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
        assert!(wait * (3 * max + 11) < 90_000, "a call that can outlast its step: {c}");
        assert_eq!((c["address"].as_str(), c["nonce"].as_str()), (Some(KNOWLU), Some(NONCE)));
    }
    let asked: u64 = calls.iter().map(|c| c["max"].as_u64().unwrap()).sum();
    assert!(asked <= 10 + 5, "never more than the attempts and five: {asked}");
}

// ---- the batch nonce is kept until its Sent copies are gone (pre-PR review, 2026-10-20, sent-nonce) -----------

use knowlu::mail::backfill::{leftovers_for, note_batch, MAX_SENT_NONCES, SENT_NONCE_DAYS};
use knowlu::mail::recover::SWEEP_ROWS;
use knowlu::mail_session::SentNonce;

/// An earlier pass's batch nonce, and another batch's.
const OLD: &str = "old0batch001";
const OTHER: &str = "other0batch2";
const NOT_MINE: &str = "someone.else@in.example.invalid";

/// A callback that writes each batch note into the fake's own log, so its place among the scripts is seen.
fn note_into(mb: &Fake) -> impl Fn(&str, bool) + '_ {
    move |nonce, open| {
        let n = mb.forwards.borrow().len();
        mb.log.borrow_mut().push((format!("batch {} {nonce}", if open { "open" } else { "close" }), json!({ "forwards": n })));
    }
}

fn batch_notes(mb: &Fake) -> Vec<(String, usize)> {
    mb.log
        .borrow()
        .iter()
        .filter(|(n, _)| n.starts_with("batch "))
        .map(|(n, a)| (n.clone(), a["forwards"].as_u64().unwrap() as usize))
        .collect()
}

fn recover_job(provider: Provider) -> Job<'static> {
    job(provider, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP))
}

#[test]
fn a_pass_records_its_batch_nonce_before_its_first_forward_and_drops_it_once_sent_is_clean() {
    let svc = holding(&[]);
    let mb = fake(Provider::M365, unheld(3));
    let cb = note_into(&mb);
    let out = pass(&mb, &svc, &mut routine(Provider::M365), &Job { on_batch: Some(&cb), ..recover_job(Provider::M365) });
    assert_eq!((out.stop.clone(), out.sent, out.cleaned), (Stop::Done, 3, 3));
    let notes = batch_notes(&mb);
    assert_eq!(notes[0], (format!("batch open {NONCE}"), 0), "kept before the first forward: {notes:?}");
    assert_eq!(notes.len(), 2, "{notes:?}");
    let names = steps(&mb);
    let at = |what: &str| names.iter().position(|n| n == what).unwrap_or_else(|| panic!("no {what} in {names:?}"));
    assert!(at(&format!("batch open {NONCE}")) < at("m365/forward.js"));
    assert!(names.iter().rposition(|n| n == "m365/sent_cleanup.js").unwrap() < at(&format!("batch close {NONCE}")), "{names:?}");

    // A cleanup that did not complete keeps the nonce: the next pass sweeps it.
    let mut broken = fake(Provider::M365, unheld(3));
    broken.cleanup_fails = true;
    let cb = note_into(&broken);
    let out = pass(&broken, &holding(&[]), &mut routine(Provider::M365), &Job { on_batch: Some(&cb), ..recover_job(Provider::M365) });
    assert_eq!(out.sent, 3);
    assert_eq!(batch_notes(&broken), [(format!("batch open {NONCE}"), 0)], "no close after a cleanup that failed");

    // A batch that sent nothing has no copies: it is dropped at once.
    let msgs = unheld(2);
    let held: Vec<(String, String, i64)> = msgs.iter().map(|m| (m.sender.clone(), m.subject.clone(), m.at)).collect();
    let held: Vec<(&str, &str, i64)> = held.iter().map(|(a, b, c)| (a.as_str(), b.as_str(), *c)).collect();
    let quiet = fake(Provider::M365, msgs);
    let cb = note_into(&quiet);
    let j = job(Provider::M365, Phase::Backfill, backfill_window(NOW.div_euclid(DAY) - 5, NOW), None);
    pass(&quiet, &holding(&held), &mut routine(Provider::M365), &Job { on_batch: Some(&cb), ..j });
    assert_eq!(batch_notes(&quiet), [(format!("batch open {NONCE}"), 0), (format!("batch close {NONCE}"), 0)]);
}

#[test]
fn a_leftover_nonce_is_swept_from_sent_before_the_next_batch_opens_and_nothing_else_is_deleted() {
    let mb = fake(Provider::M365, unheld(3));
    *mb.old_copies.borrow_mut() = vec![
        (OLD.into(), KNOWLU.into()),
        (OLD.into(), KNOWLU.into()),
        (OLD.into(), KNOWLU.into()),
        (OLD.into(), NOT_MINE.into()),
        (OTHER.into(), KNOWLU.into()),
    ];
    let cb = note_into(&mb);
    let j = Job { leftovers: vec![OLD.to_string()], on_batch: Some(&cb), ..recover_job(Provider::M365) };
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::M365), &j);
    assert_eq!((out.stop.clone(), out.sent, out.cleaned), (Stop::Done, 3, 6), "the sweep's three and the pass's own three");
    let first = &mb.calls("sent_cleanup.js")[0];
    assert_eq!((first["nonce"].as_str(), first["address"].as_str()), (Some(OLD), Some(KNOWLU)));
    let names = steps(&mb);
    let at = |what: &str| names.iter().position(|n| n == what).unwrap_or_else(|| panic!("no {what} in {names:?}"));
    assert!(at("open Sent") < at("m365/sent_cleanup.js") && at("m365/sent_cleanup.js") < at("m365/forward.js"), "{names:?}");
    assert!(at("open Sent") < at("open Inbox"), "swept before the pass looks at the Inbox: {names:?}");
    assert!(at(&format!("batch close {OLD}")) < at(&format!("batch open {NONCE}")), "{names:?}");
    let left = mb.old_copies.borrow().clone();
    assert_eq!(left, [(OLD.to_string(), NOT_MINE.to_string()), (OTHER.to_string(), KNOWLU.to_string())], "only our nonce to our address went");
    // The sweep names no other nonce: every call before the pass's own is for OLD.
    let before: Vec<_> = mb.calls("sent_cleanup.js").into_iter().take_while(|c| c["nonce"] != NONCE).collect();
    assert!(!before.is_empty() && before.iter().all(|c| c["nonce"] == OLD));
}

#[test]
fn the_sweep_is_bounded_and_a_sweep_that_cannot_run_keeps_its_nonce() {
    // 200 copies: no more than SWEEP_ROWS go in one pass, and the nonce is kept for the next.
    let mb = fake(Provider::M365, unheld(3));
    *mb.old_copies.borrow_mut() = (0..200).map(|_| (OLD.to_string(), KNOWLU.to_string())).collect();
    let cb = note_into(&mb);
    let j = Job { leftovers: vec![OLD.to_string()], on_batch: Some(&cb), ..recover_job(Provider::M365) };
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::M365), &j);
    assert_eq!((out.stop.clone(), out.sent), (Stop::Done, 3), "the pass goes on");
    assert_eq!(200 - mb.old_copies.borrow().len(), SWEEP_ROWS as usize);
    let notes = batch_notes(&mb);
    assert!(!notes.iter().any(|(n, _)| n == &format!("batch close {OLD}")), "{notes:?}");
    assert!(notes.iter().any(|(n, _)| n == &format!("batch close {NONCE}")), "{notes:?}");

    // A Sent that will not open keeps the entry, and the pass still reaches Done.
    let mb = fake(Provider::M365, unheld(3));
    mb.sent_opens_fail.set(1);
    *mb.old_copies.borrow_mut() = (0..3).map(|_| (OLD.to_string(), KNOWLU.to_string())).collect();
    let cb = note_into(&mb);
    let j = Job { leftovers: vec![OLD.to_string()], on_batch: Some(&cb), ..recover_job(Provider::M365) };
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::M365), &j);
    assert_eq!((out.stop.clone(), out.sent, out.cleaned), (Stop::Done, 3, 3));
    assert_eq!(mb.old_copies.borrow().len(), 3);
    assert!(!batch_notes(&mb).iter().any(|(n, _)| n == &format!("batch close {OLD}")));

    // Gmail with too little page room (the sweep's open and the pass's three): no sweep, the entry kept. With room it goes.
    let gmail = |loads: u32| {
        let mb = fake(Provider::Gmail, unheld(3));
        *mb.old_copies.borrow_mut() = (0..3).map(|_| (OLD.to_string(), KNOWLU.to_string())).collect();
        let cb = note_into(&mb);
        let mut r = routine(Provider::Gmail).with_loads(loads);
        let j = Job { leftovers: vec![OLD.to_string()], on_batch: Some(&cb), ..gmail_backfill() };
        pass(&mb, &holding(&[]), &mut r, &j);
        let closed = batch_notes(&mb).iter().any(|(n, _)| n == &format!("batch close {OLD}"));
        let swept = mb.calls("sent_cleanup.js").iter().any(|c| c["nonce"] == OLD);
        let left = mb.old_copies.borrow().len();
        (closed, swept, left)
    };
    let tight = knowlu::mail_steps::MAX_PAGE_LOADS_PER_DAY - 7;
    assert_eq!(gmail(tight), (false, false, 3), "three loads of room is the pass's own and no more");
    assert_eq!(gmail(tight - 1), (true, true, 0), "one more load of room and the sweep runs");
}

#[test]
fn a_mail_json_from_before_sent_nonces_loads_with_none_and_writes_none_until_one_opens() {
    let dir = temp("sent-nonces-old-file");
    let old = r#"{"keep_session":true,"last_reconcile":{"0b0e5b2c-1111-4aaa-8bbb-000000000001":"2026-10-02T10:00:40Z"},"pending_deletes":[],"turned_off":[]}"#;
    std::fs::write(dir.join("mail.json"), old).unwrap();
    let s = MailState::load(&dir).expect("an older file loads");
    assert!(s.sent_nonces.is_empty() && leftovers_for(&s, A, NOW).is_empty());
    MailState::update(&dir, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    assert!(!std::fs::read_to_string(dir.join("mail.json")).unwrap().contains("sent_nonces"));

    // Opening a batch writes the nonce and its instant, in `dumps_value`'s bytes, and nothing that names the address.
    note_batch(&dir, A, "n1", true, NOW);
    let text = std::fs::read_to_string(dir.join("mail.json")).unwrap();
    let want = knowlu_engine::ledger::dumps_value(&json!({ "sent_nonces": { A: [{ "nonce": "n1", "opened": NOW }] } }));
    assert!(text.contains(&want[1..want.len() - 1]), "{text}");
    assert!(!text.contains(KNOWLU));
    assert_eq!(leftovers_for(&MailState::load(&dir).unwrap(), A, NOW + 60), ["n1"]);
    assert!(!format!("{:?}", MailState::load(&dir).unwrap()).contains("n1"), "Debug never shows a nonce");

    // Closing removes it, and the key with it.
    note_batch(&dir, A, "n1", false, NOW + 5);
    assert!(!std::fs::read_to_string(dir.join("mail.json")).unwrap().contains("sent_nonces"));

    // At most MAX_SENT_NONCES a mailbox, the oldest dropped; another mailbox is not counted.
    let n = MAX_SENT_NONCES + 2;
    for i in 0..n {
        note_batch(&dir, A, &format!("a{i}"), true, NOW + i as i64);
    }
    note_batch(&dir, B, "b0", true, NOW);
    let st = MailState::load(&dir).unwrap();
    let kept: Vec<_> = st.sent_nonces[A].iter().map(|e| e.nonce.clone()).collect();
    assert_eq!(kept, (2..n).map(|i| format!("a{i}")).collect::<Vec<_>>());
    assert_eq!(st.sent_nonces[B].len(), 1);

    // Every write prunes the entries older than SENT_NONCE_DAYS, in every mailbox; a close of a nonce not kept writes nothing.
    let later = NOW + SENT_NONCE_DAYS * DAY + 100;
    note_batch(&dir, B, "b1", true, later);
    let st = MailState::load(&dir).unwrap();
    assert!(!st.sent_nonces.contains_key(A), "A's are past the age: {:?}", st.sent_nonces);
    assert_eq!(st.sent_nonces[B], [SentNonce { nonce: "b1".into(), opened: later }]);
    let before = std::fs::read_to_string(dir.join("mail.json")).unwrap();
    note_batch(&dir, B, "never-kept", false, later);
    assert_eq!(std::fs::read_to_string(dir.join("mail.json")).unwrap(), before);

    // What a pass is handed: young entries whose nonce is 1 to 64 letters or digits.
    let mut s = MailState::default();
    let entry = |nonce: &str, opened: i64| SentNonce { nonce: nonce.into(), opened };
    s.sent_nonces.insert(
        A.to_string(),
        vec![entry("good1", NOW), entry("", NOW), entry("has space", NOW), entry(&"x".repeat(65), NOW), entry("old", NOW - SENT_NONCE_DAYS * DAY - 1), entry(&"y".repeat(64), NOW)],
    );
    assert_eq!(leftovers_for(&s, A, NOW), ["good1".to_string(), "y".repeat(64)]);
    assert!(leftovers_for(&s, B, NOW).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
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
fn a_microsoft_conversation_row_is_opened_and_every_message_in_it_is_judged() {
    // T18c (2026-10-07, live): Outlook's conversation view. `list.js` opens a conversation row and reads each
    // message in it (Quinn's ruling for Gmail threads, 2026-10-02); the routine judges each one: the
    // student's own reply never, a later message under its "Re: " subject too, a missing one forwarded alone,
    // asked for as a message inside a conversation. A conversation the page could not read whole is left,
    // never a failed pass. `handled` stays in the search's unit: a conversation is one.
    let since = window(NOW, None, None).since;
    let t = |h: i64| since + h * 3_600;
    let single = mail(BO, "Lab report due", t(2));
    let mut talk = mail(ANA, "Group project", t(10));
    talk.convo = vec![(ANA.into(), t(4)), (ME.into(), t(6)), (BO.into(), t(8)), (ANA.into(), t(10))];
    let mut broken = mail(ANA, "Seminar room", t(20));
    broken.convo = vec![(ANA.into(), t(19)), (BO.into(), t(20))];
    broken.unreadable = true;
    let svc = holding(&[(ANA, "Group project", t(4)), (BO, "Re: Group project", t(8))]);
    let mb = fake(Provider::M365, vec![broken, talk, single]);
    let out = pass(&mb, &svc, &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!(out.stop, Stop::Done);
    assert_eq!((out.expected, out.handled, out.held, out.sent, out.left), (3, 3, 2, 2, 1));
    let sends: Vec<Value> = mb.forwards.borrow().iter().map(|(_, a)| a.clone()).collect();
    assert_eq!(mb.forwarded(), [received(t(2)), received(t(10))], "oldest first; never the student's own reply");
    assert!(sends[0].get("in_conversation").is_none(), "a single row is asked for as before: {}", sends[0]);
    assert_eq!((sends[1]["sender"].as_str(), sends[1]["in_conversation"].as_bool()), (Some(ANA), Some(true)));
}

#[test]
fn a_microsoft_conversation_found_in_two_spans_is_judged_once() {
    // A conversation's search finds it by any message in it, so a window narrowed into spans may list it in
    // two of them: each message in it is judged, and a missing one forwarded, once a pass.
    let w = backfill_window(NOW.div_euclid(DAY) - 5, NOW);
    let day = |d: i64, k: i64| (NOW.div_euclid(DAY) - 4 + d) * DAY + 3_600 * (k + 1);
    let mut msgs: Vec<Mail> = (1..3).flat_map(|d| (0..2).map(move |k| (d, k))).map(|(d, k)| mail(BO, &format!("Day {d} mail {k}"), day(d, k))).collect();
    let mut talk = mail(ANA, "Group project", day(3, 0));
    talk.convo = vec![(ANA.into(), day(0, 0)), (ANA.into(), day(3, 0))];
    msgs.push(talk);
    let mut mb = fake(Provider::M365, msgs);
    mb.drawn = 2;
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::M365), &job(Provider::M365, Phase::Backfill, w, None));
    assert_eq!((out.stop.clone(), out.sent, out.left), (Stop::Done, 6, 0));
    let mut sent = mb.forwarded();
    let all = sent.len();
    sent.sort();
    sent.dedup();
    assert_eq!(sent.len(), all, "nothing forwarded twice: {:?}", mb.forwarded());
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
    // conversation is left, once, never retried and never counted as sent (once a send in the pass has shown the
    // page works: review of fd702e6).
    let mut thin = fake(Provider::Gmail, vec![one.clone(), single.clone()]);
    thin.mismatch = true;
    let out = pass(&thin, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!((out.stop.clone(), out.sent, out.left, out.handled), (Stop::Done, 1, 1, 2));
    assert_eq!(thin.forwards.borrow().len(), 2, "one attempt each");
    // Alone in its pass it is left too (review round 1 of e7ae803): a chip that drew shows the page works, so the
    // shortfall is the conversation's (G1), never a stall.
    let mut alone = fake(Provider::Gmail, vec![one.clone()]);
    alone.mismatch = true;
    let out = pass(&alone, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!((out.stop.clone(), out.sent, out.left), (Stop::Done, 0, 1));
    assert_eq!(alone.forwards.borrow().len(), 1, "one attempt");
}

#[test]
fn missing_gmail_conversations_side_by_side_at_the_walks_end_are_each_left_and_the_backfill_is_done() {
    // Review round 1 of e7ae803: under G1 every missing conversation answers "attachments do not match the rows"
    // with one chip attached. Two such lone rows with no send between them stopped the pass as a stall that no day
    // count reached, so the backfill stopped at the first of them on every launch. Each is the row's own, and left.
    for single in [false, true] {
        let mut msgs = convos(3, 2);
        if single {
            msgs.insert(0, mail(BO, "Lab report due", window(NOW, None, None).since + 60));
        }
        let mut mb = fake(Provider::Gmail, msgs);
        mb.mismatch = true;
        let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
        let sent = u32::from(single);
        assert_eq!((out.stop.clone(), out.sent, out.left, out.handled), (Stop::Done, sent, 3, 3 + sent), "single: {single}");
        assert_eq!(mb.forwards.borrow().len(), 3 + sent as usize, "single: {single}: one attempt each");
    }
}

#[test]
fn a_conversation_left_at_once_gives_its_originals_back_to_the_day() {
    // Review round 2 of e7ae803, #5: a conversation took its messages from the day's 1,500 originals before its send,
    // and under G1 it is always left, so 200 four-message conversations used 800 of the day on rows that sent
    // nothing. A row left with nothing sent gives back what it took; what went still counts.
    for single in [false, true] {
        let mut msgs = convos(3, 4);
        if single {
            msgs.insert(0, mail(BO, "Lab report due", window(NOW, None, None).since + 60));
        }
        let mut mb = fake(Provider::Gmail, msgs);
        mb.mismatch = true;
        let mut r = routine(Provider::Gmail);
        let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
        assert_eq!((out.stop.clone(), out.sent, out.left), (Stop::Done, u32::from(single), 3), "single: {single}");
        assert_eq!(r.sent_today(), out.sent, "single: {single}: only what went counts for the day");
    }
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
fn a_gmail_count_walk_cut_short_is_aimed_at_the_first_day_and_its_rows_are_not_judged_twice() {
    // Live 2026-10-19: a search has no Oldest button, so search.js walks "Next results" inside its budget. Ten
    // single messages, two to a page (five pages), and time for one page: the count is a lower bound and the page
    // is not the oldest. The design of 2026-10-19 never walks on from there: a fresh search that does not walk,
    // page 1 read, "older" pressed to the oldest page (none is older than the first day), then newer.
    let since = window(NOW, None, None).since;
    let msgs: Vec<Mail> = (0..10).map(|i| mail(ANA, &format!("Note {i}"), since + 3_600 * (i + 1))).collect();
    let svc = holding(&[]);
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 2;
    mb.oldest_walk = Some(1);
    let out = pass(&mb, &svc, &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Backfill, backfill_window(NOW.div_euclid(DAY) - 5, NOW), None));
    assert_eq!(out.stop, Stop::Done);
    assert_eq!(gos(&mb), ["read", "older", "older", "older", "older", "older", "newer", "newer", "newer", "newer"]);
    let searches = mb.calls("search.js");
    assert_eq!((searches.len(), &searches[1]["walk"]), (2, &json!(false)));
    // Every message forwarded once, oldest first: the pages the seek passed were never judged.
    let sent = mb.forwarded();
    let want: Vec<String> = (0..10).map(|i| received(since + 3_600 * (i + 1))).collect();
    assert_eq!(sent, want);
    // The batch's expected is the cut walk's lower bound (two pages counted), never a guess.
    assert_eq!((out.expected, out.handled, out.sent), (4, 10, 10));
}

#[test]
fn a_seek_longer_than_one_calls_presses_goes_on_across_calls_and_ends_only_at_the_days_page_limit() {
    // Review of c8806b3: a seek toward the row a walk goes on from that needed more than one call's 40 presses
    // ended the pass (and the backfill) by name. It goes on across calls instead, each press charged: 43 pages of
    // one row and no time to count them take 42 presses to the oldest, and every row goes once, oldest first.
    let since = window(NOW, None, None).since;
    let msgs: Vec<Mail> = (0..43).map(|i| mail(ANA, &format!("Note {i}"), since + 3_600 * (i + 1))).collect();
    let want: Vec<String> = msgs.iter().map(|m| received(m.at)).collect();
    let mut mb = fake(Provider::Gmail, msgs.clone());
    mb.size = 1;
    mb.oldest_walk = Some(0);
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent), (Stop::Done, 43));
    assert_eq!(mb.forwarded(), want);
    // The first seek, from page 1 to the oldest page: 42 presses, and one more call that finds the pager's end (no
    // row is older than the window's first day). A look's restart seeks again later.
    assert_eq!(gos(&mb).iter().skip(1).take_while(|g| *g == "older").count(), 43, "{:?}", gos(&mb));
    assert_eq!(u64::from(r.loads_today()), mb.loads());
    // With the day's pages nearly spent the seek ends as a page limit, never a stop by name, and nothing is sent.
    let mut short = fake(Provider::Gmail, msgs);
    short.size = 1;
    short.oldest_walk = Some(0);
    let mut r = routine(Provider::Gmail).with_loads(570);
    let out = pass(&short, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent), (Stop::PageLimit { next_day_at: (NOW.div_euclid(DAY) + 1) * DAY }, 0));
    assert!(r.loads_today() <= 600, "{}", r.loads_today());
}

#[test]
fn a_3000_row_window_mostly_held_is_walked_to_its_end_past_a_seek_of_more_than_40_pages() {
    // Review of c8806b3: over held mail a narrowed search's span doubles at each re-aim, and a seek past one call's
    // 40 presses ended the backfill by name. Spread evenly over 120 days, 3,000 rows keep every seek under 40 pages
    // (measured 2026-10-19); packed into the window's last 60 days (a quiet first half), the re-aims cross the empty
    // days at a doubling span, and the whole search's seek from its newest page back to day 64 is about 62 pages.
    // The seek goes on across calls; the pass reaches the end, sends what is missing and stays inside the day's pages.
    let first = NOW.div_euclid(DAY) - 120;
    let msgs: Vec<Mail> = (0..3_000).map(|i| mail(ANA, &format!("Notice {i}"), (first + 60) * DAY + 3_600 + i * 1_700)).collect();
    let missing: Vec<usize> = vec![1_500, 2_996, 2_997, 2_998, 2_999];
    let held: Vec<(&str, &str, i64)> = msgs
        .iter()
        .enumerate()
        .filter(|(i, _)| !missing.contains(i))
        .map(|(_, m)| (m.sender.as_str(), m.subject.as_str(), m.at))
        .collect();
    let want: Vec<String> = missing.iter().map(|&i| received(msgs[i].at)).collect();
    let mut mb = fake(Provider::Gmail, msgs.clone());
    mb.size = 45;
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &holding(&held), &mut r, &job(Provider::Gmail, Phase::Backfill, backfill_window(first, NOW), None));
    assert_eq!((out.stop, out.sent, out.held), (Stop::Done, 5, 2_995));
    assert_eq!(mb.forwarded(), want);
    assert!(r.loads_today() <= 600 && u64::from(r.loads_today()) == mb.loads(), "{} vs {}", r.loads_today(), mb.loads());
}

#[test]
fn a_gmail_attachment_that_does_not_upload_stops_the_pass_with_a_fixed_word() {
    // Live 2026-10-19: Gmail's chip reads "Uploading of attachment: ... failed"; forward.js discards the
    // draft and says "attachment failed". It is the page's, not the row's: the pass stops and says so.
    let since = window(NOW, None, None).since;
    let svc = holding(&[]);
    let mut mb = fake(Provider::Gmail, vec![mail(ANA, "Quiz 3 moved", since + 3_600), mail(BO, "Lab report due", since + 7_200)]);
    mb.attach_fails = Some("attachment failed");
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &svc, &mut r, &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!(out.stop, Stop::Script("an attachment did not upload"));
    assert_eq!((out.sent, out.left), (0, 0));
    // Pre-PR review 2026-10-20: the draft was discarded before Send, so the originals taken for it go back to the day.
    assert_eq!((r.sent_today(), r.messages_today()), (0, 0));
    assert_eq!(mb.forwards.borrow().len(), 1, "one forward was tried, and none after it");
}

#[test]
fn a_gmail_group_with_a_chip_still_uploading_is_halved_and_the_lone_row_is_left() {
    // Live (ef24, 2026-10-19): a chip still uploading when upload_ms runs out is told apart from one that
    // failed. Controller ruling: one huge message must never stop every pass at the same place, so a group is
    // halved like a mismatch until the row is alone, and the lone row is left. Nothing is sent for it.
    let since = window(NOW, None, None).since;
    let subjects = ["Quiz 3 moved", "Lab report due", "Huge scan", "Reading list", "Office hours", "Exam room"];
    let msgs = subjects.iter().enumerate().map(|(i, s)| mail(ANA, s, since + 600 * (i as i64 + 1))).collect();
    let mut mb = fake(Provider::Gmail, msgs);
    mb.slow_chip = Some("Huge scan");
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!((out.stop, out.sent, out.left, out.handled), (Stop::Done, 5, 1, 6));
    let tried = |subject: &str| mb.forwards.borrow().iter().flat_map(|(_, a)| a["rows"].as_array().unwrap().clone()).filter(|r| r["subject"] == subject).count();
    // Review of fd702e6: alone, a chip still uploading gets one more try with a longer upload bound.
    assert_eq!(tried("Huge scan"), 5, "6, then 3, then 2, then alone, then alone with longer to upload");
    assert_eq!(tried("Quiz 3 moved"), 3, "tried in the 6, the 3 and then alone, where it went");
}

#[test]
fn a_lone_row_still_uploading_gets_one_longer_try_and_a_slow_but_real_upload_goes() {
    // Review of fd702e6: a big message whose upload takes 45 s is not a row to leave. forward.js waits 30 s by
    // default; the routine gives the lone row one more try with the longest upload that fits a step (50 s).
    let since = window(NOW, None, None).since;
    let msgs = vec![mail(ANA, "Quiz 3 moved", since + 600), mail(ANA, "Huge scan", since + 1_200), mail(BO, "Lab report due", since + 1_800)];
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 1;
    mb.slow_chip = Some("Huge scan");
    mb.slow_chip_ms = Some(45_000);
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!((out.stop, out.sent, out.left, out.handled), (Stop::Done, 3, 0, 3));
    let huge: Vec<Value> = mb.forwards.borrow().iter().map(|(_, a)| a.clone()).filter(|a| a["rows"][0]["subject"] == "Huge scan").collect();
    assert_eq!(huge.len(), 2, "the first try, then one longer one");
    assert!(huge[0].get("upload_ms").is_none(), "the first try keeps forward.js's own bound");
    assert_eq!(huge[1]["upload_ms"], 50_000);
    // The other rows never get the longer bound.
    assert_eq!(mb.forwards.borrow().iter().filter(|(_, a)| a.get("upload_ms").is_some()).count(), 1);
}

#[test]
fn a_gmail_wide_upload_stall_stops_the_pass_early_and_leaves_nothing() {
    // Review of fd702e6: when no chip uploads anywhere, halving to singles and leaving each would leave every row
    // (and a backfill row older than recover's window would be lost). A lone failure is left only when a send in
    // this pass went since the lone failure before it; a second with nothing sent between stops the pass, so the
    // next pass tries again.
    let mut mb = fake(Provider::Gmail, unheld(30));
    mb.stall = true;
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!(out.stop, Stop::Script("the attachments would not attach"));
    assert_eq!((out.sent, out.left, out.handled), (0, 0, 0), "nothing left, nothing counted handled");
    let sizes: Vec<usize> = mb.forwards.borrow().iter().map(|(_, a)| a["rows"].as_array().unwrap().len()).collect();
    // Ten, five, two, one and its longer try (waiting for a send), then the other one and its longer try: stop.
    assert_eq!(sizes, [10, 5, 2, 1, 1, 1, 1]);
}

#[test]
fn a_lone_failure_before_any_send_is_left_once_a_later_send_goes_and_alone_it_stops_the_pass() {
    let since = window(NOW, None, None).since;
    let recover = || job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP));
    // The bad row is the pass's first send: it waits, and the send after it proves the page works, so it is left.
    let msgs = vec![mail(ANA, "Bad chip", since + 600), mail(ANA, "Quiz 3 moved", since + 1_200), mail(BO, "Lab report due", since + 1_800)];
    let mut first = fake(Provider::Gmail, msgs);
    first.size = 1;
    first.no_chip = Some("Bad chip");
    let out = pass(&first, &holding(&[]), &mut routine(Provider::Gmail), &recover());
    assert_eq!((out.stop, out.sent, out.left, out.handled), (Stop::Done, 2, 1, 3));
    assert_eq!(first.forwards.borrow().len(), 3, "each row tried once");
    // Alone in the pass, nothing proves the page: the pass stops naming it, and nothing is left.
    let mut alone = fake(Provider::Gmail, vec![mail(ANA, "Bad chip", since + 600)]);
    alone.no_chip = Some("Bad chip");
    let out = pass(&alone, &holding(&[]), &mut routine(Provider::Gmail), &recover());
    assert_eq!((out.stop, out.sent, out.left), (Stop::Script("the attachments would not attach"), 0, 0));
}

#[test]
fn a_lone_gmail_row_whose_chip_is_still_uploading_or_never_draws_is_left_and_the_pass_goes_on() {
    // A page of one row each, so every send is a lone row: the second never attaches, and the first and third go.
    let since = window(NOW, None, None).since;
    for (slow, reason) in [(true, "still uploading"), (false, "never draws")] {
        let msgs = vec![mail(ANA, "Quiz 3 moved", since + 600), mail(ANA, "Huge scan", since + 1_200), mail(BO, "Lab report due", since + 1_800)];
        let mut mb = fake(Provider::Gmail, msgs);
        mb.size = 1;
        if slow {
            mb.slow_chip = Some("Huge scan");
        } else {
            mb.no_chip = Some("Huge scan");
        }
        let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
        assert_eq!((out.stop.clone(), out.sent, out.left, out.handled), (Stop::Done, 2, 1, 3), "{reason}");
        // Review of fd702e6: a chip still uploading gets one longer try alone; a chip that never draws does not.
        let tries = if slow { 4 } else { 3 };
        assert_eq!(mb.forwards.borrow().len(), tries, "{reason}: each row tried once, a slow one twice");
    }
}

#[test]
fn no_attachment_on_a_group_or_a_conversation_and_a_compose_left_open_still_stop_the_pass() {
    let since = window(NOW, None, None).since;
    let two = || vec![mail(ANA, "Quiz 3 moved", since + 600), mail(BO, "Lab report due", since + 1_200)];
    let recover = || job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP));
    // Several rows and no attachment at all is an empty compose: not a row's fault.
    let mut group = fake(Provider::Gmail, two());
    group.attach_fails = Some("no attachment");
    let out = pass(&group, &holding(&[]), &mut routine(Provider::Gmail), &recover());
    assert_eq!((out.stop, out.left), (Stop::Script("a forward did not finish"), 0));
    assert_eq!(group.forwards.borrow().len(), 1);
    // Review of fd702e6: a conversation is one row, and alone with no chip it is a lone row like a single message:
    // left when a send went before it, never a stop on every pass at the same place.
    let mut convo = mail(ANA, "Group project", since + 1_800);
    convo.count = Some(2);
    convo.convo = vec![(ANA.to_string(), since + 1_000), (ANA.to_string(), since + 1_800)];
    let mut thread = fake(Provider::Gmail, vec![mail(BO, "Lab report due", since + 600), convo]);
    thread.no_chip = Some("Group project");
    let out = pass(&thread, &holding(&[]), &mut routine(Provider::Gmail), &recover());
    assert_eq!((out.stop, out.sent, out.left, out.handled), (Stop::Done, 1, 1, 2));
    assert_eq!(thread.forwards.borrow().len(), 2, "the conversation was tried once");
    // A compose still open from an earlier try is the page's state: the pass stops naming it.
    let mut open = fake(Provider::Gmail, two());
    open.attach_fails = Some("compose still open");
    let mut r = routine(Provider::Gmail);
    let out = pass(&open, &holding(&[]), &mut r, &recover());
    assert_eq!((out.stop, out.left), (Stop::Script("a draft was still open"), 0));
    // Pre-PR review 2026-10-20: nothing went, so the day is charged nothing.
    assert_eq!((r.sent_today(), r.messages_today()), (0, 0));
    assert_eq!(open.forwards.borrow().len(), 1, "no second try over it");
}

// ---- Gmail: the Inbox look for a non-delivery report (Quinn's ruling 2026-10-19, option b) ---------------
//
// Spec 4.2 step 5 and risk 15 stop at the first non-delivery report, and a bounce lands in the Inbox, which a
// search page does not list. The Gmail pass opens the Inbox every 25 sends and once more at its end, and the
// walk it interrupted starts again with the rows it has decided skipped.

fn gmail_backfill() -> Job<'static> {
    job(Provider::Gmail, Phase::Backfill, backfill_window(NOW.div_euclid(DAY) - 5, NOW), None)
}

/// Each `list.js` call's direction, in order; "read" for a read of the page as it is.
fn gos(mb: &Fake) -> Vec<String> {
    mb.calls("list.js").iter().map(|a| a["go"].as_str().unwrap_or("read").to_string()).collect()
}

/// The scripts and places the routine used, in order ("open Inbox" is a place).
fn steps(mb: &Fake) -> Vec<String> {
    mb.log.borrow().iter().map(|(n, _)| n.clone()).collect()
}

/// How many originals had been forwarded when each look at the Inbox ran.
fn sent_at_each_look(mb: &Fake) -> Vec<usize> {
    let (mut sent, mut looks) = (0, Vec::new());
    for (name, args) in mb.log.borrow().iter() {
        if name.ends_with("forward.js") {
            // A conversation row is as many attachments as it has messages.
            sent += args["rows"].as_array().unwrap().iter().map(|r| r["messages"].as_u64().unwrap_or(1) as usize).sum::<usize>();
        } else if name.ends_with("bounce.js") {
            looks.push(sent);
        }
    }
    looks
}

#[test]
fn a_gmail_backfill_looks_in_the_inbox_every_25_sends_and_walks_again_without_sending_twice() {
    let since = window(NOW, None, None).since;
    let svc = holding(&[]);
    let mut mb = fake(Provider::Gmail, unheld(60));
    mb.size = 20;
    let out = pass(&mb, &svc, &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!(out.stop, Stop::Done);
    assert_eq!((out.expected, out.handled, out.held, out.sent, out.left), (60, 60, 0, 60, 0));
    // Every row went once, oldest first, whatever the walks that read it.
    let want: Vec<String> = (0..60).map(|i| received(since + 3_600 + i * 600)).collect();
    assert_eq!(mb.forwarded(), want);
    // The first send to bring the count to 25 or more (the third group of ten, 30 sent) is looked in after, in
    // the middle of the second page, not at its end (40); the next such send is the last (60).
    assert_eq!(sent_at_each_look(&mb), [30, 60]);
    // A fresh Inbox for the search, the Inbox for the first look, a fresh one for the restart, the second look, Sent.
    assert_eq!(*mb.places.borrow(), [Place::FreshInbox, Place::Inbox, Place::FreshInbox, Place::Inbox, Place::Sent]);
    let names = steps(&mb);
    let first = names.iter().position(|n| n.ends_with("bounce.js")).unwrap();
    // The look opens the Inbox in the document it is in; the walk that follows starts from a fresh one, before
    // its search (live 2026-10-19: a cached search view is restored in the old document and walks off its end).
    assert_eq!(
        (names[first - 1].as_str(), names[first + 1].as_str(), names[first + 2].as_str()),
        ("open Inbox", "open FreshInbox", "gmail/search.js")
    );
    // The look is told the batch it is about: the address, the nonce, and how far back to read. The first look
    // of a mailbox with no saved look reaches back a day before the batch opened (slots are up to about 13 hours
    // apart); the next starts at the look before it, so a report is counted once.
    let looks = mb.calls("bounce.js");
    assert_eq!(looks[0]["since"], json!(iso(NOW - 24 * 3_600)));
    assert_eq!(looks[1]["since"], json!(iso(mb.looked_at.borrow()[0])));
    // The restart searches again from the same day, never walking its count (the design of 2026-10-19: no walk to
    // the oldest page and back), reads page 1 and presses "older" until the page's oldest row is older than the
    // row the walk goes on from (the 31st), then walks newer. Its span (6 days past it) reaches past the window's
    // end, so it has no `until`. Page 3 is the newest, and the look after its last send finds every row decided,
    // so no walk follows it. The rows the first walk decided (the first half of page 2) are read again but not
    // judged.
    let searches = mb.calls("search.js");
    assert_eq!(searches.len(), 2);
    assert_eq!(searches[0]["since"], searches[1]["since"]);
    assert_eq!((searches[0].get("walk"), &searches[1]["walk"]), (None, &json!(false)));
    assert!(searches[1].get("until").is_none(), "{}", searches[1]);
    assert_eq!(gos(&mb), ["oldest", "newer", "read", "older", "newer"]);
    assert_eq!(mb.calls("forward.js").len(), 6, "six groups of ten, each sent once");
    // The batch's expected stays the first search's count, and it closes with what was sent.
    let open: Value = serde_json::from_str(&seen(&svc).iter().find(|(_, p, _)| p == "/batches").unwrap().2).unwrap();
    assert_eq!(open["expected"], 60);
    assert!(out.closed && seen(&svc).iter().any(|(_, p, b)| p.ends_with("/sent") && b == r#"{"sent":60}"#));
}

#[test]
fn a_report_found_at_the_look_stops_the_pass_as_paused_with_nothing_sent_after_it() {
    let svc = holding(&[]);
    let mut mb = fake(Provider::Gmail, unheld(60));
    mb.size = 10;
    mb.bounce_after = Some(3);
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &svc, &mut r, &gmail_backfill());
    assert_eq!(out.stop, Stop::Paused(Paused::new(Provider::Gmail)));
    assert_eq!((out.sent, mb.forwards.borrow().len()), (30, 3));
    assert!(r.is_paused(), "nothing sends again until the next UTC day");
    // The look was the pass's last act on the mailbox before the Sent cleanup: no walk, no forward after it.
    let names = steps(&mb);
    let at = names.iter().position(|n| n.ends_with("bounce.js")).unwrap();
    assert!(names[at..].iter().all(|n| !n.ends_with("forward.js") && !n.ends_with("search.js") && !n.ends_with("list.js")), "{names:?}");
    assert_eq!(mb.calls("bounce.js").len(), 1, "a look that found the report is not repeated at the end");
    assert!(out.closed, "the batch is closed with the count sent");
    // And a second pass the same day sends nothing.
    let again = pass(&mb, &svc, &mut r, &gmail_backfill());
    assert_eq!((again.stop, mb.forwards.borrow().len()), (Stop::Paused(Paused::new(Provider::Gmail)), 3));
}

#[test]
fn a_final_look_runs_before_the_batch_closes_and_a_report_there_turns_done_into_paused() {
    // Twelve sends: under 25, so no look between pages; the one at the end is before the Sent cleanup.
    let svc = holding(&[]);
    let mut mb = fake(Provider::Gmail, unheld(12));
    mb.size = 10;
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &svc, &mut r, &gmail_backfill());
    assert_eq!((out.stop.clone(), out.sent, out.closed), (Stop::Done, 12, true));
    assert_eq!(sent_at_each_look(&mb), [12]);
    let names = steps(&mb);
    let at = |n: &str| names.iter().position(|x| x.ends_with(n)).unwrap();
    assert!(at("bounce.js") > names.iter().rposition(|n| n.ends_with("forward.js")).unwrap());
    assert!(at("bounce.js") < at("open Sent") && at("open Sent") < at("sent_cleanup.js"), "{names:?}");
    assert!(!r.is_paused());
    // A report there: the pass ends as Paused, the routine pauses, and the batch is still closed and cleaned.
    let mut found = fake(Provider::Gmail, unheld(12));
    found.size = 10;
    found.bounce_after = Some(2);
    let mut r = routine(Provider::Gmail);
    let out = pass(&found, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop.clone(), out.sent, out.closed), (Stop::Paused(Paused::new(Provider::Gmail)), 12, true));
    assert!(r.is_paused() && out.cleaned > 0, "{out:?}");
    // A look that cannot read the Inbox at the end leaves the pass's stop as it was.
    let mut blind = fake(Provider::Gmail, unheld(12));
    blind.size = 10;
    blind.inbox_unreadable = true;
    let mut r = routine(Provider::Gmail);
    let out = pass(&blind, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop.clone(), out.sent, out.closed), (Stop::Done, 12, true));
    assert!(!r.is_paused() && out.cleaned > 0, "{out:?}");
}

#[test]
fn no_look_is_made_on_a_gmail_pass_that_sent_nothing() {
    let msgs = unheld(30);
    let held: Vec<(&str, &str, i64)> = msgs.iter().map(|m| (m.sender.as_str(), m.subject.as_str(), m.at)).collect();
    let svc = holding(&held);
    let mut mb = fake(Provider::Gmail, msgs.clone());
    mb.size = 10;
    let out = pass(&mb, &svc, &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!((out.stop, out.held, out.sent), (Stop::Done, 30, 0));
    assert!(mb.calls("bounce.js").is_empty());
    assert_eq!(*mb.places.borrow(), [Place::FreshInbox], "the Inbox was opened once, fresh, for the search");
    // Microsoft 365 never looks: it has no scripts for it, and the fake would stop the test.
    let m365 = fake(Provider::M365, unheld(30));
    let out = pass(&m365, &holding(&[]), &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!((out.stop, out.sent), (Stop::Done, 30));
    assert!(m365.calls("bounce.js").is_empty());
}

#[test]
fn an_inbox_that_will_not_read_stops_the_gmail_pass_with_a_fixed_word() {
    let svc = holding(&[]);
    let mut mb = fake(Provider::Gmail, unheld(60));
    mb.size = 10;
    mb.inbox_unreadable = true;
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &svc, &mut r, &gmail_backfill());
    assert_eq!(out.stop, Stop::Script("the inbox could not be read"));
    assert_eq!((out.sent, mb.forwards.borrow().len()), (30, 3), "nothing was sent after the look that failed");
    assert!(!r.is_paused(), "an unreadable Inbox is not a report");
    assert!(out.closed);
}

#[test]
fn a_row_the_pass_left_is_not_tried_again_when_the_walk_starts_over() {
    // A conversation whose attachments do not match is left on the first page. The walk that starts over after
    // the look reads that page again: the conversation is not opened and not forwarded a second time.
    let since = window(NOW, None, None).since;
    let mut convo = mail(ANA, "Group project", since + 1_800);
    convo.count = Some(2);
    convo.convo = vec![(ANA.to_string(), since + 1_000), (ANA.to_string(), since + 1_800)];
    let mut msgs = unheld(34);
    msgs.push(convo);
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 10;
    mb.mismatch = true;
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!(out.stop, Stop::Done);
    assert_eq!((out.expected, out.handled, out.sent, out.left), (35, 35, 34, 1));
    assert_eq!(mb.calls("search.js").len(), 2, "the walk did start over");
    let tried = |subject: &str| mb.forwards.borrow().iter().flat_map(|(_, a)| a["rows"].as_array().unwrap().clone()).filter(|r| r["subject"] == subject).count();
    assert_eq!((tried("Group project"), mb.calls("thread.js").len()), (1, 1));
    let singles = mb.forwarded().into_iter().filter(|t| *t != received(since + 1_800)).count();
    assert_eq!(singles, 34, "each single message went once");
}

#[test]
fn a_group_of_single_rows_one_short_is_halved_until_the_row_that_will_not_attach_is_alone() {
    // Live (ef25, 2026-10-19): six single rows sent together attached five. Leaving all six strands five
    // that would send; the group is halved, as "over 25 MB" is, and only the row that will not attach is left.
    let since = window(NOW, None, None).since;
    let subjects = ["Quiz 3 moved", "Lab report due", "Bad chip", "Reading list", "Office hours", "Exam room"];
    let msgs = subjects.iter().enumerate().map(|(i, s)| mail(ANA, s, since + 600 * (i as i64 + 1))).collect();
    let mut mb = fake(Provider::Gmail, msgs);
    mb.no_chip = Some("Bad chip");
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!((out.stop, out.sent, out.left, out.handled), (Stop::Done, 5, 1, 6));
    let sizes: Vec<usize> = mb.forwards.borrow().iter().map(|(_, a)| a["rows"].as_array().unwrap().len()).collect();
    assert_eq!(sizes[0], 6, "the six went as one group first");
    let tried = |subject: &str| mb.forwards.borrow().iter().flat_map(|(_, a)| a["rows"].as_array().unwrap().clone()).filter(|r| r["subject"] == subject).count();
    assert_eq!(tried("Bad chip"), 4, "6, then 3, then 2, then alone");
}

#[test]
fn two_rows_that_read_alike_on_different_pages_are_both_judged() {
    // The rows a re-walk skips are keyed by what list.js reads (sender, subject, time), and two messages can
    // read the same: both are sent, and the walk that starts over sends neither again.
    let since = window(NOW, None, None).since;
    let mut msgs = unheld(30);
    // "Notice 9" is the tenth row, last on the first page; its twin is first on the second.
    let at = since + 3_600 + 9 * 600;
    msgs.push(mail(ANA, "Notice 9", at));
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 10;
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!((out.stop, out.sent, out.handled, out.expected), (Stop::Done, 31, 31, 31));
    assert_eq!(mb.forwarded().iter().filter(|t| **t == received(at)).count(), 2);
}

/// `n` unheld conversations of `messages` messages each, an hour apart, oldest first.
fn convos(n: usize, messages: i64) -> Vec<Mail> {
    let since = window(NOW, None, None).since;
    (0..n)
        .map(|i| {
            let base = since + 3_600 * (i as i64 + 1);
            let mut m = mail(ANA, &format!("Group {i}"), base + 60 * (messages - 1));
            m.count = Some(messages as u32);
            m.convo = (0..messages).map(|j| (ANA.to_string(), base + 60 * j)).collect();
            m
        })
        .collect()
}

#[test]
fn a_look_follows_the_forward_that_reaches_25_and_a_conversation_adds_each_of_its_messages() {
    // One page of ten conversations of five messages: the fifth conversation's forward brings the count to
    // 25, and the Inbox is looked in then, not when the page ends (50). The walk starts over, skips the five
    // sent, and sends the five it had not reached exactly once.
    let since = window(NOW, None, None).since;
    let mut mb = fake(Provider::Gmail, convos(10, 5));
    mb.size = 10;
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!(out.stop, Stop::Done);
    assert_eq!((out.expected, out.handled, out.sent, out.left), (10, 10, 50, 0));
    assert_eq!(sent_at_each_look(&mb), [25, 50], "a conversation counts each of its messages, and the look is not held to the page's end");
    let want: Vec<String> = (0..10).map(|i| received(since + 3_600 * (i + 1) + 240)).collect();
    assert_eq!(mb.forwarded(), want, "each conversation once, oldest first");
    // The restart reads the one page as its search left it: no oldest walk.
    assert_eq!(gos(&mb), ["oldest", "read"]);
    assert_eq!(mb.calls("search.js").len(), 2);
    assert_eq!(mb.calls("thread.js").len(), 15, "the ten read, then the five that were not sent");
    assert_eq!(out.held, 0);
}

#[test]
fn a_report_that_landed_after_one_passs_last_look_stops_the_next_pass_at_its_first_look() {
    // The first look of a pass reads back to the routine's last look, not to when its own batch opened.
    let mut r = routine(Provider::Gmail);
    let first = {
        let mut f = fake(Provider::Gmail, unheld(12));
        f.size = 10;
        f
    };
    let out = pass(&first, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop.clone(), out.sent), (Stop::Done, 12));
    let last = first.clk.now();
    assert_eq!(r.last_look(), Some(last), "the final look was the last thing that moved the clock");
    // A report lands after it, and the next pass starts a little later.
    first.clk.0.set(last + 120);
    let mut second = fake(Provider::Gmail, unheld(60));
    second.size = 10;
    second.clk = first.clk.clone();
    second.report_at.set(Some(last + 60));
    let out = pass(&second, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!(out.stop, Stop::Paused(Paused::new(Provider::Gmail)));
    assert_eq!(out.sent, 30, "it stopped at its first look");
    assert_eq!(second.calls("bounce.js")[0]["since"], json!(iso(last)));
    assert!(r.is_paused());
}

#[test]
fn a_report_seen_once_does_not_pause_the_routine_again_at_a_later_days_look() {
    let midnight = (NOW.div_euclid(DAY) + 1) * DAY;
    let mut r = routine(Provider::Gmail);
    // Pass 1, an hour before midnight: its last look finds a report, and the routine pauses for the day.
    let mut first = fake(Provider::Gmail, unheld(12));
    first.size = 10;
    first.clk.0.set(midnight - 3_600);
    first.report_at.set(Some(midnight - 3_600));
    let out = pass(&first, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!(out.stop, Stop::Paused(Paused::new(Provider::Gmail)));
    let seen_at = first.clk.now();
    assert_eq!(r.last_look(), Some(seen_at), "a look that found a report is a look");
    // Half an hour into the next day the same report is still in the Inbox, and it is not counted again.
    let mut second = fake(Provider::Gmail, unheld(12));
    second.size = 10;
    second.clk = first.clk.clone();
    second.clk.0.set(midnight + 1_800);
    second.report_at.set(Some(midnight - 3_600));
    let out = pass(&second, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent), (Stop::Done, 12));
    assert!(!r.is_paused());
    assert_eq!(second.calls("bounce.js")[0]["since"], json!(iso(seen_at)));
    // A look that could not read the Inbox is no look: the routine's last look stays where it was.
    let mut blind = fake(Provider::Gmail, unheld(12));
    blind.size = 10;
    blind.clk = first.clk.clone();
    blind.inbox_unreadable = true;
    let again = r.last_look();
    pass(&blind, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!(r.last_look(), again);
}

#[test]
fn a_result_that_never_ends_stops_the_gmail_pass_at_the_days_page_limit() {
    // Each "newer" is a page load: the day's 600 (300 before the day-cap ruling of 2026-10-19) run out long before
    // the 5,000-page backstop, and the pass stops as a page limit, never Done, with the end's own loads kept.
    let mut mb = fake(Provider::Gmail, unheld(1));
    mb.endless = true;
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &holding(&[]), &mut r, &job(Provider::Gmail, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert_eq!(out.stop, Stop::PageLimit { next_day_at: (NOW.div_euclid(DAY) + 1) * DAY });
    assert!(mb.calls("list.js").len() < 600, "{}", mb.calls("list.js").len());
    assert!(r.loads_today() <= 600 && u64::from(r.loads_today()) == mb.loads(), "{} vs {}", r.loads_today(), mb.loads());
}

/// `n` messages from `who`, `per_day` a day from the start of UTC day `first`, oldest first.
fn daily(who: &str, tag: &str, first: i64, n: usize, per_day: i64) -> Vec<Mail> {
    let gap = 20 * 3_600 / per_day;
    (0..n as i64).map(|i| mail(who, &format!("{tag} {i}"), first * DAY + 3_600 + (i / per_day) * DAY + (i % per_day) * gap)).collect()
}

/// Each search's `until`, in order (`None`: the whole search).
fn untils(mb: &Fake) -> Vec<Option<String>> {
    mb.calls("search.js").iter().map(|a| a.get("until").and_then(Value::as_str).map(str::to_string)).collect()
}

#[test]
fn a_gmail_restart_searches_narrowed_from_where_the_walk_goes_on_and_never_walks_to_the_oldest_page() {
    // The design of 2026-10-19: a restart after a look reloads (FreshInbox), searches the same day narrowed with
    // `before:` a few days past the row the walk goes on from, does not walk the count, and presses "older" a page
    // or two to that row. 300 messages over 30 days of a 40-day window, 20 to a page: ten looks.
    let first = NOW.div_euclid(DAY) - 40;
    let msgs = daily(ANA, "Notice", first + 5, 300, 10);
    let want: Vec<String> = msgs.iter().map(|m| received(m.at)).collect();
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 20;
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &holding(&[]), &mut r, &job(Provider::Gmail, Phase::Backfill, backfill_window(first, NOW), None));
    assert_eq!((out.stop, out.handled, out.sent, out.left), (Stop::Done, 300, 300, 0));
    assert_eq!(mb.forwarded(), want, "every row once, oldest first");
    // Only the pass's first search walks its count to the oldest page; no restart's does, and none walks there.
    let searches = mb.calls("search.js");
    assert_eq!(searches.len(), 11, "the first search and one a look");
    assert!(searches[0].get("walk").is_none() && searches[0].get("until").is_none(), "{}", searches[0]);
    assert!(searches[1..].iter().all(|s| s["walk"] == json!(false) && s["since"] == searches[0]["since"]), "{searches:?}");
    assert_eq!(gos(&mb).iter().filter(|g| *g == "oldest").count(), 1);
    // Every restart inside the window is narrowed, and a bound never goes back; once whole, a search stays whole.
    let bounds = untils(&mb);
    assert!(bounds[1].is_some(), "{bounds:?}");
    let narrowed: Vec<&String> = bounds[1..].iter().map_while(Option::as_ref).collect();
    assert!(narrowed.windows(2).all(|w| w[0] <= w[1]), "{bounds:?}");
    assert!(bounds[1 + narrowed.len()..].iter().all(Option::is_none), "{bounds:?}");
    // Each seek passes at most the span's rows (9 days of 10, five pages of 20) where the old restart walked all
    // 15 pages to the oldest and back.
    let go = gos(&mb);
    let seeks: Vec<usize> = go.split(|g| g == "read").skip(1).map(|s| s.iter().take_while(|g| *g == "older").count()).collect();
    assert!(seeks.len() == 10 && seeks.iter().all(|n| *n <= 5), "{go:?}");
    assert_eq!(u64::from(r.loads_today()), mb.loads(), "the routine counted every load the page made");
}

#[test]
fn a_restart_lands_on_a_page_older_than_the_row_it_goes_on_from_so_that_rows_twin_is_still_judged() {
    // Rows 29 and 30 read alike (sender, subject, instant) and fall on two pages of ten. The look after the 30th
    // send restarts the walk from row 29 (every row met is decided: the newest met). The seek lands on the first
    // page whose oldest row is strictly older than it (rows 20-29), not on the one whose oldest row is its twin, so
    // the walk meets row 29 again before the twin, and the twin is judged and sent.
    let since = window(NOW, None, None).since;
    let msgs: Vec<Mail> = (0..60)
        .map(|i| {
            let k = if i == 30 { 29 } else { i };
            mail(ANA, &format!("Notice {k}"), since + 3_600 + k * 600)
        })
        .collect();
    let twin = received(since + 3_600 + 29 * 600);
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 10;
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!((out.stop, out.handled, out.sent), (Stop::Done, 60, 60));
    assert_eq!(mb.forwarded().iter().filter(|t| **t == twin).count(), 2, "both twins went");
    assert_eq!(sent_at_each_look(&mb), [30, 60]);
    // The restart reads its newest page (rows 50-59) and presses older past rows 40-49 and 30-39 to rows 20-29.
    let go = gos(&mb);
    assert_eq!(go[..7], ["oldest", "newer", "newer", "read", "older", "older", "older"], "{go:?}");
}

#[test]
fn a_re_aim_across_a_gap_in_the_mail_longer_than_its_span_moves_its_bound_forward_until_the_search_is_whole() {
    // The judge's loop (2026-10-19): a narrowed search whose newest page is reached is aimed again with a later
    // bound (at least a day past the last, its span doubled). Mail on days 1-5 of a 40-day window, none for 30
    // days, then mail on days 36-39 whose last five are missing: the re-aims cross the gap, and only the whole
    // search's newest page ends the pass.
    let first = NOW.div_euclid(DAY) - 40;
    let a: Vec<Mail> = (0..100).map(|i| mail(ANA, &format!("A {i}"), (first + 1) * DAY + i * 4_320)).collect();
    let b: Vec<Mail> = (0..105).map(|i| mail(BO, &format!("B {i}"), (first + 36) * DAY + i * 3_000)).collect();
    let held: Vec<(&str, &str, i64)> = a[30..].iter().chain(&b[..100]).map(|m| (m.sender.as_str(), m.subject.as_str(), m.at)).collect();
    let svc = holding(&held);
    let mut mb = fake(Provider::Gmail, a.iter().chain(&b).cloned().collect());
    mb.size = 20;
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &svc, &mut r, &job(Provider::Gmail, Phase::Backfill, backfill_window(first, NOW), None));
    assert_eq!((out.stop, out.handled, out.held, out.sent), (Stop::Done, 205, 170, 35));
    let want: Vec<String> = a[..30].iter().chain(&b[100..]).map(|m| received(m.at)).collect();
    assert_eq!(mb.forwarded(), want, "the gap's far side was reached and its missing mail sent");
    // The first search is whole; then the look's restart and each re-aim are narrowed with strictly later bounds,
    // until one reaches past the window's end and the search runs whole again, and its newest page ends the pass.
    let bounds = untils(&mb);
    assert!(bounds.first() == Some(&None) && bounds.last() == Some(&None), "{bounds:?}");
    let narrowed: Vec<&String> = bounds[1..bounds.len() - 1].iter().map(|b| b.as_ref().expect("narrowed")).collect();
    assert!(narrowed.len() >= 2, "{bounds:?}");
    assert!(narrowed.windows(2).all(|w| w[0] < w[1]), "{bounds:?}");
    assert_eq!(u64::from(r.loads_today()), mb.loads());
}

#[test]
fn no_restart_is_made_once_the_days_1500_sends_are_spent() {
    // 1,470 sent already today: the third group of ten brings the day to Gmail's 1,500 (Quinn, 2026-10-19; 600
    // before) and the count since the look to 30. The look runs, but the walk does not start over: no fresh Inbox,
    // no search, nothing listed.
    let mut mb = fake(Provider::Gmail, unheld(60));
    mb.size = 20;
    let mut r = Routine::new(Provider::Gmail, NOW.div_euclid(DAY), 1_470);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent), (Stop::DayLimit { next_day_at: (NOW.div_euclid(DAY) + 1) * DAY }, 30));
    assert_eq!(*mb.places.borrow(), [Place::FreshInbox, Place::Inbox, Place::Sent], "the look, then the batch's end");
    let names = steps(&mb);
    let look = names.iter().position(|n| n.ends_with("bounce.js")).unwrap();
    assert!(names[look..].iter().all(|n| !n.ends_with("search.js") && !n.ends_with("list.js")), "{names:?}");
}

#[test]
fn gmails_300_messages_a_day_stop_a_pass_whose_every_forward_is_a_single_original() {
    // Quinn's day-cap ruling (2026-10-19): one forward is one Gmail message whatever its originals, and the day
    // sends at most 300 (Gmail's consumer limit is about 500 with the student's own mail). Every group of two or
    // more is "over 25 MB", so it is halved down to single rows and each send is one original: the message cap,
    // not the 1,500 originals, ends the pass, as a day limit. A send that did not go counts no message. The limit
    // holds until the first of the 300 is 24 hours old (review round 2 of e7ae803: a rolling day, never UTC midnight).
    let mut mb = fake(Provider::Gmail, unheld(320));
    mb.over_rows = Some(1);
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    let first = mb.forwards.borrow().iter().find(|(_, a)| a["rows"].as_array().unwrap().len() == 1).map(|(t, _)| *t).unwrap();
    assert_eq!((out.stop, out.sent), (Stop::DayLimit { next_day_at: first + DAY }, 300));
    assert_eq!(r.messages_today(), 300);
    assert!(r.sent_today() < 1_500, "{}", r.sent_today());
    let fwd = mb.forwards.borrow();
    let went = fwd.iter().filter(|(_, a)| a["rows"].as_array().unwrap().len() == 1).count();
    assert_eq!(went, 300, "every send that went was one original, and no more went than the day's messages");
    assert!(fwd.len() > 300, "the halved groups were refused and counted nothing: {}", fwd.len());
    let mut once: Vec<String> = fwd.iter().filter(|(_, a)| a["rows"].as_array().unwrap().len() == 1).flat_map(|(_, a)| row_texts(a)).collect();
    once.sort();
    once.dedup();
    assert_eq!(once.len(), 300, "each original that went, went once");
    drop(fwd);
    // The next pass the same day sends nothing: the routine waits for the first message to be a day old.
    let again = fake(Provider::Gmail, unheld(5));
    let out = pass(&again, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, again.forwards.borrow().len()), (Stop::DayLimit { next_day_at: first + DAY }, 0));
}

#[test]
fn a_newer_press_the_days_pages_cannot_pay_for_ends_the_pass_as_a_page_limit_never_done() {
    // 591 of the day's 600 loads spent: the fresh Inbox (two loads since the review of c8806b3; this test held 592
    // when it was one), the search and its two presses leave none for "newer" beyond the
    // end's own four. list.js answers "no presses left" (never `moved: false`, which reads as the newest page), and
    // the pass ends as a page limit with its end still run.
    let mut mb = fake(Provider::Gmail, unheld(30));
    mb.size = 10;
    let mut r = routine(Provider::Gmail).with_loads(591);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent), (Stop::PageLimit { next_day_at: (NOW.div_euclid(DAY) + 1) * DAY }, 10));
    let last = mb.calls("list.js").last().cloned().unwrap();
    assert_eq!((last["go"].as_str(), last["max_presses"].as_u64()), (Some("newer"), Some(0)), "{last}");
    assert!(out.closed && sent_at_each_look(&mb) == [10], "the look and the batch's close still ran");
    assert_eq!(u64::from(r.loads_today()), 591 + mb.loads());
    assert!(r.loads_today() <= 600, "{}", r.loads_today());
}

#[test]
fn the_pass_end_waits_out_undo_send_after_the_last_send_however_the_pass_stopped() {
    // Gmail holds a send for its Undo Send delay: the look's Inbox, the batch's close and Sent come at least 31 s
    // after the last send that went, whether the pass was done, stopped by the student or paused by a refusal.
    // Two groups of three go in the same minute, so nothing else spaces them.
    let stop_after_one = |_: u32, _: u32, sent: u32| sent == 0;
    let watch: &dyn Fn(u32, u32, u32) -> bool = &stop_after_one;
    let cases = [(None, None, Stop::Done), (None, Some(watch), Stop::Stopped), (Some(1), None, Stop::Paused(Paused::new(Provider::Gmail)))];
    for (refuse, watch, want) in cases {
        let mut mb = fake(Provider::Gmail, unheld(6));
        mb.size = 3;
        mb.refuse_on.set(refuse);
        let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &Job { watch, ..gmail_backfill() });
        assert_eq!(out.stop, want);
        let fwd = mb.forwards.borrow();
        let last_ok = fwd.iter().enumerate().filter(|(i, _)| Some(*i) != refuse).map(|(_, (t, _))| *t).max().unwrap();
        let ends: Vec<(Place, i64)> = mb.opened_at.borrow().iter().filter(|(p, _)| matches!(p, Place::Inbox | Place::Sent)).copied().collect();
        assert!(ends.iter().any(|(p, _)| *p == Place::Sent), "{want:?}: {ends:?}");
        assert!(ends.iter().all(|(_, t)| *t >= last_ok + 31), "{want:?}: {ends:?} vs the last send at {last_ok}");
    }
}

#[test]
fn a_1485_message_backfill_is_done_on_its_first_day_in_under_400_page_loads() {
    // The design's sizing (2026-10-19): 1,485 messages over a 120-day window, 45 to a page. Quinn's day-cap ruling
    // (2026-10-19) gives Gmail 1,500 originals in at most 300 messages and 600 page loads a day: the whole window
    // goes on the first day, in groups of ten (149 messages). Each day is one pass from the frontier the last kept
    // (forward only), against a service that holds what went; a second day would find nothing left.
    let first = NOW.div_euclid(DAY) - 120;
    let msgs: Vec<Mail> = (0..1_485).map(|i| mail(ANA, &format!("Notice {i}"), first * DAY + 3_600 + i * 6_900)).collect();
    let mut sent: Vec<String> = Vec::new();
    let mut frontier: Option<i64> = None;
    let mut days = Vec::new();
    for d in 0..2 {
        let start = NOW + d * DAY;
        let went: std::collections::HashSet<String> = sent.iter().cloned().collect();
        let held: Vec<(&str, &str, i64)> =
            msgs.iter().filter(|m| went.contains(&received(m.at))).map(|m| (m.sender.as_str(), m.subject.as_str(), m.at)).collect();
        let svc = holding(&held);
        let mut mb = fake(Provider::Gmail, msgs.clone());
        mb.size = 45;
        mb.clk.0.set(start);
        let mut r = Routine::new(Provider::Gmail, start.div_euclid(DAY), 0);
        let j = Job { frontier, ..job(Provider::Gmail, Phase::Backfill, backfill_window(first, start), None) };
        let out = pass(&mb, &svc, &mut r, &j);
        if let Some(f) = out.frontier.filter(|_| out.stop != Stop::Done) {
            frontier = Some(frontier.map_or(f, |o| o.max(f)));
        }
        assert_eq!(u64::from(r.loads_today()), mb.loads(), "day {d}");
        sent.extend(mb.forwarded());
        days.push((out.stop, out.sent, r.messages_today(), mb.loads(), mb.presses.get()));
    }
    let stops: Vec<(Stop, u32)> = days.iter().map(|d| (d.0.clone(), d.1)).collect();
    assert_eq!(stops, [(Stop::Done, 1_485), (Stop::Done, 0)], "{days:?}");
    // Measured (2026-10-19): 293 messages, 380 loads and 139 presses on the first day (320 loads while a fresh
    // Inbox was charged one load; it is two since the review of c8806b3). A look comes every 25 to 30 sends, so about
    // 60 restarts, each the look's Inbox, the fresh Inbox (two), the narrowed search and a press or two, with the first
    // search's count walk (40 presses). Groups fall to five after the first look: the minute's ten slots free in two
    // halves. The bar sits a little above the measure, as the 160 one did.
    assert!(days[0].2 <= 300 && days[0].3 <= 400, "{days:?}");
    let mut once = sent.clone();
    once.sort();
    once.dedup();
    assert_eq!((once.len(), sent.len()), (1_485, 1_485), "every message went once");
}

#[test]
fn a_fresh_inbox_is_two_page_loads_and_every_conversation_opened_is_charged() {
    // Review of c8806b3: a fresh Inbox is a navigation and a reload, two loads, not one; and opening a conversation
    // (thread.js: the row's open, any Expand all, the back press) is page loads too, charged to the same day's
    // budget as the pager's. Ten conversations of three messages, a page of ten.
    let mut mb = fake(Provider::Gmail, convos(10, 3));
    mb.size = 10;
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent), (Stop::Done, 30));
    let fresh = mb.places.borrow().iter().filter(|p| **p == Place::FreshInbox).count() as u64;
    assert!(fresh >= 2 && mb.calls("thread.js").len() >= 10, "{fresh}");
    assert_eq!(u64::from(r.loads_today()), mb.loads(), "fresh opens at two, and the conversations' presses");
    // A conversation the day's pages cannot pay for (its open, three Expand all and its back) is not opened: the
    // pass ends as a page limit.
    let mut tight = fake(Provider::Gmail, convos(3, 3));
    tight.size = 10;
    let mut r = routine(Provider::Gmail).with_loads(590);
    let out = pass(&tight, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!(out.stop, Stop::PageLimit { next_day_at: (NOW.div_euclid(DAY) + 1) * DAY });
    assert!(tight.calls("thread.js").is_empty());
    assert!(r.loads_today() <= 600, "{}", r.loads_today());
}

#[test]
fn a_page_script_that_gives_no_answer_is_charged_what_it_may_have_pressed() {
    // Review of c8806b3: a call that ran out its step after pressing charged nothing. With no answer the routine
    // charges the bound it gave the call: a "newer" its max_presses, a conversation's read five (its open, three
    // Expand all, its back).
    let mut mb = fake(Provider::Gmail, unheld(30));
    mb.size = 10;
    mb.newer_times_out.set(true);
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!(out.stop, Stop::Script(Refusal::TimedOut.as_str()));
    let newer = mb.calls("list.js").into_iter().rfind(|a| a["go"] == "newer").expect("a newer was asked");
    let allowed = newer["max_presses"].as_u64().unwrap();
    assert!(allowed > 0);
    assert_eq!(u64::from(r.loads_today()), mb.loads() + allowed);
    let convo = fake(Provider::Gmail, convos(2, 3));
    convo.thread_times_out.set(true);
    let mut r = routine(Provider::Gmail);
    let out = pass(&convo, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!(out.stop, Stop::Script(Refusal::TimedOut.as_str()));
    assert_eq!(u64::from(r.loads_today()), convo.loads() + 5);
}

#[test]
fn a_list_whose_range_cannot_be_read_stops_the_pass_by_name_and_is_never_its_end() {
    // Review of c8806b3: list.js read a page with no range as the end, so a seek could stop short and the pass end
    // Done past pages it never read. list.js now refuses by name; the pass stops there, never Done, sends nothing
    // and keeps the row it was seeking as its frontier's bound.
    let since = window(NOW, None, None).since;
    let msgs: Vec<Mail> = (0..10).map(|i| mail(ANA, &format!("Note {i}"), since + 3_600 * (i + 1))).collect();
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 2;
    mb.oldest_walk = Some(0);
    mb.range_unreadable.set(true);
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!((out.stop, out.sent, out.handled), (Stop::Script("the page range could not be read"), 0, 0));
    assert!(out.frontier.is_none_or(|f| f < since.div_euclid(DAY)), "{:?}", out.frontier);
}

#[test]
fn a_left_row_never_holds_the_frontier_back_and_a_sent_one_does() {
    // Review of c8806b3 (plan :2314: a left row is never retried): the frontier keeps the oldest row sent (a bounced
    // batch is searched again) and drops rows that were left. A conversation that will not attach, three days before
    // the first single, is left; the day's sends run out after ten singles.
    let first = NOW.div_euclid(DAY) - 20;
    let mut convo = mail(ANA, "Group project", (first + 2) * DAY + 7_200);
    convo.count = Some(2);
    convo.convo = vec![(ANA.to_string(), (first + 2) * DAY + 3_600), (ANA.to_string(), (first + 2) * DAY + 7_200)];
    let mut msgs = daily(BO, "Notice", first + 5, 30, 5);
    msgs.insert(0, convo);
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 50;
    mb.mismatch = true;
    // 1,490 sent today: the singles take the last ten. The conversation sent nothing, so it gives back the two it
    // took (review round 2 of e7ae803, #5; this test started at 1,488 while it kept them).
    let mut r = Routine::new(Provider::Gmail, NOW.div_euclid(DAY), 1_490);
    let out = pass(&mb, &holding(&[]), &mut r, &job(Provider::Gmail, Phase::Backfill, backfill_window(first, NOW), None));
    assert_eq!((out.stop, out.sent, out.left), (Stop::DayLimit { next_day_at: (NOW.div_euclid(DAY) + 1) * DAY }, 10, 1));
    assert_eq!(out.frontier, Some(first + 5 - 1), "the day before the first single sent, not before the left conversation");
    // A lone row mail.json says stalled two days is left at once from the third (it will not attach), and no more
    // holds the frontier back than a conversation left.
    let stuck_at = (first + 2) * DAY + 3_600;
    let mut msgs = daily(BO, "Notice", first + 5, 30, 5);
    msgs.insert(0, mail(ANA, "Stuck", stuck_at));
    let mut mb = fake(Provider::Gmail, msgs);
    mb.size = 50;
    mb.no_chip = Some("Stuck");
    // The stuck row left at once gives back its original too (1,489 while it kept it).
    let mut r = Routine::new(Provider::Gmail, NOW.div_euclid(DAY), 1_490);
    let j = Job { stuck: Some(Stuck { at: stuck_at, leave: true }), ..job(Provider::Gmail, Phase::Backfill, backfill_window(first, NOW), None) };
    let out = pass(&mb, &holding(&[]), &mut r, &j);
    assert_eq!((out.stop, out.sent, out.left), (Stop::DayLimit { next_day_at: (NOW.div_euclid(DAY) + 1) * DAY }, 10, 1));
    assert_eq!(out.frontier, Some(first + 5 - 1));
    assert_eq!(out.stalled_at, None);
}

#[test]
fn every_look_that_read_the_inbox_is_told_to_the_job_clean_or_refused_and_an_unreadable_one_is_not() {
    // The job's `on_look` is how a look reaches `mail.json`; it hears the clock at each look that read the Inbox.
    let told = RefCell::new(Vec::new());
    let cb = |t: i64| told.borrow_mut().push(t);
    let mut mb = fake(Provider::Gmail, unheld(60));
    mb.size = 10;
    mb.bounce_after = Some(6);
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &Job { on_look: Some(&cb), ..gmail_backfill() });
    // Looks after 30 (clean) and 60 (a report, refused): both are told, in the order they ran.
    assert_eq!(out.stop, Stop::Paused(Paused::new(Provider::Gmail)));
    assert_eq!(*told.borrow(), *mb.looked_at.borrow());
    assert_eq!(told.borrow().len(), 2);
    let blind = RefCell::new(Vec::new());
    let cb = |t: i64| blind.borrow_mut().push(t);
    let mut mb = fake(Provider::Gmail, unheld(12));
    mb.inbox_unreadable = true;
    pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &Job { on_look: Some(&cb), ..gmail_backfill() });
    assert!(blind.borrow().is_empty(), "a look that could not read the Inbox is no look");
}

#[test]
fn a_mail_json_from_before_last_look_loads_with_none_and_writes_none_until_there_is_one() {
    let dir = temp("last-look-old-file");
    let old = r#"{"keep_session":true,"last_reconcile":{"0b0e5b2c-1111-4aaa-8bbb-000000000001":"2026-10-02T10:00:40Z"},"pending_deletes":[],"turned_off":[]}"#;
    std::fs::write(dir.join("mail.json"), old).unwrap();
    let s = MailState::load(&dir).expect("an older file loads");
    assert!(s.last_look.is_empty() && s.last_look.get(A).is_none());
    assert_eq!(s.last_reconcile.len(), 1, "what it did hold is read");
    // Written only once a look exists, beside the other per-mailbox facts, and kept by an update of another field.
    MailState::update(&dir, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    assert!(!std::fs::read_to_string(dir.join("mail.json")).unwrap().contains("last_look"));
    MailState::update(&dir, |s| {
        s.last_look.insert(A.to_string(), NOW);
        true
    })
    .unwrap();
    assert_eq!(MailState::load(&dir).unwrap().last_look.get(A), Some(&NOW));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_resumed_backfill_saves_its_looks_and_starts_from_the_last_one_saved() {
    let r = open_backfill_rig("last-look-backfill", 2);
    resume_and_time_forwards(&r);
    let saved = *MailState::load(&r.dir).unwrap().last_look.get(A).expect("the final look was saved");
    assert!(saved >= NOW, "{saved}");
    assert_eq!(r.calls("gmail/bounce.js")[0]["since"], json!(iso(NOW - 24 * 3_600)), "none saved: a day before the batch opened");
    // A relaunch seeds the routine from it: the first look reads from there, and nothing reaches back further.
    let again = open_backfill_rig("last-look-backfill-again", 2);
    MailState::update(&again.dir, |s| {
        s.last_look.insert(A.to_string(), NOW - 100);
        true
    })
    .unwrap();
    resume_and_time_forwards(&again);
    assert_eq!(again.calls("gmail/bounce.js")[0]["since"], json!(iso(NOW - 100)));
}

#[test]
fn a_restart_whose_fresh_inbox_will_not_open_stops_the_pass_with_that_refusals_word() {
    let mut mb = fake(Provider::Gmail, unheld(60));
    mb.size = 20;
    mb.fresh_refuses_after = Some(1);
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!(out.stop, Stop::Script(Refusal::TimedOut.as_str()));
    assert_eq!(out.sent, 30, "it stopped at the first look's restart");
    let names = steps(&mb);
    let first = names.iter().position(|n| n.ends_with("bounce.js")).unwrap();
    assert_eq!(names[first + 1], "open FreshInbox");
    assert_eq!(mb.calls("search.js").len(), 1, "no search was made over the cached view");
    assert!(names[first + 2..].iter().all(|n| !n.ends_with("search.js") && !n.ends_with("list.js") && !n.ends_with("forward.js")), "{names:?}");
    // One that will not open at the start ends the pass before it searches anything.
    let mut first_open = fake(Provider::Gmail, unheld(5));
    first_open.fresh_refuses_after = Some(0);
    let out = pass(&first_open, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!((out.stop, out.sent), (Stop::Script(Refusal::TimedOut.as_str()), 0));
    assert!(first_open.calls("search.js").is_empty());
}

#[test]
fn every_gmail_pass_searches_a_freshly_loaded_inbox_and_microsoft_365_does_not_need_one() {
    // A backfill keeps one window across its passes (it waits through a day limit or a pause into the next
    // day), and the last pass's looks left the cached search view standing: a second pass over the same window
    // would search over it and walk off its end. Each pass starts from a fresh document.
    let mut mb = fake(Provider::Gmail, unheld(12));
    mb.size = 10;
    let svc = holding(&[]);
    let mut r = routine(Provider::Gmail);
    let first = pass(&mb, &svc, &mut r, &gmail_backfill());
    assert_eq!((first.stop, first.expected, first.sent), (Stop::Done, 12, 12));
    mb.clk.0.set(mb.clk.now() + 600);
    let second = pass(&mb, &svc, &mut r, &gmail_backfill());
    assert_eq!((second.stop, second.expected, second.sent), (Stop::Done, 12, 12));
    let fresh = mb.places.borrow().iter().filter(|p| **p == Place::FreshInbox).count();
    assert_eq!(fresh, 2, "one for each pass's search");
    let m365 = fake(Provider::M365, unheld(3));
    pass(&m365, &holding(&[]), &mut routine(Provider::M365), &job(Provider::M365, Phase::Recover, window(NOW, None, None), Some(SLOT_CAP)));
    assert!(m365.places.borrow().iter().all(|p| *p == Place::Inbox || *p == Place::Sent), "Microsoft 365 reloads nothing");
}

#[test]
fn a_restart_waits_out_gmails_undo_send_delay_after_the_last_send_before_the_fresh_open() {
    // forward.js answers ok when the compose closes, but Gmail holds the send for the student's Undo Send delay
    // (30 seconds at most): a reload inside it could cancel the held send or raise a "Leave site?" dialog.
    let mut mb = fake(Provider::Gmail, unheld(60));
    mb.size = 20;
    let out = pass(&mb, &holding(&[]), &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!((out.stop, out.sent), (Stop::Done, 60));
    // The pass started with no send behind it: no wait.
    assert_eq!(mb.fresh_at.borrow()[0], NOW);
    // The restart came right after the third forward (the 30th send): the fresh open is 31 seconds after it.
    let third = mb.forwards.borrow()[2].0;
    assert_eq!(mb.fresh_at.borrow()[1], third + 31);
}

#[test]
fn a_pass_started_soon_after_a_send_waits_and_one_started_long_after_does_not() {
    let svc = holding(&[]);
    let mut r = routine(Provider::Gmail);
    let mut first = fake(Provider::Gmail, unheld(3));
    first.size = 10;
    pass(&first, &svc, &mut r, &gmail_backfill());
    let sent_at = first.forwards.borrow()[0].0;
    // Five seconds later, a second pass in the same run (the same window) waits for the delay to pass.
    let mut soon = fake(Provider::Gmail, unheld(3));
    soon.clk = first.clk.clone();
    soon.clk.0.set(sent_at + 5);
    pass(&soon, &svc, &mut r, &gmail_backfill());
    assert_eq!(soon.fresh_at.borrow()[0], sent_at + 31);
    // Long after any send there is nothing to wait for.
    let last = soon.forwards.borrow()[0].0;
    let mut late = fake(Provider::Gmail, unheld(3));
    late.clk = first.clk.clone();
    late.clk.0.set(last + 3_600);
    pass(&late, &svc, &mut r, &gmail_backfill());
    assert_eq!(late.fresh_at.borrow()[0], last + 3_600);
    // And a routine that has not sent anything waits for nothing.
    let none = fake(Provider::Gmail, unheld(3));
    pass(&none, &svc, &mut routine(Provider::Gmail), &gmail_backfill());
    assert_eq!(none.fresh_at.borrow()[0], NOW);
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
    /// The title turns signed in once the clock reaches this second.
    signed_in_from: Option<(Arc<SClk>, i64)>,
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
            // {"refuse": "lost" | "failed"} is a refusal, not an answer.
            return match v["refuse"].as_str() {
                Some("lost") => Err(Refusal::Lost),
                Some("failed") => Err(Refusal::Failed),
                _ => Ok(v),
            };
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
            // Gmail looks in the Inbox for a non-delivery report between sends and once at the end.
            "gmail/bounce.js" => json!({ "result": "ok", "rows": 0 }),
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
        let s = self.st.lock().unwrap();
        s.signed_in || s.signed_in_from.as_ref().is_some_and(|(clk, at)| clk.at() >= *at)
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
    /// Canaries sent, and how many more the day's limit takes (`None`: no limit; then 429, §4.10).
    canaries: usize,
    canaries_left: Option<usize>,
    /// Called as each canary is asked for (the test reads the progress and the clock there).
    on_canary: Option<Box<dyn Fn() + Send>>,
    confirming: bool,
    /// The address never reads `confirmed` (the provider's mail never comes), whatever `confirming` says.
    never_confirm: bool,
    hint: Option<String>,
    /// The `since` of each backfill batch opened.
    batches: Vec<String>,
    /// Set by a resume test: what the row says about the backfill.
    backfill_since: Option<String>,
    provider: String,
    /// `POST /addresses/<id>/rotate` answers 429 (the mint limit).
    rotate_refused: bool,
    /// `POST /addresses` answers this status and `{"error": <sentence>}` (D3's caps).
    mint_refused: Option<(u16, String)>,
    /// `POST /addresses/<id>/hint` answers this status and `{"error": <sentence>}`.
    hint_refused: Option<(u16, String)>,
    /// Called as each hint is posted, before its answer (a test reads Settings' status there).
    on_hint: Option<Box<dyn Fn() + Send>>,
    /// `DELETE /addresses/<id>` answers 502 (the mailbox's stored mail could not be deleted).
    delete_refused: bool,
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
                if let Some((status, why)) = &s.mint_refused {
                    return (*status, json!({ "error": why }).to_string());
                }
                s.provider = body["provider"].as_str().unwrap_or("").to_string();
                (201, json!({ "address_id": A, "address": KNOWLU, "provider": s.provider, "status": "pending" }).to_string())
            }
            ("GET", "/addresses") => {
                let ok_at = (s.canary_sent && s.canary_arrives).then(|| "2026-10-02T10:00:40Z");
                let status = if ok_at.is_some() { "active" } else if s.confirming && !s.never_confirm { "confirmed" } else { "pending" };
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
                if let Some(f) = &s.on_hint {
                    f();
                }
                if let Some((status, why)) = &s.hint_refused {
                    return (*status, json!({ "error": why }).to_string());
                }
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
                if let Some(f) = &s.on_canary {
                    f();
                }
                if s.canaries_left == Some(0) {
                    return (429, r#"{"error":"this mailbox has had three delivery checks today"}"#.to_string());
                }
                s.canaries_left = s.canaries_left.map(|n| n - 1);
                s.canaries += 1;
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
                if s.delete_refused {
                    return (502, r#"{"error":"the mailbox's stored mail could not be deleted"}"#.to_string());
                }
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

/// Waits for the setup to end. D13: a Microsoft 365 setup stops at the guided backfill step while C3 is built,
/// and the student's "I've done this" (sent here once) closes it; every other state is left to the test.
fn wait(shared: &Shared) -> Progress {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let mut closed_guide = false;
    while !shared.wait_done(Duration::from_millis(20)) {
        assert!(std::time::Instant::now() < deadline, "the setup did not end");
        if !closed_guide && shared.progress().provider == "m365" && has(shared, "backfill", "needs_you") {
            closed_guide = shared.answer(Answer::Done);
        }
    }
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
    // Forwarding was already on to Knowlu (nothing changed): one canary, waited three minutes (D14). A
    // forwarding set in this run is waited for first (T18c, 2026-10-07): the next test.
    let kept = json!({ "result": "ok", "forwarding": "on", "to": "ours", "keep_copy": true, "changed": false });
    queue(&r, "m365/forwarding_set.js", vec![kept]);
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
    assert!(ran_retired(&r).is_empty(), "no backfill is read while forwarding does not arrive");
    assert!(!r.events().contains(&"svc batch".to_string()));
    // Retry: the school's setting is fixed and the canary comes back.
    r.sv.lock().unwrap().canary_arrives = true;
    assert!(shared.answer(Answer::Retry));
    until("the canary done by hand", || has(&shared, "canary", "done_by_hand"));
    // D13: the canary arriving does not re-arm a Microsoft backfill (it is held while C3 is built); it stays the
    // guided step, and the student's "I've done this" ends the setup naming the day mail was not brought in from.
    assert_eq!(row(&shared.progress(), "backfill").state, "failed");
    assert!(shared.answer(Answer::Done));
    let p = wait(&shared);
    assert_eq!(p.ended, Some("finished"));
    assert_eq!(p.not_brought_in_before, Some(since_day()));
    assert!(ran_retired(&r).is_empty(), "{:?}", r.scripts());
    assert!(r.sv.lock().unwrap().batches.is_empty(), "no backfill ran after the Retry");
    assert!(MailState::load(&r.dir).unwrap().open_backfills.is_empty());
}

/// Each canary asked for: the clock, and the sentence the setup showed at that moment.
fn watch_canaries(r: &Rig) -> Arc<Mutex<Vec<(i64, &'static str)>>> {
    let seen: Arc<Mutex<Vec<(i64, &'static str)>>> = Default::default();
    let (log, clk, setups) = (seen.clone(), r.clk.clone(), r.setups.clone());
    r.sv.lock().unwrap().on_canary = Some(Box::new(move || {
        let sentence = setups.current().map(|c| c.progress().sentence).unwrap_or("");
        log.lock().unwrap().push((clk.at(), sentence));
    }));
    seen
}

#[test]
fn a_forwarding_just_set_is_given_time_to_start_and_a_second_canary_before_the_step_fails() {
    // T18c (2026-10-07, live): the first canary went the second Outlook's Save took and never came back (no
    // non-delivery report); a Retry seven minutes later came back at once. Outlook had not begun forwarding.
    // So once this run has set forwarding, the canary waits FORWARD_SETTLE first, saying so, and one that does
    // not come back is followed by one more before the step fails: two of the day's three (§4.10).
    let r = rig("t18c-settle");
    r.sv.lock().unwrap().canary_arrives = false;
    let seen = watch_canaries(&r);
    let shared = r.start(curated_m365(), true);
    until("the blocked state", || shared.progress().blocked);
    let p = shared.progress();
    assert_eq!(row(&p, "canary").state, "failed");
    assert_eq!(r.sv.lock().unwrap().canaries, 2, "one more, never a third");
    let seen = seen.lock().unwrap().clone();
    assert!(seen[0].0 - NOW >= 120, "the first waited for forwarding to start: {seen:?}");
    assert!(seen[1].0 - seen[0].0 >= 180, "the second after the first's three minutes: {seen:?}");
    assert_eq!(seen[1].1, "waiting for Outlook to start forwarding", "the wait says why");
    let waited = r.clk.at() - NOW;
    assert!((480..600).contains(&waited), "bounded: {waited}");
    assert!(!r.events().contains(&"svc batch".to_string()), "no backfill while forwarding does not arrive");
    shared.stop();
    wait(&shared);

    // A canary that comes back takes one: the second is never sent.
    let ok = rig("t18c-settle-ok");
    let seen = watch_canaries(&ok);
    let p = wait(&ok.start(curated_m365(), true));
    assert_eq!(p.ended, Some("finished"), "D13: a Microsoft setup ends at the guided backfill");
    assert_eq!(ok.sv.lock().unwrap().canaries, 1);
    let seen = seen.lock().unwrap().clone();
    assert!(seen[0].0 - NOW >= 120, "{seen:?}");

    // The day's limit refuses the second: the step fails as before, and nothing asks again.
    let capped = rig("t18c-settle-cap");
    {
        let mut sv = capped.sv.lock().unwrap();
        sv.canary_arrives = false;
        sv.canaries_left = Some(1);
    }
    let shared = capped.start(curated_m365(), true);
    until("the blocked state", || shared.progress().blocked);
    assert_eq!(row(&shared.progress(), "canary").state, "failed");
    assert_eq!(capped.events().iter().filter(|e| *e == "svc canary").count(), 2, "asked twice, refused once");
    shared.stop();
    wait(&shared);
}

/// A clock that moves only when the runner waits and sets the setup's stop on the third wait after `when`
/// first reads true (F8d, F1 test 30's runner cases); `tripped` is the fake second the stop was set.
struct TripClk {
    clk: Arc<SClk>,
    setups: Arc<Setups>,
    when: Box<dyn Fn() -> bool + Send>,
    seen: AtomicU32,
    tripped: Arc<AtomicI64>,
}

impl Clock for TripClk {
    fn now(&self) -> i64 {
        self.clk.at()
    }
    fn sleep(&self, secs: i64) {
        self.clk.0.fetch_add(secs, Ordering::SeqCst);
        if (self.when)() && self.seen.fetch_add(1, Ordering::SeqCst) == 2 {
            self.tripped.store(self.clk.at(), Ordering::SeqCst);
            self.setups.current().expect("a setup is running").stop();
        }
    }
}

/// Runs `kind` on a rig whose clock stops the setup once `when` holds; returns the ended progress and how many
/// fake seconds the run took to end after the stop.
fn stopped_while(r: &Rig, kind: Kind, when: Box<dyn Fn() -> bool + Send>) -> (Progress, i64) {
    let tripped = Arc::new(AtomicI64::new(0));
    let clock = TripClk { clk: r.clk.clone(), setups: r.setups.clone(), when, seen: AtomicU32::new(0), tripped: tripped.clone() };
    let env = Env { clock: Box::new(clock), ..r.env() };
    let shared = r.setups.start(r.plan(kind, true), env).expect("a setup started");
    let p = wait(&shared);
    let at = tripped.load(Ordering::SeqCst);
    assert!(at > 0, "the stop was never set: {p:?}");
    (p, r.clk.at() - at)
}

fn assert_stop_not_failure(p: &Progress, step: &str, took: i64) {
    assert_eq!(p.ended, Some("stopped"), "a stop, never a failure: {p:?}");
    assert!(took <= 5, "it ended within one POLL of the stop, not at the wait's limit: {took} s");
    assert_eq!(row(p, step).state, "skipped", "{step} is neither failed nor done");
    assert_eq!(row(p, "sign_in").state, "done", "a step done before the stop stays done");
    assert!(!p.blocked && p.offers.is_empty(), "no blocked mailbox is offered a retry");
}

#[test]
fn a_stop_during_the_address_confirm_ends_the_run_within_one_poll_and_is_not_a_failed_confirm() {
    let r = rig("f8d-confirm");
    r.sv.lock().unwrap().never_confirm = true;
    r.host.lock().unwrap().title_address = Some(HINT_PAGE.to_string());
    let setups = r.setups.clone();
    let when = Box::new(move || setups.current().is_some_and(|c| c.progress().sentence == "Knowlu is confirming the address"));
    let (p, took) = stopped_while(&r, Kind::Gmail, when);
    assert_stop_not_failure(&p, "confirm_address", took);
    assert!(r.clk.at() - NOW < 15 * 60, "never CONFIRM_WAIT");
    assert!(!r.events().contains(&"svc canary".to_string()), "nothing after the confirm ran");
}

#[test]
fn a_stop_while_forwarding_settles_sends_no_canary_and_is_not_a_failed_canary() {
    let r = rig("f8d-settle");
    r.sv.lock().unwrap().canary_arrives = false;
    let setups = r.setups.clone();
    let when = Box::new(move || setups.current().is_some_and(|c| c.progress().sentence == "waiting for Outlook to start forwarding"));
    let (p, took) = stopped_while(&r, curated_m365(), when);
    assert_stop_not_failure(&p, "canary", took);
    assert_eq!(r.sv.lock().unwrap().canaries, 0, "no canary was sent after the stop");
    assert_eq!(row(&p, "forwarding").state, "done");
}

#[test]
fn a_stop_while_the_canary_is_awaited_ends_within_one_poll_sends_no_second_canary_and_is_not_blocked() {
    let r = rig("f8d-canary-back");
    r.sv.lock().unwrap().canary_arrives = false;
    // Forwarding was already on to Knowlu (nothing changed): no settle, one canary waited for.
    let kept = json!({ "result": "ok", "forwarding": "on", "to": "ours", "keep_copy": true, "changed": false });
    queue(&r, "m365/forwarding_set.js", vec![kept]);
    let sv = r.sv.clone();
    let (p, took) = stopped_while(&r, curated_m365(), Box::new(move || sv.lock().unwrap().canaries >= 1));
    assert_stop_not_failure(&p, "canary", took);
    assert_eq!(r.sv.lock().unwrap().canaries, 1, "never the second");
    assert!(r.clk.at() - NOW < 180, "never CANARY_WAIT");
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
    assert_eq!((p.ended, p.provider), (Some("finished"), "m365"), "D13: a Microsoft setup ends at the guided backfill");
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
    for (tag, kind, typed) in [("t113-date-a", curated_m365(), false), ("t113-date-b", uncurated(), true), ("t113-date-c", Kind::Gmail, false)] {
        let r = rig(tag);
        {
            let mut h = r.host.lock().unwrap();
            h.title_address = Some(HINT_PAGE.to_string());
            h.msgs = ana_mail(1);
        }
        let shared = r.start(kind.clone(), true);
        if typed {
            answer_address(&shared);
        }
        let p = wait(&shared);
        if kind == Kind::Gmail {
            assert_eq!(p.ended, Some("finished"), "{tag}");
            assert_eq!(r.sv.lock().unwrap().batches, [since_day()], "{tag}: the batch opens on the fixed window start");
            assert_eq!(p.backfill.as_ref().map(|b| b.since.clone()), Some(since_day()), "{tag}");
        } else {
            // D13: Microsoft's backfill is the guided step; the window start is still the fixed 120 days.
            assert_eq!(p.ended, Some("finished"), "{tag}");
            assert!(r.sv.lock().unwrap().batches.is_empty(), "{tag}: no backfill batch opens");
            assert_eq!(p.not_brought_in_before, Some(since_day()), "{tag}");
        }
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
    // Gmail: Microsoft 365's backfill is held while C3 is built (D13).
    {
        let mut h = r.host.lock().unwrap();
        h.title_address = Some(HINT_PAGE.to_string());
        h.msgs = ana_mail(2);
    }
    let p = wait(&r.start(Kind::Gmail, true));
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
    assert_eq!(p.ended, Some("finished"), "D13: ends at the guided backfill");
    assert_eq!(off.host.lock().unwrap().opens, [(setup_dir(&off.dir), true)]);
    assert!(!setup_dir(&off.dir).exists(), "mail-setup is wiped when the window closes");
    assert!(!session_dir(&off.dir, A).unwrap().exists());
    assert!(MailState::load(&off.dir).unwrap().pending_deletes.is_empty());

    let on = rig("t113-keep-on");
    let p = wait(&on.start(curated_m365(), true));
    assert_eq!(p.ended, Some("finished"), "D13: ends at the guided backfill");
    assert_eq!(on.host.lock().unwrap().opens, [(session_dir(&on.dir, A).unwrap(), true)]);
    assert!(session_dir(&on.dir, A).unwrap().exists(), "the kept profile stays");
    assert!(!setup_dir(&on.dir).exists());
}

/// A Gmail mailbox (Microsoft 365's backfill is held while C3 is built, D13) whose first backfill began and was
/// cut off: the service holds its window start, the kept
/// profile is on disk, and `mail.json` marks it open.
fn open_backfill_rig(tag: &str, n: usize) -> Rig {
    let r = rig(tag);
    {
        let mut s = r.sv.lock().unwrap();
        s.provider = "google".into();
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
    // Gmail forwards a page of messages in one call: one call, both messages.
    let calls = r.calls("gmail/forward.js");
    assert_eq!((calls.len(), calls.iter().map(|c| c["rows"].as_array().map_or(1, |r| r.len())).sum::<usize>()), (1, 2));
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
fn a_quit_mid_backfill_leaves_its_nonce_in_mail_json_and_the_resume_sweeps_it_first() {
    let r = open_backfill_rig("sent-nonce-resume", 2);
    MailState::update(&r.dir, |s| {
        s.sent_nonces.insert(A.to_string(), vec![SentNonce { nonce: OLD.into(), opened: NOW - 100 }]);
        true
    })
    .unwrap();
    // What mail.json holds when the first forward runs: the old nonce is already swept and this pass's is kept.
    let held = Arc::new(Mutex::new(Vec::new()));
    let (h, dir) = (held.clone(), r.dir.clone());
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "gmail/forward.js" {
            let st = MailState::load(&dir).unwrap();
            h.lock().unwrap().push(st.sent_nonces.get(A).map(|v| v.iter().map(|e| e.nonce.clone()).collect::<Vec<_>>()));
        }
    }));
    let shared = r.setups.start_resume(r.dir.clone(), vec![A.to_string()], r.env()).expect("a resume started");
    assert_eq!(wait(&shared).ended, Some("finished"));
    let first = r.calls("gmail/sent_cleanup.js").into_iter().next().expect("Sent was cleaned");
    assert_eq!((first["nonce"].as_str(), first["address"].as_str()), (Some(OLD), Some(KNOWLU)));
    let scripts = r.scripts();
    let at = |what: &str| scripts.iter().position(|n| n == what).unwrap_or_else(|| panic!("no {what} in {scripts:?}"));
    assert!(at("gmail/sent_cleanup.js") < at("gmail/forward.js"), "{scripts:?}");
    assert_eq!(*held.lock().unwrap().first().expect("a forward ran"), Some(vec![NONCE.to_string()]));
    assert!(MailState::load(&r.dir).unwrap().sent_nonces.is_empty(), "every nonce was cleaned and dropped");
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

use knowlu::mail::backfill::{drive_backfill, launch_resumable, open_backfills, Drive, Drove};

/// Runs the driver on `routine` (its provider's scripts) with the first forward stopping it (a quit mid-day), and
/// returns how it ended.
fn quit_after_first_forward(r: &Rig, provider: Provider, routine: &mut Routine) -> Drove {
    let env = r.env();
    let shared = Arc::new(Shared::new());
    let stopper = shared.clone();
    let forward = if provider == Provider::Gmail { "gmail/forward.js" } else { "m365/forward.js" };
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == forward {
            stopper.stop();
        }
    }));
    let d = Drive {
        mb: &*env.host,
        client: &env.client,
        clock: &*env.clock,
        provider,
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
        if name == "gmail/forward.js" {
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
fn a_relaunch_on_the_same_utc_day_never_sends_past_the_days_1500() {
    // Spec 4.2 step 5: a mailbox's day cap, Gmail's 1,500 since Quinn's ruling of 2026-10-19 (the mailbox is
    // Gmail's, so its pass is too; this test ran a Microsoft pass at 600 before). 1,499 were sent, one Gmail call
    // went (its row brings the day to 1,500), the app quit; the relaunch the same UTC day sends nothing until the
    // next day.
    let r = open_backfill_rig("t113-relaunch-day", 3);
    let next_day = (NOW.div_euclid(DAY) + 1) * DAY;
    let mut routine = Routine::new(Provider::Gmail, NOW.div_euclid(DAY), 1_499);
    assert_eq!(quit_after_first_forward(&r, Provider::Gmail, &mut routine), Drove::Stopped);
    assert_eq!(r.calls("gmail/forward.js").len(), 1);
    assert_eq!(MailState::load(&r.dir).unwrap().backfill_pace[A].sent_today, 1_500);
    r.clk.0.fetch_add(3_600, Ordering::SeqCst);
    let times = resume_and_time_forwards(&r);
    assert_eq!(times.len(), 1, "the window (one Gmail call for its three messages) is forwarded once the day turns: {times:?}");
    assert!(times.iter().all(|t| *t >= next_day), "nothing went before the next UTC day: {times:?} vs {next_day}");
}

#[test]
fn a_relaunch_the_next_utc_day_sends_no_gmail_message_until_the_first_of_the_300_is_a_day_old() {
    // Review round 2 of e7ae803, #2: Gmail's 300 messages are a rolling 24 hours, kept in mail.json with their
    // instants. 299 went over the five minutes before NOW (14:13 UTC), the 300th at NOW (the first group of ten of the
    // window's twelve), and the app quit; the relaunch at 01:00 the next UTC day sends nothing until the first of
    // them is 24 hours old.
    let r = open_backfill_rig("messages-rolling-relaunch", 12);
    let first = NOW - 299;
    let mut routine = Routine::new(Provider::Gmail, NOW.div_euclid(DAY), 0);
    for i in 0..299 {
        assert!(routine.reserve_message(first + i));
    }
    assert_eq!(quit_after_first_forward(&r, Provider::Gmail, &mut routine), Drove::Stopped);
    assert_eq!(routine.messages_today(), 300);
    r.clk.0.store((NOW.div_euclid(DAY) + 1) * DAY + 3_600, Ordering::SeqCst);
    let times = resume_and_time_forwards(&r);
    assert!(!times.is_empty() && times.iter().all(|t| *t >= first + DAY), "{times:?} vs {}", first + DAY);
}

#[test]
fn a_resume_after_a_refused_send_waits_for_the_next_utc_day() {
    // The pause after a refused send (an NDR) survives a relaunch: nothing retries until the next day.
    let r = open_backfill_rig("t113-relaunch-paused", 2);
    let day = NOW.div_euclid(DAY);
    let next_day = (day + 1) * DAY;
    MailState::update(&r.dir, |s| {
        s.backfill_pace.insert(A.to_string(), BackfillPace { day, sent_today: 41, paused_until: Some(next_day), loads_today: 0, messages_today: 0 });
        true
    })
    .unwrap();
    let times = resume_and_time_forwards(&r);
    assert_eq!(times.len(), 1, "one Gmail call for both messages: {times:?}");
    assert!(times.iter().all(|t| *t >= next_day), "{times:?} vs {next_day}");
    // A pace from an earlier day counts for nothing.
    let old = open_backfill_rig("t113-relaunch-old-pace", 2);
    MailState::update(&old.dir, |s| {
        s.backfill_pace.insert(A.to_string(), BackfillPace { day: day - 1, sent_today: 600, paused_until: Some(day * DAY), loads_today: 0, messages_today: 0 });
        true
    })
    .unwrap();
    assert!(resume_and_time_forwards(&old).iter().all(|t| *t < next_day), "yesterday's limit does not hold today");
}

#[test]
fn a_resume_on_a_day_whose_page_loads_are_spent_waits_for_the_next_utc_day() {
    // The day's 600 Gmail page loads kept in mail.json survive a relaunch: the backfill opens nothing and sends
    // nothing until the next UTC day, then goes on.
    let r = open_backfill_rig("loads-relaunch-spent", 2);
    let day = NOW.div_euclid(DAY);
    let next_day = (day + 1) * DAY;
    MailState::update(&r.dir, |s| {
        s.backfill_pace.insert(A.to_string(), BackfillPace { day, sent_today: 41, paused_until: None, loads_today: 600, messages_today: 0 });
        true
    })
    .unwrap();
    let times = resume_and_time_forwards(&r);
    assert_eq!(times.len(), 1, "one Gmail call for both messages: {times:?}");
    assert!(times.iter().all(|t| *t >= next_day), "{times:?} vs {next_day}");
}

#[test]
fn a_resume_on_a_day_whose_300_gmail_messages_are_spent_waits_for_the_next_utc_day() {
    // The day's Gmail messages (2026-10-19: at most 300) kept in mail.json survive a relaunch, as its sends and
    // loads do: with 41 originals sent but 300 messages, the backfill sends nothing until the next UTC day.
    let r = open_backfill_rig("messages-relaunch-spent", 2);
    let day = NOW.div_euclid(DAY);
    let next_day = (day + 1) * DAY;
    MailState::update(&r.dir, |s| {
        s.backfill_pace.insert(A.to_string(), BackfillPace { day, sent_today: 41, paused_until: None, loads_today: 0, messages_today: 300 });
        true
    })
    .unwrap();
    let times = resume_and_time_forwards(&r);
    assert_eq!(times.len(), 1, "one Gmail call for both messages: {times:?}");
    assert!(times.iter().all(|t| *t >= next_day), "{times:?} vs {next_day}");
}

#[test]
fn a_backfill_keeps_its_days_gmail_messages_in_mail_json_merged_never_lowered() {
    // A pass keeps the messages it sent beside its sends and loads; a same-day pace merges each by the larger, so
    // the 5 a relaunch carried in are never lowered by a progress note (which knows the sends only).
    let r = open_backfill_rig("messages-merge", 2);
    let day = NOW.div_euclid(DAY);
    MailState::update(&r.dir, |s| {
        s.backfill_pace.insert(A.to_string(), BackfillPace { day, sent_today: 3, paused_until: None, loads_today: 0, messages_today: 5 });
        true
    })
    .unwrap();
    let kept = Arc::new(Mutex::new(Vec::new()));
    let (k, dir) = (kept.clone(), r.dir.clone());
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if ["gmail/forward.js", "gmail/bounce.js", "gmail/sent_cleanup.js"].contains(&name) {
            k.lock().unwrap().push(MailState::load(&dir).unwrap().backfill_pace[A].clone());
        }
    }));
    let env = r.env();
    let shared = Shared::new();
    let mut routine = Routine::new(Provider::Gmail, day, 3).with_messages(5);
    let d = Drive {
        mb: &*env.host,
        client: &env.client,
        clock: &*env.clock,
        provider: Provider::Gmail,
        address_id: A,
        knowlu_address: KNOWLU,
        since: "2026-06-04".parse().unwrap(),
        shared: &shared,
        on_closed: &*env.on_closed,
        data_dir: &r.dir,
    };
    assert_eq!(drive_backfill(&d, &mut routine), Drove::Done);
    assert_eq!(routine.messages_today(), 6, "the one Gmail call is one message");
    let kept = kept.lock().unwrap().clone();
    assert!(!kept.is_empty() && kept.iter().all(|p| p.messages_today >= 5), "{kept:?}");
    // The final look's note (after the send, before Sent is cleaned) already carries the pass's message.
    assert!(kept.iter().any(|p| p.messages_today == 6), "{kept:?}");
}

#[test]
fn the_days_page_loads_kept_in_mail_json_are_never_lowered_by_a_progress_note() {
    // A progress note knows the sends but not the loads (it writes none): a same-day pace merges by the larger
    // of each count, so the 50 loads a relaunch carried in stay while the pass sends, and its own are added.
    let r = open_backfill_rig("loads-merge", 2);
    let day = NOW.div_euclid(DAY);
    MailState::update(&r.dir, |s| {
        s.backfill_pace.insert(A.to_string(), BackfillPace { day, sent_today: 3, paused_until: None, loads_today: 50, messages_today: 0 });
        true
    })
    .unwrap();
    let kept = Arc::new(Mutex::new(Vec::new()));
    let (k, dir) = (kept.clone(), r.dir.clone());
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "gmail/forward.js" {
            k.lock().unwrap().push(MailState::load(&dir).unwrap().backfill_pace[A].clone());
        }
    }));
    let shared = r.setups.start_resume(r.dir.clone(), vec![A.to_string()], r.env()).expect("a resume started");
    assert_eq!(wait(&shared).ended, Some("finished"));
    let kept = kept.lock().unwrap().clone();
    assert!(!kept.is_empty() && kept.iter().all(|p| p.loads_today >= 50 && p.sent_today >= 3), "{kept:?}");
    // The pass kept no pace once done (the backfill closed), but while it ran the loads only grew.
}

#[test]
fn a_bounced_backfill_pass_keeps_its_frontier_at_or_before_what_it_sent_and_the_done_backfill_drops_it() {
    // Quinn 2026-10-19: a backfill pass searches from its frontier, the day before the oldest mail a pass found
    // missing, sent mail included, so a batch a report stopped is searched and sent again. It only moves forward:
    // here it already stands on the mail's own day, and the bounced pass (whose rows are on that day) leaves it.
    let r = open_backfill_rig("frontier-bounce", 3);
    let on = (NOW - 5 * DAY).div_euclid(DAY);
    MailState::update(&r.dir, |s| s.backfill_frontier.insert(A.to_string(), on).is_none()).unwrap();
    queue(&r, "gmail/bounce.js", vec![json!({ "result": "refused", "reason": "non-delivery report" }), json!({ "result": "ok", "rows": 0 })]);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (s, dir) = (seen.clone(), r.dir.clone());
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "gmail/search.js" {
            s.lock().unwrap().push(MailState::load(&dir).unwrap().backfill_frontier.get(A).copied());
        }
    }));
    let shared = r.setups.start_resume(r.dir.clone(), vec![A.to_string()], r.env()).expect("a resume started");
    assert_eq!(wait(&shared).ended, Some("finished"));
    // The bounced pass sent its three, and the next day's pass searched from the same day and sent them again.
    assert_eq!(r.calls("gmail/forward.js").len(), 2, "one Gmail call a pass");
    let seen = seen.lock().unwrap().clone();
    assert!(seen.len() >= 2 && seen.iter().all(|f| *f == Some(on)), "{seen:?}");
    let day_of = |d: i64| jiff::Timestamp::from_second(d * DAY).unwrap().to_string()[..10].to_string();
    assert!(r.calls("gmail/search.js").iter().all(|a| a["since"] == json!(day_of(on))), "{:?}", r.calls("gmail/search.js"));
    // Done: the open marker and the frontier went together.
    let st = MailState::load(&r.dir).unwrap();
    assert!(st.open_backfills.is_empty() && st.backfill_frontier.is_empty(), "{st:?}");
}

/// One run of a Gmail mailbox's backfill driver over the Fake, from `mail.json` in `dir`, on the window from UTC
/// day `first`, at the Fake's clock; `on_closed` is told each closed batch.
fn drive_fake(mb: &Fake, svc: &Service, dir: &Path, first: i64, on_closed: &dyn Fn()) -> Drove {
    let shared = Shared::new();
    let client = client(svc);
    let since: jiff::civil::Date = iso(first * DAY)[..10].parse().unwrap();
    let d = Drive {
        mb,
        client: &client,
        clock: &*mb.clk,
        provider: Provider::Gmail,
        address_id: A,
        knowlu_address: KNOWLU,
        since,
        shared: &shared,
        on_closed,
        data_dir: dir,
    };
    let mut r = Routine::new(Provider::Gmail, mb.clk.now().div_euclid(DAY), 0);
    drive_backfill(&d, &mut r)
}

/// A mailbox whose only missing mail is one lone row, "Stuck", two days into a ten-day window; four rows after it
/// are held. Returns the rows, the service and the row's instant (a whole minute).
fn one_stuck_row(first: i64) -> (Vec<Mail>, Service, i64) {
    let stuck_at = (first + 2) * DAY + 3_600;
    let others: Vec<Mail> = (0..4).map(|i| mail(BO, &format!("Held {i}"), (first + 3 + i) * DAY + 600)).collect();
    let held: Vec<(&str, &str, i64)> = others.iter().map(|m| (m.sender.as_str(), m.subject.as_str(), m.at)).collect();
    let svc = holding(&held);
    let mut msgs = others.clone();
    msgs.insert(0, mail(ANA, "Stuck", stuck_at));
    (msgs, svc, stuck_at)
}

#[test]
fn a_lone_row_that_never_attaches_stalls_two_days_and_is_left_on_the_third_and_the_backfill_completes() {
    // Review of c8806b3: a lone row that never attaches, with nothing sent around it, ended every pass stalled and
    // held the frontier on it for good. mail.json counts the separate UTC days a pass ended stalled on it alone (its
    // received minute only); from the third, the row is left and the backfill goes on.
    let dir = temp("lone-stall-three-days");
    let first = NOW.div_euclid(DAY) - 10;
    let (msgs, svc, stuck_at) = one_stuck_row(first);
    let stall = || MailState::load(&dir).unwrap().lone_stall.get(A).copied();
    let today = NOW.div_euclid(DAY);
    // Day 1, twice (a second run the same day counts no new day), then day 2.
    for (at, days, last_day) in [(NOW, 1, today), (NOW + 3_600, 1, today), (NOW + DAY, 2, today + 1)] {
        let mut mb = fake(Provider::Gmail, msgs.clone());
        mb.no_chip = Some("Stuck");
        mb.clk.0.set(at);
        assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Failed("the attachments would not attach"), "{at}");
        assert_eq!(stall(), Some(LoneStall { at: stuck_at, days, last_day }), "{at}");
        assert_eq!(mb.forwards.borrow().len(), 1, "{at}: the row was tried once");
    }
    // Day 3: tried once more, and left at once when it will not attach. The backfill is done, and closing it
    // drops the record with the open marker and the frontier.
    let mut mb = fake(Provider::Gmail, msgs);
    mb.no_chip = Some("Stuck");
    mb.clk.0.set(NOW + 2 * DAY);
    assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Done);
    assert_eq!(mb.forwards.borrow().len(), 1);
    let st = MailState::load(&dir).unwrap();
    assert!(st.open_backfills.is_empty() && st.lone_stall.is_empty() && st.backfill_frontier.is_empty(), "{st:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_stalled_lone_row_that_attaches_later_clears_its_record_when_it_is_sent() {
    let dir = temp("lone-stall-cleared");
    let first = NOW.div_euclid(DAY) - 10;
    let (msgs, svc, stuck_at) = one_stuck_row(first);
    let stall = || MailState::load(&dir).unwrap().lone_stall.get(A).copied();
    let mut mb = fake(Provider::Gmail, msgs.clone());
    mb.no_chip = Some("Stuck");
    assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Failed("the attachments would not attach"));
    assert_eq!(stall(), Some(LoneStall { at: stuck_at, days: 1, last_day: NOW.div_euclid(DAY) }));
    // The next day it attaches: the record is gone as soon as the pass that sent it ends (the batch's close is told
    // after mail.json is written, before the backfill itself closes).
    let mb = fake(Provider::Gmail, msgs);
    mb.clk.0.set(NOW + DAY);
    let at_close = RefCell::new(Vec::new());
    let probe = || at_close.borrow_mut().push(MailState::load(&dir).unwrap());
    assert_eq!(drive_fake(&mb, &svc, &dir, first, &probe), Drove::Done);
    assert_eq!(mb.forwarded(), [received(stuck_at)]);
    let at_close = at_close.borrow();
    assert!(at_close.len() == 1 && at_close[0].open_backfills == [A] && at_close[0].lone_stall.is_empty(), "{at_close:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_held_row_in_the_stuck_minute_does_not_clear_the_record_of_a_pass_that_stalled_on_that_minute() {
    // Review round 1 of e7ae803, #2: a held row received in the stuck row's minute (a notification burst) cleared
    // the record on every pass, even one that ended stalled on that minute, so the day count went 1, cleared, 1 and
    // never reached three. A pass that stalled on the minute counts the day; the row is left on the third.
    let dir = temp("lone-stall-burst");
    let first = NOW.div_euclid(DAY) - 10;
    let (mut msgs, _, stuck_at) = one_stuck_row(first);
    msgs.insert(1, mail(BO, "Burst", stuck_at + 30));
    let mut held: Vec<(&str, &str, i64)> = msgs[1..].iter().map(|m| (m.sender.as_str(), m.subject.as_str(), m.at)).collect();
    held.sort_by_key(|h| h.2);
    let svc = holding(&held);
    let stall = || MailState::load(&dir).unwrap().lone_stall.get(A).copied();
    for (at, days) in [(NOW, 1), (NOW + DAY, 2)] {
        let mut mb = fake(Provider::Gmail, msgs.clone());
        mb.no_chip = Some("Stuck");
        mb.clk.0.set(at);
        assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Failed("the attachments would not attach"), "{at}");
        assert_eq!(stall(), Some(LoneStall { at: stuck_at, days, last_day: at.div_euclid(DAY) }), "{at}");
    }
    let mut mb = fake(Provider::Gmail, msgs);
    mb.no_chip = Some("Stuck");
    mb.clk.0.set(NOW + 2 * DAY);
    assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Done);
    assert_eq!(mb.forwards.borrow().len(), 1, "tried once more, then left");
    assert!(MailState::load(&dir).unwrap().lone_stall.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two lone rows side by side that never attach, "Stuck A" and "Stuck B" a minute apart two days into a ten-day
/// window, then `newer` unheld rows that must be sent, then the held rows of [`one_stuck_row`]. Returns the rows,
/// the service and the two instants.
fn two_stuck_rows(first: i64, newer: i64) -> (Vec<Mail>, Service, i64, i64) {
    let a = (first + 2) * DAY + 3_600;
    let others: Vec<Mail> = (0..4).map(|i| mail(BO, &format!("Held {i}"), (first + 3 + i) * DAY + 600)).collect();
    let held: Vec<(&str, &str, i64)> = others.iter().map(|m| (m.sender.as_str(), m.subject.as_str(), m.at)).collect();
    let svc = holding(&held);
    let mut msgs = vec![mail(ANA, "Stuck A", a), mail(ANA, "Stuck B", a + 60)];
    msgs.extend((0..newer).map(|i| mail(BO, &format!("New {i}"), a + 600 + i * 60)));
    msgs.extend(others);
    (msgs, svc, a, a + 60)
}

/// Whether a forward of `mb` carried only rows whose subject starts "New" (the newer rows, sent together).
fn sent_the_newer_rows(mb: &Fake) -> bool {
    mb.forwards.borrow().iter().any(|(_, a)| a["rows"].as_array().is_some_and(|r| r.iter().all(|r| r["subject"].as_str().is_some_and(|s| s.starts_with("New")))))
}

#[test]
fn two_lone_rows_side_by_side_that_never_attach_count_their_days_and_the_newer_mail_goes_on_the_third() {
    // Review round 2 of e7ae803, #1: the second of two lone rows that never attach stopped the pass through the
    // breaker, which returned before the stalled minute was set, so no day was ever counted and every pass stopped
    // at the same place: the newer mail never went. The first waiting row's minute counts the day; on the third
    // it is left at once, the second waits, and the newer rows' send shows the page works, so it is left too.
    let dir = temp("lone-stall-two-side-by-side");
    let first = NOW.div_euclid(DAY) - 10;
    let (msgs, svc, a, _) = two_stuck_rows(first, 3);
    let stall = || MailState::load(&dir).unwrap().lone_stall.get(A).copied();
    for (at, days) in [(NOW, 1), (NOW + DAY, 2)] {
        let mut mb = fake(Provider::Gmail, msgs.clone());
        mb.no_chips = vec!["Stuck A", "Stuck B"];
        mb.clk.0.set(at);
        assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Failed("the attachments would not attach"), "{at}");
        assert_eq!(stall(), Some(LoneStall { at: a, days, last_day: at.div_euclid(DAY) }), "{at}");
        assert!(!sent_the_newer_rows(&mb), "{at}");
    }
    let mut mb = fake(Provider::Gmail, msgs);
    mb.no_chips = vec!["Stuck A", "Stuck B"];
    mb.clk.0.set(NOW + 2 * DAY);
    assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Done);
    assert!(sent_the_newer_rows(&mb), "the newer mail went");
    let st = MailState::load(&dir).unwrap();
    assert!(st.open_backfills.is_empty() && st.lone_stall.is_empty() && st.lone_left.is_empty(), "{st:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_row_left_at_once_stays_left_while_the_next_stuck_row_counts_its_own_days() {
    // Review round 2 of e7ae803, #1: with nothing newer to send, the second stuck row stalls the pass on the day the
    // first is left at once. One record a mailbox let the two replace each other for good (A, then B, then A, each
    // at one day). A row left at once is kept in mail.json's `lone_left` until the backfill closes, so it stays
    // left while the next row counts its own three days, and the backfill completes.
    let dir = temp("lone-stall-left-stays-left");
    let first = NOW.div_euclid(DAY) - 10;
    let (msgs, svc, a, b) = two_stuck_rows(first, 0);
    let st = || MailState::load(&dir).unwrap();
    let want = [(a, 1, vec![]), (a, 2, vec![]), (b, 1, vec![a]), (b, 2, vec![a])];
    for (d, (at, days, left)) in want.into_iter().enumerate() {
        let now = NOW + d as i64 * DAY;
        let mut mb = fake(Provider::Gmail, msgs.clone());
        mb.no_chips = vec!["Stuck A", "Stuck B"];
        mb.clk.0.set(now);
        assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Failed("the attachments would not attach"), "day {d}");
        assert_eq!(st().lone_stall.get(A).copied(), Some(LoneStall { at, days, last_day: now.div_euclid(DAY) }), "day {d}");
        assert_eq!(st().lone_left.get(A).cloned().unwrap_or_default(), left, "day {d}");
    }
    let mut mb = fake(Provider::Gmail, msgs);
    mb.no_chips = vec!["Stuck A", "Stuck B"];
    mb.clk.0.set(NOW + 4 * DAY);
    assert_eq!(drive_fake(&mb, &svc, &dir, first, &|| {}), Drove::Done);
    assert!(st().lone_stall.is_empty() && st().lone_left.is_empty(), "{:?}", st());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_backfill_held_by_gmails_300_messages_resumes_when_30_have_freed_not_1() {
    // Review of 5f71623..6a8e738, #1: at the cap the wait ended when the oldest kept message was a day old, which
    // frees one, so each pass paid a fresh Inbox, a search and its walk to send about one. The 300 went one a minute,
    // ending a little under a day ago; the relaunch at NOW sends nothing until the 30th oldest is a day old.
    let r = open_backfill_rig("messages-batch-resume", 2);
    let oldest = NOW - DAY + 600;
    let times: Vec<i64> = (0..300).map(|i| oldest + i * 60).collect();
    MailState::update(&r.dir, |s| s.gmail_messages.insert(A.to_string(), times.clone()).is_none()).unwrap();
    let thirtieth = times[29] + DAY;
    let forwards = resume_and_time_forwards(&r);
    assert!(!forwards.is_empty() && forwards.iter().all(|t| *t >= thirtieth), "{forwards:?} vs {thirtieth} (the oldest frees at {})", oldest + DAY);
    assert!(forwards[0] < thirtieth + 120, "and no later than that: {forwards:?} vs {thirtieth}");
}

#[test]
fn a_2000_message_backfill_whose_every_send_is_halved_to_one_finishes_in_a_few_passes_a_day() {
    // Review of 5f71623..6a8e738, #1: with every send halved to one original, the day's 300 messages are the limit.
    // Once they were spent, each pass waited for one to free and paid its fresh Inbox, search and walk for about one
    // send: some 60 passes a day, past MAX_PASSES (400) in about a week, so the backfill failed undone. A pass now
    // starts once 30 have freed. The service holds what went, so each pass forwards only what is still missing.
    let dir = temp("messages-batch-halved");
    let first = NOW.div_euclid(DAY) - 120;
    let msgs: Vec<Mail> = (0..2_000).map(|i| mail(ANA, &format!("Notice {i}"), first * DAY + 3_600 + i * 5_000)).collect();
    let learned = Arc::new(Mutex::new(Vec::new()));
    let svc = learning(learned.clone());
    let mut mb = fake(Provider::Gmail, msgs);
    mb.over_rows = Some(1);
    mb.learn = Some(learned.clone());
    // Each pass closes its batch: the clock and the page loads so far at each close.
    let ends = RefCell::new(Vec::new());
    let probe = || ends.borrow_mut().push((mb.clk.now(), mb.loads()));
    assert_eq!(drive_fake(&mb, &svc, &dir, first, &probe), Drove::Done);
    let ends = ends.into_inner();
    let mut by_day: std::collections::BTreeMap<i64, (u32, u64)> = std::collections::BTreeMap::new();
    let mut before = 0;
    for (at, loads) in &ends {
        let d = by_day.entry(at.div_euclid(DAY)).or_default();
        d.0 += 1;
        d.1 += loads - before;
        before = *loads;
    }
    let days = by_day.len() as u32;
    let passes = ends.len() as u32;
    // Measured (review of 5f71623..6a8e738): 14 passes over 7 days, at most 8 and 165 page loads on any day; before
    // the fix, 120 passes over 8 days, up to 33 a day, with the day's 600 loads spent on four of them.
    assert!((7..=8).contains(&days), "300 messages a day: {by_day:?}");
    assert!(passes <= 20, "far under MAX_PASSES: {passes}, {by_day:?}");
    assert!(by_day.values().all(|(n, loads)| *n <= 10 && *loads <= 200), "{by_day:?}");
    // Every original went once, alone.
    let fwd = mb.forwards.borrow();
    let mut once: Vec<String> = fwd.iter().filter(|(_, a)| a["rows"].as_array().unwrap().len() == 1).flat_map(|(_, a)| row_texts(a)).collect();
    assert_eq!(once.len(), 2_000);
    once.sort();
    once.dedup();
    assert_eq!((once.len(), learned.lock().unwrap().len()), (2_000, 2_000), "each original went once");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_gmail_message_sent_is_in_mail_json_before_the_next_send() {
    // Review round 1 of e7ae803, #3 (b): the day's Gmail messages reached mail.json only at a look and at the pass's
    // end, and each progress note wrote 0, so an app killed between looks relaunched up to 24 messages short. Three
    // conversations are three sends: each finds the ones before it kept.
    let dir = temp("messages-each-send");
    let mut mb = fake(Provider::Gmail, convos(3, 2));
    mb.pace_dir = Some(dir.clone());
    assert_eq!(drive_fake(&mb, &holding(&[]), &dir, NOW.div_euclid(DAY) - 10, &|| {}), Drove::Done);
    assert_eq!(*mb.paces.borrow(), [0, 1, 2]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_gmail_send_that_did_not_settle_keeps_its_message_counted() {
    // Review round 1 of e7ae803, #3 (a): "send did not settle" comes after Send was pressed, so the message may
    // have gone; its reservation is kept, never given back.
    let since = window(NOW, None, None).since;
    let mut mb = fake(Provider::Gmail, vec![mail(ANA, "Quiz 3 moved", since + 600)]);
    mb.attach_fails = Some("send did not settle");
    let mut r = routine(Provider::Gmail);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent, r.messages_today()), (Stop::Script("a forward did not finish"), 0, 1));
    // Pre-PR review 2026-10-20: the originals of a send that may have gone stay charged too.
    assert_eq!(r.sent_today(), 1);
}

#[test]
fn a_gmail_message_cap_stop_gives_back_the_originals_no_send_went_for() {
    // Pre-PR review 2026-10-20: the stop comes before any Send, so the originals taken for the group are given back.
    let since = window(NOW, None, None).since;
    let recent: Vec<i64> = (0..300).map(|i| NOW - 3_600 + i).collect();
    // Every message spent: the first reservation fails, and nothing is charged.
    let mb = fake(Provider::Gmail, vec![mail(ANA, "Quiz 3 moved", since + 600)]);
    let mut r = routine(Provider::Gmail).with_message_times(&recent);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent, r.sent_today()), (Stop::DayLimit { next_day_at: recent[0] + DAY }, 0, 0));
    assert_eq!(mb.forwards.borrow().len(), 0);
    // One message left: a pair is refused as too big, the first half takes it, and the second half finds the cap
    // spent. The two originals taken were one sent and one not: only the sent one stays charged.
    let mut mb = fake(Provider::Gmail, vec![mail(ANA, "Quiz 3 moved", since + 600), mail(BO, "Lab report due", since + 1_200)]);
    mb.over_rows = Some(1);
    let mut r = routine(Provider::Gmail).with_message_times(&recent[1..]);
    let out = pass(&mb, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop.clone(), out.sent), (Stop::DayLimit { next_day_at: recent[1] + DAY }, 1));
    assert_eq!(r.sent_today(), 1);
}

#[test]
fn a_gmail_recipient_not_accepted_is_tried_once_more_and_each_miss_gives_its_message_back() {
    // Live (ef28, 2026-10-19): a page's first forward answered "recipient not accepted" with none drawn, and the
    // whole backfill pass stopped. forward.js discards that draft before Send, so nothing went: the same forward is
    // tried once more, its Gmail message reserved again, and the miss gives its own back.
    let since = window(NOW, None, None).since;
    let two = || vec![mail(ANA, "Quiz 3 moved", since + 600), mail(BO, "Lab report due", since + 1_200)];
    let once = fake(Provider::Gmail, two());
    once.recipient_misses.set(1);
    let mut r = routine(Provider::Gmail);
    let out = pass(&once, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent, out.left, r.messages_today()), (Stop::Done, 2, 0, 1));
    let fwd = once.forwards.borrow();
    assert_eq!(fwd.len(), 2, "the missed forward, then the same one once more");
    assert_eq!(fwd[0].1, fwd[1].1, "the same rows and args");
    // Missed on its second try too: the pass stops naming it (review of ae3d428: its own word, not the catch-all),
    // with nothing sent, left or counted.
    let twice = fake(Provider::Gmail, two());
    twice.recipient_misses.set(2);
    let mut r = routine(Provider::Gmail);
    let out = pass(&twice, &holding(&[]), &mut r, &gmail_backfill());
    let refused = Stop::Script("the forward's address was not accepted");
    assert_eq!((out.stop, out.sent, out.left, r.messages_today()), (refused.clone(), 0, 0, 0));
    assert_eq!(twice.forwards.borrow().len(), 2, "one more try, never a third");
    // Review of ae3d428 (M1): another address drawn is never a miss to try again, here as in forward.js. The
    // pass stops at once, after one forward, with its message given back.
    let mut other = fake(Provider::Gmail, two());
    other.recipient_misses.set(1);
    other.other_recipient = true;
    let mut r = routine(Provider::Gmail);
    let out = pass(&other, &holding(&[]), &mut r, &gmail_backfill());
    assert_eq!((out.stop, out.sent, out.left, r.messages_today()), (refused, 0, 0, 0));
    assert_eq!(other.forwards.borrow().len(), 1, "no second forward over another recipient");
}

#[test]
fn a_mail_json_written_before_the_page_budget_reads_and_writes_back_byte_for_byte() {
    // mail.json is a contract with existing installs: a file with a pace and no loads, and no frontier, is read
    // and written again unchanged (the new fields are not written while 0 or empty).
    let r = rig("loads-old-file");
    let old = json!({
        "keep_session": true, "last_reconcile": {}, "pending_deletes": [], "turned_off": [], "open_backfills": [A],
        "backfill_pace": { A: { "day": 20_000, "sent_today": 41, "paused_until": null } }, "last_look": { A: NOW },
    });
    let text = knowlu_engine::ledger::dumps_value(&old);
    std::fs::write(r.dir.join("mail.json"), &text).unwrap();
    let st = MailState::update(&r.dir, |_| true).unwrap();
    assert_eq!((st.backfill_pace[A].loads_today, st.backfill_frontier.len()), (0, 0));
    assert_eq!(std::fs::read_to_string(r.dir.join("mail.json")).unwrap(), text);
}

#[test]
fn the_pace_is_kept_beside_the_open_marker_and_costs_nothing_when_empty() {
    let r = open_backfill_rig("t113-pace-file", 3);
    let mut routine = Routine::new(Provider::M365, NOW.div_euclid(DAY), 599);
    quit_after_first_forward(&r, Provider::M365, &mut routine);
    let pace = MailState::load(&r.dir).unwrap().backfill_pace;
    assert_eq!(pace.get(A), Some(&BackfillPace { day: NOW.div_euclid(DAY), sent_today: 600, paused_until: None, loads_today: 0, messages_today: 0 }));
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
    // Gmail: Microsoft 365's backfill is held while C3 is built (D13) and is never failed or retried.
    let r = rig("t113-failed-backfill");
    {
        let mut h = r.host.lock().unwrap();
        h.title_address = Some(HINT_PAGE.to_string());
        h.msgs = ana_mail(2);
    }
    let ok = json!({ "result": "ok", "page": { "from": 1, "to": 2, "total": 2 }, "moved": false, "rows": SHost::rows(&r.host.lock().unwrap()) });
    queue(&r, "gmail/list.js", vec![json!({ "result": "not found", "control": "list" }), ok]);
    let shared = r.start(Kind::Gmail, true);
    until("the backfill failed", || has(&shared, "backfill", "failed"));
    assert_eq!(MailState::load(&r.dir).unwrap().open_backfills, [A], "still open, so the next launch resumes it");
    assert!(shared.answer(Answer::Retry));
    let p = wait(&shared);
    assert_eq!((p.ended, row(&p, "backfill").state), (Some("finished"), "done"));
    assert!(p.not_brought_in_before.is_none());
}

/// The Microsoft DOM backfill scripts C3 retires: while it is built (D13) none of them runs, for any reason.
const RETIRED: [&str; 4] = ["m365/search.js", "m365/list.js", "m365/forward.js", "m365/sent_cleanup.js"];

fn ran_retired(r: &Rig) -> Vec<String> {
    r.scripts().into_iter().filter(|s| RETIRED.contains(&s.as_str())).collect()
}

#[test]
fn a_microsoft_setup_ends_at_a_guided_backfill_and_runs_none_of_the_retired_scripts() {
    // D13 (Quinn, 2026-10-11): forwarding and the canary run as script; the backfill is the guided step.
    let r = rig("d13-setup");
    r.host.lock().unwrap().msgs = ana_mail(2);
    let shared = r.start(curated_m365(), true);
    until("the backfill step", || has(&shared, "backfill", "needs_you"));
    let p = shared.progress();
    assert!(row(&p, "backfill").guided, "shown as the guided step");
    for step in ["sign_in", "step_aside", "forwarding", "canary"] {
        assert_eq!(row(&p, step).state, "done", "{step}");
    }
    assert!(shared.answer(Answer::Done));
    let p = wait(&shared);
    assert_eq!(p.not_brought_in_before, Some(since_day()), "the end names the day before which mail was not brought in");
    assert!(p.done.contains(&"canary") && !p.done.contains(&"backfill") && !p.left.contains(&"backfill"), "{:?} {:?}", p.done, p.left);
    assert_ne!(row(&p, "backfill").state, "done_by_hand");
    assert!(ran_retired(&r).is_empty(), "{:?}", r.scripts());
    assert!(!r.events().iter().any(|e| e == "svc batch"), "no backfill batch was opened: {:?}", r.events());
    assert!(MailState::load(&r.dir).unwrap().open_backfills.is_empty(), "no backfill was begun");
    assert_eq!(r.closed.load(Ordering::SeqCst), 0);
}

#[test]
fn a_launch_skips_a_microsoft_open_backfill_silently_and_leaves_its_marker_for_c3() {
    // Quinn 2026-10-19: Knowlu stops trying to resume a Microsoft 365 backfill it cannot run yet. The launch
    // picks no Microsoft mailbox to resume, so no run exists: nothing failed, no line, no amber. The marker
    // stays in mail.json for C3, and Gmail's open backfill is still picked.
    let r = open_backfill_rig("d13-launch-skip", 2);
    r.sv.lock().unwrap().provider = "m365".into();
    let env = r.env();
    let picked = launch_resumable(open_backfills(&r.dir), &env.client);
    assert!(picked.is_empty(), "{picked:?}");
    assert!(r.setups.current().is_none(), "no run, so nothing for the view or the tray to show");
    assert_eq!(open_backfills(&r.dir), [A], "the marker stays for C3");
    assert!(r.host.lock().unwrap().opens.is_empty() && r.scripts().is_empty());
    let g = open_backfill_rig("d13-launch-skip-gmail", 2);
    let genv = g.env();
    assert_eq!(launch_resumable(open_backfills(&g.dir), &genv.client), [A]);
    // Even if a Microsoft id reached the resume itself, it is skipped with no failure and no window.
    let shared = r.setups.start_resume(r.dir.clone(), vec![A.to_string()], r.env()).expect("a resume started");
    let p = wait(&shared);
    assert_ne!(p.ended, Some("failed"), "{:?}", p.line);
    assert_ne!(p.line, Some("this mailbox is set up by hand"));
    assert!(r.host.lock().unwrap().opens.is_empty(), "no window");
    assert!(r.scripts().is_empty() && r.sv.lock().unwrap().batches.is_empty());
    assert_eq!(open_backfills(&r.dir), [A], "left open so C3 can resume it");
    assert_eq!(r.closed.load(Ordering::SeqCst), 0);
}

#[test]
fn a_resume_whose_every_mailbox_is_held_for_c3_never_says_the_backfill_finished() {
    // Review round 2 of e7ae803, #3: a launch that kept a Microsoft 365 id because the service could not be asked,
    // whose resume then listed it and skipped it, ended "finished" / "backfill finished" though nothing ran. A
    // resume that only skipped ends with no outcome and no line, and shows no backfill row.
    let r = open_backfill_rig("d13-resume-all-held", 2);
    r.sv.lock().unwrap().provider = "m365".into();
    let shared = r.setups.start_resume(r.dir.clone(), vec![A.to_string()], r.env()).expect("a resume started");
    let p = wait(&shared);
    assert_eq!((p.ended, p.line), (None, None));
    assert!(p.rows.is_empty(), "{:?}", p.rows);
    assert_eq!(open_backfills(&r.dir), [A], "left open so C3 can resume it");
}

#[test]
fn a_microsoft_setup_whose_forwarding_worked_ends_as_a_success_not_a_stop() {
    // Quinn 2026-10-19 (the M365 copy ruling): the guided backfill is held for C3, and ending there by hand
    // is the designed end. Forwarding is set and the canary arrived, so the line says so and the state is
    // `finished`; the console shows a success. What was not brought in is still named.
    let r = rig("m365-forwarding-set-end");
    r.host.lock().unwrap().msgs = ana_mail(2);
    let shared = r.start(curated_m365(), true);
    until("the backfill step", || has(&shared, "backfill", "needs_you"));
    assert!(shared.answer(Answer::Done));
    let p = wait(&shared);
    assert_eq!((p.ended, p.line), (Some("finished"), Some("Forwarding is set. New mail is arriving.")));
    assert!(p.not_brought_in_before.is_some(), "what was not brought in is still named");
    // The held backfill and the Finish that never ran are not "Not done": the end reads as a success.
    assert!(p.left.is_empty(), "{:?}", p.left);
    assert!(!r.events().contains(&"svc delete address".to_string()), "the address is kept");
    // The student's own Stop is still a stop.
    let s = rig("m365-forwarding-set-stop");
    s.host.lock().unwrap().msgs = ana_mail(2);
    let shared = s.start(curated_m365(), true);
    until("the backfill step", || has(&shared, "backfill", "needs_you"));
    shared.stop();
    let p = wait(&shared);
    assert_eq!((p.ended, p.line), (Some("stopped"), Some("setup stopped")));
    assert!(p.left.contains(&"backfill") && p.left.contains(&"finish"), "a stop still lists what was left: {:?}", p.left);
}

#[test]
fn an_open_gmail_backfill_still_resumes_at_launch() {
    let r = open_backfill_rig("d13-resume-gmail", 2);
    let shared = r.setups.start_resume(r.dir.clone(), vec![A.to_string()], r.env()).expect("a resume started");
    let p = wait(&shared);
    assert_eq!(p.ended, Some("finished"), "{:?}", p.line);
    assert_eq!(r.host.lock().unwrap().opens, [(session_dir(&r.dir, A).unwrap(), false)], "hidden, on the kept profile");
    assert_eq!(r.sv.lock().unwrap().batches, ["2026-06-04"]);
    assert!(!r.calls("gmail/forward.js").is_empty());
    assert!(ran_retired(&r).is_empty());
    assert!(open_backfills(&r.dir).is_empty());
}

fn position(ev: &[String], e: &str) -> usize {
    ev.iter().position(|x| x == e).unwrap_or_else(|| panic!("no {e}: {ev:?}"))
}

#[test]
fn a_school_microsoft_mailbox_sets_up_in_order() {
    // D13: forwarding and the canary run as script; the backfill is the guided step, "I've done this" ends the
    // setup, and none of the retired scripts runs.
    let r = rig("t113-m365");
    r.host.lock().unwrap().msgs = ana_mail(2);
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!(p.ended, Some("finished"));
    assert_eq!(p.line, Some("Forwarding is set. New mail is arriving."));
    for step in ["sign_in", "step_aside", "forwarding", "canary"] {
        assert_eq!(row(&p, step).state, "done", "{step}");
    }
    assert_eq!((row(&p, "backfill").state, row(&p, "finish").state), ("skipped", "skipped"));
    assert_eq!(p.not_brought_in_before, Some(since_day()));
    assert!(ran_retired(&r).is_empty(), "{:?}", r.scripts());
    let ev = r.events();
    let at = |e: &str| position(&ev, e);
    assert!(at("svc mint") < at("window visible") && at("window visible") < at("script m365/signin.js"));
    assert!(at("script m365/signin.js") < at("svc hint") && at("svc hint") < at("aside"));
    assert!(at("aside") < at("script m365/forwarding_set.js"));
    assert!(at("script m365/forwarding_set.js") < at("script m365/forwarding_read.js"), "read back after navigating away and back");
    assert!(at("script m365/forwarding_read.js") < at("svc canary"));
    assert!(!ev.iter().any(|e| e == "svc batch" || e == "slot now"), "no backfill batch, so no slot: {ev:?}");
    let set = &r.calls("m365/forwarding_set.js")[0];
    assert_eq!((set["address"].clone(), set["replace"].clone()), (json!(KNOWLU), json!(false)));
    let places: Vec<&String> = ev.iter().filter(|e| e.starts_with("open ")).collect();
    assert_eq!(places[..3], ["open Forwarding", "open Inbox", "open Forwarding"], "away and back: {places:?}");
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
    // The read-back of a Verify row (a popup that confirmed with no dialog) needs the address to look for.
    let done = r.calls("gmail/forwarding_add_done.js");
    assert_eq!(done.len(), 1);
    assert_eq!(done[0]["address"], json!(KNOWLU), "forwarding_add_done.js is told the Knowlu address");
}

fn queue(r: &Rig, script: &str, answers: Vec<Value>) {
    r.host.lock().unwrap().queued.insert(script.to_string(), answers.into());
}

/// `m365/signin.js` on an Outlook page whose mail list is drawn and whose folder pane is not yet.
fn address_not_drawn() -> Value {
    json!({ "result": "not found", "control": "mailbox address" })
}

#[test]
fn a_microsoft_folder_pane_drawn_after_the_title_still_signs_in() {
    // T18c (2026-10-05): the title reads signed in a second or two before Outlook draws its folder pane,
    // so the first reads find no address. The step waits for the page instead of ending on the first.
    let r = rig("t18c-late-pane");
    r.host.lock().unwrap().msgs = ana_mail(1);
    let mut answers = vec![address_not_drawn(); 20];
    answers.push(json!({ "result": "ok", "signed_in": true, "mailbox_hint": HINT_PAGE }));
    queue(&r, "m365/signin.js", answers);
    let p = wait(&r.start(curated_m365(), true));
    // D13: a Microsoft setup ends at the guided backfill step, which closes as the student's "I've done this".
    assert_eq!((p.ended, p.line), (Some("finished"), Some("Forwarding is set. New mail is arriving.")));
    assert_eq!(row(&p, "sign_in").state, "done");
    assert_eq!(r.calls("m365/signin.js").len(), 21, "read again until the address was drawn");
    assert_eq!(r.sv.lock().unwrap().hint.as_deref(), Some(HINT_PAGE));
    assert!(!r.events().contains(&"svc delete address".to_string()));
}

#[test]
fn a_microsoft_address_never_drawn_fails_sign_in_naming_why_within_a_bound() {
    let r = rig("t18c-no-address");
    queue(&r, "m365/signin.js", vec![address_not_drawn()]);
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!((p.status, p.ended, p.line), ("ended", Some("failed"), Some("sign-in")));
    let s = row(&p, "sign_in");
    assert_eq!((s.state, s.reason, s.guided), ("failed", Some("not_found"), false), "the failure is on its own row");
    for step in ["step_aside", "forwarding", "canary", "backfill", "finish"] {
        assert_eq!((row(&p, step).state, row(&p, step).reason), ("skipped", None), "{step}");
    }
    assert!(p.left.contains(&"sign_in") && p.done.is_empty());
    // Bounded: well past the page's drawing time, far short of the sign-in's fifteen minutes.
    let waited = r.clk.at() - NOW;
    assert!((30..=120).contains(&waited), "waited {waited} s for the address");
    assert!(r.calls("m365/signin.js").len() > 1, "read more than once");
    let ev = r.events();
    assert!(!ev.contains(&"svc hint".to_string()) && ev.contains(&"svc delete address".to_string()), "nothing kept: {ev:?}");
    assert!(!r.scripts().iter().any(|s| s != "m365/signin.js"), "no step ran after it: {:?}", r.scripts());
}

#[test]
fn a_sign_in_that_takes_in_the_last_seconds_still_gets_its_read_and_names_why() {
    // T18c review: the title turns signed in 10 s before the fifteen-minute sign-in limit. The page still
    // gets its full read of the address, so a failure says why instead of reading as a cancelled setup.
    let r = rig("t18c-late-sign-in");
    {
        let mut h = r.host.lock().unwrap();
        h.signed_in = false;
        h.signed_in_from = Some((r.clk.clone(), NOW + 15 * 60 - 10));
    }
    queue(&r, "m365/signin.js", vec![address_not_drawn()]);
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!((p.ended, p.line), (Some("failed"), Some("sign-in")));
    let s = row(&p, "sign_in");
    assert_eq!((s.state, s.reason), ("failed", Some("not_found")));
    assert!(r.clk.at() - NOW >= 15 * 60 - 10 + 45, "the read had its 45 s");
}

#[test]
fn a_sign_in_whose_page_is_still_navigating_at_the_limit_still_gets_its_read() {
    // T18c review: the 45 s read starts when the title first reads signed in, not on the first `not found`.
    // Here the title turns 10 s before the limit and the script errs for 15 s while Outlook finishes
    // navigating (a `Wait`), past the limit; the page then reads, so the step names why, never cancelled.
    let r = rig("t18c-late-navigating");
    {
        let mut h = r.host.lock().unwrap();
        h.signed_in = false;
        h.signed_in_from = Some((r.clk.clone(), NOW + 15 * 60 - 10));
    }
    let mut answers = vec![json!({ "result": "failed", "reason": "page not ready" }); 15];
    answers.push(address_not_drawn());
    queue(&r, "m365/signin.js", answers);
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!((p.ended, p.line), (Some("failed"), Some("sign-in")));
    let s = row(&p, "sign_in");
    assert_eq!((s.state, s.reason), ("failed", Some("not_found")));
    assert!(r.calls("m365/signin.js").len() > 15, "the page was read after it stopped erring");
    let waited = r.clk.at() - NOW;
    assert!((15 * 60 - 10 + 45..=15 * 60 + 60).contains(&waited), "the read ran 45 s from the title: {waited} s");
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

/// The service's fixed sentence for a hint another of the account's mailboxes holds (`mail/handler.ts`).
const HINT_HELD: &str = "another of this account's mailboxes has this address";

#[test]
fn a_mailbox_another_setup_already_holds_fails_sign_in_as_already_connected() {
    // T18c (2026-10-05): the service refused the page's address because a stalled earlier setup held it,
    // and the view said "Signing in did not finish". It names the refusal and points at Settings.
    let r = rig("t18c-held");
    r.sv.lock().unwrap().hint_refused = Some((409, HINT_HELD.to_string()));
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!(p.ended, Some("failed"));
    let line = p.line.expect("a line");
    assert!(line.contains("already set up in Knowlu") && line.contains("Settings"), "{line}");
    let s = row(&p, "sign_in");
    assert_eq!((s.state, s.reason), ("failed", Some("already_connected")));
    let ev = r.events();
    assert!(ev.contains(&"svc delete address".to_string()), "the address minted for it is revoked: {ev:?}");
    assert!(!r.scripts().iter().any(|s| s != "m365/signin.js"), "no step ran after it: {:?}", r.scripts());

    // Any other refusal of the hint is its own reason, and the line is the service's fixed sentence.
    let other = "this address does not fit the mailbox's provider";
    let o = rig("t18c-hint-refused");
    o.sv.lock().unwrap().hint_refused = Some((422, other.to_string()));
    let p = wait(&o.start(curated_m365(), true));
    assert_eq!((p.ended, p.line), (Some("failed"), Some(other)));
    assert_eq!(row(&p, "sign_in").reason, Some("refused"));
    assert!(o.events().contains(&"svc delete address".to_string()));

    // The hint route's other 409 is a sentence the device knows too (T18c review).
    let set = "this mailbox's own address is already set";
    let a = rig("t18c-hint-set");
    a.sv.lock().unwrap().hint_refused = Some((409, set.to_string()));
    let p = wait(&a.start(curated_m365(), true));
    assert_eq!((p.ended, p.line), (Some("failed"), Some(set)));
}

#[test]
fn a_refused_mint_shows_the_services_sentence() {
    // T18c (2026-10-05): a refused `POST /addresses` showed the bare word "refused". D3's caps are fixed
    // sentences the device shows as they are (`mail/handler.ts` LIVE_CAP, MINT_CAP).
    for (status, cap) in [
        (409, "this account already has three live mail addresses"),
        (429, "this account has made six mail addresses in the last 30 days"),
    ] {
        let r = rig(&format!("t18c-cap-{status}"));
        r.sv.lock().unwrap().mint_refused = Some((status, cap.to_string()));
        let p = wait(&r.start(curated_m365(), true));
        assert_eq!((p.ended, p.sentence, p.line), (Some("failed"), cap, Some(cap)), "{status}");
        assert!(r.host.lock().unwrap().opens.is_empty(), "no window opens");
    }
    // A refusal whose sentence the device does not know is still a sentence, never a bare word.
    let r = rig("t18c-cap-other");
    r.sv.lock().unwrap().mint_refused = Some((400, "something new".to_string()));
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!(p.ended, Some("failed"));
    assert!(p.sentence.contains(' ') && p.line.is_some_and(|l| l.contains(' ')), "{p:?}");
}

#[test]
fn a_setup_that_ends_before_forwarding_was_set_removes_its_new_address() {
    // T18c (2026-10-05): a setup stopped at a forwarding step whose script changed nothing left its address
    // pending and holding the mailbox's hint, so the next setup of that mailbox was refused. Nothing was
    // set in the mailbox, so the address goes, as a closed sign-in window's does.
    // Removed: the forwarding script found nothing, and Stop came while it ran, so no guided step (the
    // address with Copy) was ever shown.
    let not_found = json!({ "result": "not found", "control": "switch" });
    let r = rig("t18c-withdrawn");
    queue(&r, "m365/forwarding_set.js", vec![not_found.clone()]);
    let setups = r.setups.clone();
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "m365/forwarding_set.js" {
            setups.current().expect("a setup is running").stop();
        }
    }));
    let p = wait(&r.start(curated_m365(), true));
    assert_eq!(p.ended, Some("stopped"));
    assert!(p.line.is_some_and(|l| l.contains("was removed")), "it says the address went: {:?}", p.line);
    assert!(r.events().contains(&"svc delete address".to_string()), "{:?}", r.events());
    assert!(!session_dir(&r.dir, A).unwrap().exists(), "its profile is gone");

    // Kept (T18c review): the failed forwarding fell to its guided step, which shows the address with Copy;
    // the student may have pasted it into the mailbox's settings without saying so.
    let s = rig("t18c-kept-shown");
    queue(&s, "m365/forwarding_set.js", vec![not_found.clone()]);
    let shared = s.start(curated_m365(), true);
    until("the guided forwarding", || has(&shared, "forwarding", "failed"));
    shared.stop();
    assert_eq!(wait(&shared).line, Some("setup stopped"));
    assert!(!s.events().contains(&"svc delete address".to_string()), "the guided step showed the address");

    // Kept: a guided provider shows the address with Copy from its first step.
    let gd = rig("t18c-kept-guided");
    gd.sv.lock().unwrap().provider_of = "other".into();
    let shared = gd.start(uncurated(), true);
    answer_address(&shared);
    until("the guided forwarding", || has(&shared, "forwarding", "needs_you"));
    shared.stop();
    assert_eq!(wait(&shared).line, Some("setup stopped"));
    assert!(!gd.events().contains(&"svc delete address".to_string()), "a guided step's address may be in use");

    // Kept: the student said they did the guided step, even though its check failed.
    let k = rig("t18c-kept-by-hand");
    k.sv.lock().unwrap().canary_arrives = false;
    queue(&k, "m365/forwarding_set.js", vec![not_found]);
    let shared = k.start(curated_m365(), true);
    until("the guided forwarding", || has(&shared, "forwarding", "failed"));
    until("I've done this is taken", || shared.answer(Answer::Done));
    until("the check", || k.events().contains(&"svc canary".to_string()));
    shared.stop();
    assert_eq!(wait(&shared).line, Some("setup stopped"));
    assert!(!k.events().contains(&"svc delete address".to_string()), "the student may have set it by hand");

    // Kept: on Gmail the address was added before turning forwarding on failed.
    let g = rig("t18c-kept-gmail");
    g.host.lock().unwrap().title_address = Some(HINT_PAGE.to_string());
    queue(&g, "gmail/forwarding_set.js", vec![json!({ "result": "not found", "control": "save" })]);
    let shared = g.start(Kind::Gmail, true);
    until("the guided turn-on", || has(&shared, "turn_on", "failed"));
    shared.stop();
    assert_eq!(wait(&shared).line, Some("setup stopped"));
    assert!(!g.events().contains(&"svc delete address".to_string()), "Google's list holds the address");

    // Removed: the student kept a forwarding that goes elsewhere.
    let n = rig("t18c-withdrawn-else");
    queue(&n, "m365/forwarding_set.js", vec![json!({ "result": "elsewhere" })]);
    let shared = n.start(curated_m365(), true);
    until("the question", || shared.progress().ask == Some("forwarding_elsewhere"));
    assert!(shared.answer(Answer::Replace(false)));
    assert_eq!(wait(&shared).ended, Some("stopped"));
    assert!(n.events().contains(&"svc delete address".to_string()));
}

#[test]
fn a_setup_whose_backfill_began_never_removes_its_address() {
    // T18c review: revoking an address purges the mail stored under it (`mail/handler.ts` purgeAddress), so a
    // Stop during a backfill must not revoke the address the mail was brought in under. Since 2026-10-06 the
    // backfill waits for the canary, so it begins only once forwarding was set. Gmail: Microsoft 365's
    // backfill is held while C3 is built (D13), so it never begins.
    let r = rig("t18c-backfilled-kept");
    {
        let mut h = r.host.lock().unwrap();
        h.title_address = Some(HINT_PAGE.to_string());
        h.msgs = ana_mail(2);
    }
    let setups = r.setups.clone();
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "gmail/forward.js" {
            setups.current().expect("a setup is running").stop();
        }
    }));
    let p = wait(&r.start(Kind::Gmail, true));
    assert_eq!((p.ended, p.line), (Some("stopped"), Some("setup stopped")));
    let ev = r.events();
    assert!(ev.contains(&"svc batch".to_string()), "the backfill began: {ev:?}");
    assert!(!ev.contains(&"svc delete address".to_string()), "the mail brought in is kept: {ev:?}");
}

#[test]
fn a_withdrawal_the_service_refuses_never_says_the_address_was_removed() {
    // T18c review: the DELETE failed (502, or offline), yet the line said the address was removed. It says
    // it could not be, and points at Settings → Email to turn it off.
    let r = rig("t18c-withdraw-refused");
    r.sv.lock().unwrap().delete_refused = true;
    queue(&r, "m365/forwarding_set.js", vec![json!({ "result": "elsewhere" })]);
    let shared = r.start(curated_m365(), true);
    until("the question", || shared.progress().ask == Some("forwarding_elsewhere"));
    assert!(shared.answer(Answer::Replace(false)));
    let p = wait(&shared);
    assert!(r.events().contains(&"svc delete address".to_string()), "the delete was asked");
    assert_eq!(p.ended, Some("stopped"));
    let line = p.line.expect("a line");
    assert!(!line.contains("was removed"), "{line}");
    assert!(line.contains("could not be removed") && line.contains("Settings") && line.contains("Email"), "{line}");
}

#[test]
fn a_forwarding_set_by_hand_holds_the_backfill_until_the_canary_arrives() {
    // Quinn, 2026-10-06: the backfill ran while forwarding had failed (`requires(Backfill)` was empty). It
    // waits for the canary, and Finish waits for it, so D16's guided path keeps its order and its window:
    // forwarding by hand, "I've done this", the canary, then the backfill. D13: Microsoft's backfill is the
    // guided step, shown once the canary is done and holding the window open until the student closes it.
    let r = rig("t1006-held-backfill");
    r.host.lock().unwrap().msgs = ana_mail(2);
    queue(&r, "m365/forwarding_set.js", vec![json!({ "result": "not found", "control": "switch" })]);
    let shared = r.start(curated_m365(), true);
    until("the guided forwarding", || has(&shared, "forwarding", "failed"));
    // "I've done this" is taken only once the runner waits on the student: by then a backfill not held
    // would already have run, and Finish closed the window.
    until("I've done this is taken", || shared.answer(Answer::Done));
    until("the guided backfill", || has(&shared, "backfill", "needs_you"));
    assert!(r.host.lock().unwrap().window, "Finish waited: the window is still open at the guided backfill");
    assert!(shared.answer(Answer::Done));
    let p = wait(&shared);
    assert_eq!(p.ended, Some("finished"), "{p:?}");
    assert_eq!(row(&p, "forwarding").state, "done_by_hand");
    assert_eq!((row(&p, "canary").state, row(&p, "backfill").state), ("done", "skipped"));
    assert_eq!(p.not_brought_in_before, Some(since_day()));
    let ev = r.events();
    assert!(!ev.iter().any(|e| e == "svc batch"), "no backfill batch: {ev:?}");
    assert!(ran_retired(&r).is_empty(), "{:?}", r.scripts());
}

use knowlu::mail::setup::clean_unfinished;
use knowlu::mail_session::PendingSetup;

fn pending(id: &str, placed: bool, backfilled: bool) -> PendingSetup {
    PendingSetup { address_id: id.to_string(), placed, backfilled, shown: false }
}

fn record(dir: &Path, list: Vec<PendingSetup>) {
    MailState::update(dir, |s| {
        s.pending_setups = list;
        true
    })
    .unwrap();
}

fn pendings(dir: &Path) -> Vec<PendingSetup> {
    MailState::load(dir).unwrap().pending_setups
}

#[test]
fn a_setup_in_flight_keeps_its_address_and_whether_it_was_placed_on_this_device() {
    // Quinn, 2026-10-06: a setup ended by quitting the app mid-run left its new address pending. The device
    // keeps, while a setup runs, its address id and the facts `withdraw` reads, written before the act.
    let r = rig("t1006-in-flight");
    r.host.lock().unwrap().msgs = ana_mail(2);
    let seen: Arc<Mutex<Vec<(String, Vec<PendingSetup>)>>> = Default::default();
    let (log, dir) = (seen.clone(), r.dir.clone());
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if ["m365/signin.js", "m365/forwarding_set.js"].contains(&name) {
            log.lock().unwrap().push((name.to_string(), pendings(&dir)));
        }
    }));
    let shared = r.start(curated_m365(), true);
    until("the guided backfill", || has(&shared, "backfill", "needs_you"));
    assert_eq!(pendings(&r.dir), [pending(A, true, false)], "D13: a guided backfill is not a begun one");
    assert_eq!(wait(&shared).ended, Some("finished"));
    let at = |name: &str| seen.lock().unwrap().iter().find(|(n, _)| n == name).expect(name).1.clone();
    assert_eq!(at("m365/signin.js"), [pending(A, false, false)]);
    assert_eq!(at("m365/forwarding_set.js"), [pending(A, true, false)], "written before a script that may set it");
    assert!(pendings(&r.dir).is_empty(), "an ended setup leaves no record");
    let text = std::fs::read_to_string(r.dir.join("mail.json")).unwrap();
    assert!(!text.contains("pending_setups"), "none costs mail.json nothing: {text}");
}

#[test]
fn at_start_up_an_unfinished_setup_never_placed_has_its_address_removed() {
    let r = rig("t1006-start-up");
    let client = || MailClient::with_token_fn(&r.svc.base, Box::new(|| Ok(TOKEN.to_string()))).unwrap();
    let deletes = || r.events().iter().filter(|e| *e == "svc delete address").count();
    let kept = format!("mail-session-{A}");
    // Never placed, the row still pending: DELETE, the profile goes through the guard, the record clears.
    record(&r.dir, vec![pending(A, false, false)]);
    profile(&r.dir, &kept);
    assert!(clean_unfinished(&r.dir, &client()).is_empty());
    assert_eq!(deletes(), 1);
    assert!(!session_dir(&r.dir, A).unwrap().exists());
    assert!(pendings(&r.dir).is_empty());
    // Placed, backfilled, or an open backfill: nothing is touched, and the record clears.
    for (rec, open) in [(pending(A, true, false), false), (pending(A, false, true), false), (pending(A, false, false), true)] {
        record(&r.dir, vec![rec.clone()]);
        MailState::update(&r.dir, |s| {
            s.open_backfills = if open { vec![A.to_string()] } else { vec![] };
            true
        })
        .unwrap();
        profile(&r.dir, &kept);
        assert!(clean_unfinished(&r.dir, &client()).is_empty(), "{rec:?}");
        assert_eq!(deletes(), 1, "{rec:?}: never deleted");
        assert!(session_dir(&r.dir, A).unwrap().exists() && pendings(&r.dir).is_empty(), "{rec:?}");
    }
    MailState::update(&r.dir, |s| std::mem::take(&mut s.open_backfills).len() > 0).unwrap();
    // A row Settings shows as set up (confirmed, active, quiet...) is never touched.
    for set_up in ["confirmed", "active"] {
        {
            let mut s = r.sv.lock().unwrap();
            s.confirming = set_up == "confirmed";
            s.canary_sent = set_up == "active";
        }
        record(&r.dir, vec![pending(A, false, false)]);
        assert!(clean_unfinished(&r.dir, &client()).is_empty(), "{set_up}");
        assert_eq!(deletes(), 1, "{set_up}: never deleted");
        assert!(session_dir(&r.dir, A).unwrap().exists() && pendings(&r.dir).is_empty(), "{set_up}");
    }
    {
        let mut s = r.sv.lock().unwrap();
        (s.confirming, s.canary_sent) = (false, false);
    }
    // The list cannot be read, or the delete is refused: the record and the profile stay for the next start.
    record(&r.dir, vec![pending(A, false, false)]);
    assert_eq!(clean_unfinished(&r.dir, &client_on_a_closed_port()), [A]);
    assert_eq!(deletes(), 1);
    r.sv.lock().unwrap().delete_refused = true;
    assert_eq!(clean_unfinished(&r.dir, &client()), [A]);
    assert_eq!(deletes(), 2, "the delete was asked");
    assert!(session_dir(&r.dir, A).unwrap().exists() && pendings(&r.dir) == [pending(A, false, false)]);
    // The next start's delete is taken.
    r.sv.lock().unwrap().delete_refused = false;
    assert!(clean_unfinished(&r.dir, &client()).is_empty());
    assert_eq!(deletes(), 3);
    assert!(!session_dir(&r.dir, A).unwrap().exists() && pendings(&r.dir).is_empty());
    // An address the service no longer lists is already gone: its profile goes, nothing is asked.
    record(&r.dir, vec![pending(B, false, false)]);
    profile(&r.dir, &format!("mail-session-{B}"));
    assert!(clean_unfinished(&r.dir, &client()).is_empty());
    assert_eq!(deletes(), 3);
    assert!(!session_dir(&r.dir, B).unwrap().exists() && pendings(&r.dir).is_empty());
}

#[test]
fn a_withdrawal_the_service_refused_is_tried_again_at_the_next_start() {
    let r = rig("t1006-withdraw-later");
    r.sv.lock().unwrap().delete_refused = true;
    queue(&r, "m365/forwarding_set.js", vec![json!({ "result": "elsewhere" })]);
    let shared = r.start(curated_m365(), true);
    until("the question", || shared.progress().ask == Some("forwarding_elsewhere"));
    assert!(shared.answer(Answer::Replace(false)));
    assert!(wait(&shared).line.is_some_and(|l| l.contains("could not be removed")));
    assert_eq!(pendings(&r.dir), [pending(A, false, false)], "kept for the next start");
    r.sv.lock().unwrap().delete_refused = false;
    let client = MailClient::with_token_fn(&r.svc.base, Box::new(|| Ok(TOKEN.to_string()))).unwrap();
    assert!(clean_unfinished(&r.dir, &client).is_empty());
    assert!(pendings(&r.dir).is_empty());
}

#[test]
fn the_launch_hook_cleans_unfinished_setups_under_the_mail_window_before_any_resume() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("mail").join("live.rs")).unwrap();
    let body = &src[src.find("pub fn on_start").expect("on_start")..];
    let body = &body[..body.find("\n}\n").unwrap()];
    let (sweep, clean) = (body.find("sweep_at_launch(").unwrap(), body.find("clean_unfinished(").expect("on_start cleans"));
    let resume = body.find("start_resume(").unwrap();
    assert!(sweep < clean && clean < resume, "{body}");
    assert!(clean < body.find("keep_session").unwrap(), "cleaned whatever the saved sign-in switch says");
    let gate = body.find("resume_held_after_privacy_check(").expect("on_start asks the privacy gate");
    assert!(clean < gate && gate < body.find("open_backfills(").unwrap() && gate < resume, "held before any resume: {body}");
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
fn a_gmail_script_that_throws_on_the_page_still_settling_after_a_reload_gets_one_settled_retry() {
    // Live 2026-10-18: Save took and Gmail reloaded (Lost); the rerun with reloaded set threw (Failed) on a page
    // still filling its forward-to list. The same script a moment later read the setting back fine.
    let ours = json!({ "result": "ok", "forwarding": "on", "to": "ours", "keep_copy": true, "changed": false });
    let (lost, failed) = (json!({ "refuse": "lost" }), json!({ "refuse": "failed" }));
    let r = rig("t18c-settled-retry");
    r.host.lock().unwrap().title_address = Some(HINT_PAGE.to_string());
    queue(&r, "gmail/forwarding_set.js", vec![lost.clone(), failed.clone(), ours]);
    let p = wait(&r.start(Kind::Gmail, true));
    assert_eq!(row(&p, "turn_on").state, "done");
    let sets = r.calls("gmail/forwarding_set.js");
    assert_eq!(sets.len(), 3, "run, rerun after the reload, one settled retry");
    assert_ne!(sets[0]["reloaded"], json!(true));
    assert_eq!((sets[1]["reloaded"].clone(), sets[2]["reloaded"].clone()), (json!(true), json!(true)));
    assert_eq!(sets[1], sets[2], "the retry carries the same arguments");

    // A second throw is the failure it is today, and the script is not run a fourth time.
    let t = rig("t18c-settled-retry-twice");
    t.host.lock().unwrap().title_address = Some(HINT_PAGE.to_string());
    queue(&t, "gmail/forwarding_set.js", vec![lost, failed]);
    let shared = t.start(Kind::Gmail, true);
    until("the failed step", || has(&shared, "turn_on", "failed"));
    assert_eq!(t.calls("gmail/forwarding_set.js").len(), 3);
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
fn mail_disconnect_removes_every_per_mailbox_entry_for_that_mailbox_and_no_other() {
    // Review of c8806b3: disconnect left the backfill's frontier behind (and now the lone-stall record and the
    // day's Gmail message count, which lives in the pace). Every per-mailbox entry of the disconnected mailbox goes,
    // and the other mailbox's stay.
    let r = rig("disconnect-entries");
    profile(&r.dir, &format!("mail-session-{A}"));
    let day = NOW.div_euclid(DAY);
    MailState::update(&r.dir, |s| {
        for id in [A, B] {
            s.open_backfills.push(id.to_string());
            s.backfill_pace.insert(id.to_string(), BackfillPace { day, sent_today: 9, paused_until: None, loads_today: 40, messages_today: 2 });
            s.last_reconcile.insert(id.to_string(), iso(NOW));
            s.last_look.insert(id.to_string(), NOW);
            s.backfill_frontier.insert(id.to_string(), day - 30);
            s.lone_stall.insert(id.to_string(), LoneStall { at: NOW - 60, days: 1, last_day: day });
            s.recover_stall.insert(id.to_string(), LoneStall { at: NOW - 240, days: 2, last_day: day });
            s.lone_left.insert(id.to_string(), vec![NOW - 120]);
            s.gmail_messages.insert(id.to_string(), vec![NOW - 180]);
        }
        true
    })
    .unwrap();
    let svc = disc_service(&r, "m365", 200);
    assert_eq!(disconnect(&r, &svc)["ok"], json!(true));
    let st = MailState::load(&r.dir).unwrap();
    assert_eq!(st.open_backfills, [B]);
    for (name, keys) in [
        ("backfill_pace", st.backfill_pace.keys().cloned().collect::<Vec<_>>()),
        ("last_reconcile", st.last_reconcile.keys().cloned().collect()),
        ("last_look", st.last_look.keys().cloned().collect()),
        ("backfill_frontier", st.backfill_frontier.keys().cloned().collect()),
        ("lone_stall", st.lone_stall.keys().cloned().collect()),
        ("recover_stall", st.recover_stall.keys().cloned().collect()),
        ("lone_left", st.lone_left.keys().cloned().collect()),
        ("gmail_messages", st.gmail_messages.keys().cloned().collect()),
    ] {
        assert_eq!(keys, [B], "{name}");
    }
}

#[test]
fn mail_disconnect_drops_that_mailboxs_sent_nonces_and_no_other() {
    let r = rig("disconnect-sent-nonces");
    profile(&r.dir, &format!("mail-session-{A}"));
    MailState::update(&r.dir, |s| {
        for id in [A, B] {
            s.sent_nonces.insert(id.to_string(), vec![SentNonce { nonce: OLD.into(), opened: NOW - 60 }]);
        }
        true
    })
    .unwrap();
    let svc = disc_service(&r, "m365", 200);
    assert_eq!(disconnect(&r, &svc)["ok"], json!(true));
    let st = MailState::load(&r.dir).unwrap();
    assert_eq!(st.sent_nonces.keys().cloned().collect::<Vec<_>>(), [B]);
    assert_eq!(st.sent_nonces[B].len(), 1);
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

#[test]
fn a_rotate_of_a_recorded_address_keeps_it_from_the_start_up_clean() {
    // Review of a7f1ca0: Settings' Rotate on a pending row puts the (new) address in the mailbox, yet the
    // record still said never placed, so the next start deleted it. A rotate marks the record.
    let r = rig("t1006-rotate-recorded");
    profile(&r.dir, &format!("mail-session-{A}"));
    r.sv.lock().unwrap().provider = "m365".into();
    record(&r.dir, vec![pending(A, false, false)]);
    let kind = Kind::Rotate { address_id: A.to_string(), provider: "m365".to_string() };
    assert_eq!(wait(&r.start(kind, true)).ended, Some("finished"));
    // The row reads pending again, as a Gmail rotate leaves it.
    r.sv.lock().unwrap().canary_sent = false;
    let client = MailClient::with_token_fn(&r.svc.base, Box::new(|| Ok(TOKEN.to_string()))).unwrap();
    assert!(clean_unfinished(&r.dir, &client).is_empty());
    assert!(!r.events().contains(&"svc delete address".to_string()), "{:?}", r.events());
    assert!(session_dir(&r.dir, A).unwrap().exists() && pendings(&r.dir).is_empty());
}

#[test]
fn an_address_settings_has_shown_is_kept_from_the_start_up_clean() {
    // Review of a7f1ca0: Settings shows a pending row's address with Copy, and the student may paste it into
    // the mailbox by hand. With no setup running (the console reads the status at every launch, before the
    // start-up clean may have run), a recorded never-placed row's address is withheld, so Settings offers no
    // Copy; while a setup runs, a status read hands it out and marks the record.
    let r = rig("t1006-settings-shown");
    record(&r.dir, vec![pending(A, false, false), pending(B, false, false)]);
    let rows = || {
        vec![
            json!({ "address_id": A, "provider": "m365", "address": KNOWLU, "status": "pending" }),
            json!({ "address_id": B, "provider": "m365", "address": null, "status": "pending" }),
            json!({ "address_id": "0b0e5b2c-3333-4aaa-8bbb-000000000003", "provider": "m365", "address": KNOWLU_NEW, "status": "active" }),
        ]
    };
    let idle = status_for(Ok(rows()), &r.dir, false);
    let box_a = &idle["mailboxes"][0];
    assert_eq!((box_a["address_id"].as_str(), box_a["address"].is_null()), (Some(A), true), "withheld: {idle}");
    assert_eq!(idle["mailboxes"][2]["address"], json!(KNOWLU_NEW), "an unrecorded row keeps its address");
    assert!(pendings(&r.dir).iter().all(|p| !p.shown), "nothing was handed out");
    let running = status_for(Ok(rows()), &r.dir, true);
    assert_eq!(running["mailboxes"][0]["address"], json!(KNOWLU), "a running setup's guided step reads it");
    let shown: Vec<(String, bool)> = pendings(&r.dir).into_iter().map(|p| (p.address_id, p.shown)).collect();
    assert_eq!(shown, [(A.to_string(), true), (B.to_string(), false)], "only a row whose address was handed out");
    let client = MailClient::with_token_fn(&r.svc.base, Box::new(|| Ok(TOKEN.to_string()))).unwrap();
    clean_unfinished(&r.dir, &client);
    assert!(!r.events().contains(&"svc delete address".to_string()), "{:?}", r.events());
    assert!(pendings(&r.dir).is_empty());
}

#[test]
fn an_address_settings_handed_out_during_a_setup_is_never_withdrawn() {
    // Re-check of 472da74: Settings opened while a setup runs hands its address out with Copy and marks the
    // record shown; the student may paste it into the mailbox. A stop that would otherwise withdraw keeps it,
    // and the run's own later writes never take the mark back.
    let r = rig("t1006-shown-no-withdraw");
    queue(&r, "m365/forwarding_set.js", vec![json!({ "result": "elsewhere" })]);
    let dir = r.dir.clone();
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "m365/forwarding_set.js" {
            let row = json!({ "address_id": A, "provider": "m365", "address": KNOWLU, "status": "pending" });
            status_for(Ok(vec![row]), &dir, true);
        }
    }));
    let shared = r.start(curated_m365(), true);
    until("the question", || shared.progress().ask == Some("forwarding_elsewhere"));
    // The run wrote its record again after the script (`placed` back to false): `shown` holds.
    assert_eq!(pendings(&r.dir).iter().map(|p| (p.placed, p.shown)).collect::<Vec<_>>(), [(false, true)]);
    assert!(shared.answer(Answer::Replace(false)));
    let p = wait(&shared);
    assert_eq!((p.ended, p.line), (Some("stopped"), Some("setup stopped")));
    assert!(!p.kept, "a stop is not a kept end: the console shows no kept line");
    assert!(!r.events().contains(&"svc delete address".to_string()), "{:?}", r.events());
}

/// Settings' status read while a setup runs: it hands the address out with Copy and marks the record shown.
fn settings_reads_status(dir: &Path) {
    let row = json!({ "address_id": A, "provider": "m365", "address": KNOWLU, "status": "pending" });
    status_for(Ok(vec![row]), dir, true);
}

#[test]
fn a_cancelled_sign_in_never_deletes_an_address_settings_handed_out() {
    // Review of 6e82a5c: the student opens Settings mid-sign-in (the address is handed out and marked shown),
    // pastes it into the mailbox's forwarding by hand, then closes the sign-in window. The cancel keeps the
    // address and says so, never "setup cancelled" as if nothing changed.
    let r = rig("t1012-shown-cancel");
    queue(&r, "m365/signin.js", vec![json!({ "result": "not found" })]);
    let (dir, host) = (r.dir.clone(), r.host.clone());
    r.host.lock().unwrap().on_script = Some(Arc::new(move |name| {
        if name == "m365/signin.js" {
            settings_reads_status(&dir);
            host.lock().unwrap().student_closed = true;
        }
    }));
    let p = wait(&r.start(curated_m365(), true));
    assert!(!r.events().contains(&"svc delete address".to_string()), "{:?}", r.events());
    assert_eq!(p.ended, Some("cancelled"));
    assert!(p.line.is_some_and(|l| l != "setup cancelled" && l.contains("kept")), "{:?}", p.line);
    assert!(p.kept, "the console reads the flag, never the line's words");
    assert!(pendings(&r.dir).is_empty(), "the run handled its address: kept");
    assert!(r.host.lock().unwrap().closed >= 1, "the window is closed");
}

#[test]
fn a_refused_hint_never_deletes_an_address_settings_handed_out() {
    // The same for the guided provider's typed hint: a refusal after Settings handed the address out keeps it.
    let r = rig("t1012-shown-hint");
    let dir = r.dir.clone();
    {
        let mut s = r.sv.lock().unwrap();
        s.provider_of = "other".into();
        s.hint_refused = Some((500, "boom".into()));
        s.on_hint = Some(Box::new(move || settings_reads_status(&dir)));
    }
    let kind = Kind::School { unitid: Some("0".to_string()), address: Some("student@school.example".to_string()) };
    let p = wait(&r.start(kind, true));
    assert!(r.events().contains(&"svc hint".to_string()), "{:?}", r.events());
    assert!(!r.events().contains(&"svc delete address".to_string()), "{:?}", r.events());
    assert_eq!(p.ended, Some("failed"));
    assert!(p.line.is_some_and(|l| l.contains("kept")), "{:?}", p.line);
    assert!(!p.sentence.is_empty(), "the refusal is still named");
    assert!(p.kept);
    assert!(pendings(&r.dir).is_empty(), "the run handled its address: kept");
}

// ---- the saved-sign-in switch against a running slot (pre-PR review, 2026-10-20, signin-off; spec §4.13) ----
//
// Turning the saved sign-in off closes the mail window and deletes every kept profile. A slot mail step that is
// already running must stop opening them, the switch holds the one mail window until the last profile is gone,
// and a step that starts (or is about to open a window) after the switch was recorded opens none at all.

#[path = "support/slot_rig.rs"]
mod slot_rig;

/// `mail::stop_runs` and the mail window's hold are process-wide: the tests below that run the slot take this lock.
static SLOT_RUN_LOCK: Mutex<()> = Mutex::new(());

fn slot_rows(ids: &[&str]) -> Vec<Value> {
    ids.iter()
        .map(|m| json!({ "address_id": m, "provider": "m365", "address": slot_rig::KNOWLU, "status": "active", "lapsed_since": null }))
        .collect()
}

#[test]
fn slot_opens_no_profile_once_saved_sign_in_is_off() {
    let _runs = SLOT_RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = temp("signin-off-slot");
    profile(&d, "mail-session-mbx-a");
    MailState::update(&d, |s| {
        s.keep_session = false;
        true
    })
    .unwrap();
    let rows = slot_rows(&["mbx-a"]);
    let svc = slot_rig::Svc::start(rows.clone());
    let (client, clock, host) = (svc.client(), slot_rig::Clk::at(slot_rig::NOW), slot_rig::FakeHost::new(slot_rig::fwd("on", "ours")));
    let out = knowlu::mail::slot::run_step(&knowlu::mail::slot::SlotEnv { host: &host, client: &client, clock: &clock, data_dir: &d }, &rows);
    assert_eq!(host.opens(), Vec::<(PathBuf, bool)>::new(), "a kept profile was opened with the saved sign-in off");
    assert_eq!(out, None, "nothing ran, so the step has no line");
    let _ = std::fs::remove_dir_all(&d);
}

/// The live half, which needs the app's handle, so no test drives it: the toggle's close stops the runs and then
/// closes the mail window (`stop_and_close`), and the command takes the mail window within the sign-out's bound,
/// with the same close as the stop it calls on every miss; its hold is what `keep_session_with` keeps.
#[test]
fn the_toggles_live_close_stops_runs_first_and_takes_the_mail_window_within_the_bound() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("mail.rs")).unwrap();
    let body = |head: &str| {
        let at = src.find(head).unwrap_or_else(|| panic!("{head}"));
        &src[at..at + src[at..].find("\n}\n").expect("the end of its body")]
    };
    let close = body("pub fn stop_and_close(app: &tauri::AppHandle) {");
    let (stop, shut) = (close.find("stop_runs();"), close.find("close(app);"));
    assert!(stop.is_some() && stop < shut, "stop_and_close sets the stop before it closes the window: {close}");
    let toggle = body("pub fn mail_keep_session(");
    let (closes, takes) = (toggle.find("stop_and_close(&app);"), toggle.find("MailWindow::take_by("));
    assert!(closes.is_some() && closes < takes, "the toggle closes, then takes the mail window: {toggle}");
    let take = &toggle[takes.unwrap()..];
    assert!(take.contains("crate::account::MAIL_STOP_BOUND") && take.contains("|| stop_and_close(&app)"), "within the bound, stopping the holder on a miss: {take}");
}

/// A slot-shaped mail window: each profile it opens is held, a file open with no sharing as WebView2 holds its
/// profile, until the window closes. A script waits while the window is open and then answers as a vanished
/// window does, so a pass ends only once its window is closed under it.
#[cfg(windows)]
struct HeldWindow {
    st: Arc<Mutex<HeldState>>,
    opened: std::sync::mpsc::Sender<String>,
}

#[cfg(windows)]
#[derive(Default)]
struct HeldState {
    open: Option<std::fs::File>,
    /// Every profile opened, by folder name.
    opens: Vec<String>,
}

#[cfg(windows)]
impl HeldWindow {
    fn is_open(&self) -> bool {
        self.st.lock().unwrap().open.is_some()
    }
}

#[cfg(windows)]
impl knowlu::mail::recover::Mailbox for HeldWindow {
    fn script(&self, _: &str, _: &Value) -> Result<Value, knowlu::mail::Refusal> {
        let began = std::time::Instant::now();
        while self.is_open() && began.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(knowlu::mail::Refusal::NoPage)
    }
    fn open(&self, _: knowlu::mail::recover::Place) -> Result<(), knowlu::mail::Refusal> {
        if self.is_open() { Ok(()) } else { Err(knowlu::mail::Refusal::NoPage) }
    }
}

#[cfg(windows)]
impl Host for HeldWindow {
    fn use_provider(&self, _: Provider) {}
    fn open_window(&self, profile: &Path, _visible: bool) -> Result<(), String> {
        use std::os::windows::fs::OpenOptionsExt;
        let held = std::fs::OpenOptions::new().write(true).create(true).truncate(true).share_mode(0).open(profile.join("held"));
        let name = profile.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut s = self.st.lock().unwrap();
        s.open = Some(held.map_err(|e| e.to_string())?);
        s.opens.push(name.clone());
        drop(s);
        let _ = self.opened.send(name);
        Ok(())
    }
    fn window_open(&self) -> bool {
        self.is_open()
    }
    fn signed_in(&self) -> bool {
        self.is_open()
    }
    fn title_address(&self) -> Option<String> {
        None
    }
    fn popup_open(&self) -> bool {
        false
    }
    fn step_aside(&self) -> Result<(), String> {
        Ok(())
    }
    fn come_back(&self) -> Result<(), String> {
        Ok(())
    }
    fn close_window(&self) {
        self.st.lock().unwrap().open = None;
    }
}

/// What the toggle's close hands `keep_session_with`: the mail window's hold and, when it is let go, whether
/// any `mail-session-*` profile was still on disk.
#[cfg(windows)]
struct Kept<'a> {
    _hold: Option<knowlu::mail::MailWindow>,
    dir: PathBuf,
    left_at_release: &'a Cell<Option<bool>>,
}

#[cfg(windows)]
impl Drop for Kept<'_> {
    fn drop(&mut self) {
        let left = std::fs::read_dir(&self.dir).map(|d| d.flatten().any(|e| e.file_name().to_string_lossy().starts_with("mail-session-")));
        self.left_at_release.set(Some(left.unwrap_or(true)));
    }
}

/// A slot's mail step holds the mail window on the first of three mailboxes when the student turns the saved
/// sign-in off. The toggle's close (as `mail::stop_and_close` does it: the stop, then the window) ends that
/// mailbox's pass; the step then opens no second profile. The toggle takes the hold within its time limit,
/// keeps it until every `mail-session-<id>` is deleted, and lets it go only then.
#[cfg(windows)]
#[test]
fn mail_keep_session_off_stops_the_slot_and_holds_the_window_until_every_profile_is_gone() {
    let _runs = SLOT_RUN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = temp("signin-off-hold");
    let ids = ["mbx-a", "mbx-b", "mbx-c"];
    for m in ids {
        std::fs::create_dir_all(d.join(format!("mail-session-{m}"))).unwrap();
    }
    let rows = slot_rows(&ids);
    let st = Arc::new(Mutex::new(HeldState::default()));
    let (opened, first) = std::sync::mpsc::channel();
    let window = HeldWindow { st: st.clone(), opened };
    let (svc, slot_dir, due) = (slot_rig::Svc::start(rows.clone()), d.clone(), rows.clone());
    let slot = std::thread::spawn(move || {
        let _hold = knowlu::mail::MailWindow::try_take().expect("the slot's mail step holds the mail window");
        let (client, clock) = (svc.client(), slot_rig::Clk::at(slot_rig::NOW));
        knowlu::mail::slot::run_step(&knowlu::mail::slot::SlotEnv { host: &window, client: &client, clock: &clock, data_dir: &slot_dir }, &due)
    });
    assert_eq!(first.recv_timeout(Duration::from_secs(60)).expect("the slot opened its first mailbox"), "mail-session-mbx-a");

    let left_at_release = Cell::new(None);
    let out = keep_session_with(&d, false, || {
        let close = || {
            knowlu::mail::stop_runs();
            st.lock().unwrap().open = None;
        };
        close();
        let hold = knowlu::mail::MailWindow::take_by(std::time::Instant::now() + Duration::from_secs(30), close);
        assert!(hold.is_some(), "the toggle took the mail window within its time limit");
        Kept { _hold: hold, dir: d.clone(), left_at_release: &left_at_release }
    });
    slot.join().expect("the slot's thread");

    assert_eq!(st.lock().unwrap().opens, ["mail-session-mbx-a"], "a mail profile was opened after the toggle's close");
    assert_eq!((out["ok"].clone(), out["profiles_left"].clone()), (json!(true), json!(false)), "{out}");
    for m in ids {
        assert!(!d.join(format!("mail-session-{m}")).exists(), "mail-session-{m} survived the switch");
    }
    assert_eq!(left_at_release.get(), Some(false), "the mail window was let go before every mail profile was gone");
    assert!(knowlu::mail::MailWindow::try_take().is_some(), "the toggle let the mail window go when it was done");
    let _ = std::fs::remove_dir_all(&d);
}
