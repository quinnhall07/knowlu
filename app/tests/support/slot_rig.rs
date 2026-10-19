//! A fake mailbox window and a loopback `mail` service for the slot's mail step (email plan T11.5), shared by
//! `#[path]` include with `app/tests/scheduler.rs`. Nothing here leaves the machine: the service binds
//! `127.0.0.1:0`, the window is a model that answers the compiled-in scripts' JSON, and the clock moves only
//! when the code under test waits. The address and the token are unique markers, so a byte search can prove
//! neither reaches a file Knowlu writes.
#![allow(dead_code)]
use knowlu::mail::host::Host;
use knowlu::mail::recover::{Clock, Mailbox, Place};
use knowlu::mail::Refusal;
use knowlu::mail_client::MailClient;
use knowlu::mail_steps::Provider;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

/// A mailbox id, a Knowlu forwarding address and a bearer no real account holds.
pub const ID: &str = "0b0e5b2c-1111-4aaa-8bbb-000000000001";
pub const KNOWLU: &str = "marker-address-qzx7731@in.example.invalid";
pub const TOKEN: &str = "marker-bearer-qzx7731-secret";
pub const NOW: i64 = 1_790_000_000;
pub const DAY: i64 = 86_400;
const FPKEY_B64: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";
const NONCE: &str = "bk7f3a9c2e41";

pub fn iso(secs: i64) -> String {
    jiff::Timestamp::from_second(secs).unwrap().to_string()
}

/// A clock that moves only when something waits.
pub struct Clk(pub AtomicI64);

impl Clk {
    pub fn at(secs: i64) -> Clk {
        Clk(AtomicI64::new(secs))
    }
}

impl Clock for Clk {
    fn now(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
    fn sleep(&self, secs: i64) {
        self.0.fetch_add(secs, Ordering::SeqCst);
    }
}

// ---- the loopback service ---------------------------------------------------------------------------

#[derive(Default)]
pub struct SvcState {
    /// What `GET /addresses` answers.
    pub rows: Vec<Value>,
    /// The status `DELETE /addresses/<id>` answers (0 is 200).
    pub delete_status: u16,
    /// Each request: (method, path after `/mail`, body).
    pub log: Vec<(String, String, String)>,
}

pub struct Svc {
    pub base: String,
    pub st: Arc<Mutex<SvcState>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Svc {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// A `GET /addresses` row.
pub fn row(status: &str, provider: &str) -> Value {
    json!({ "address_id": ID, "provider": provider, "address": KNOWLU, "status": status,
            "lapsed_since": null, "last_canary_ok_at": "2026-10-02T10:00:40Z", "backfill_since": null })
}

impl Svc {
    pub fn start(rows: Vec<Value>) -> Svc {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        listener.set_nonblocking(true).expect("nonblocking");
        let base = format!("http://127.0.0.1:{}/functions/v1", listener.local_addr().unwrap().port());
        let st = Arc::new(Mutex::new(SvcState { rows, ..Default::default() }));
        let stop = Arc::new(AtomicBool::new(false));
        let (st2, stop2) = (st.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            while !stop2.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let Some((method, path, body)) = read_request(&mut stream) else { continue };
                let (status, reply) = answer(&st2, &method, &path, &body);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        Svc { base, st, stop, thread: Some(thread) }
    }

    pub fn client(&self) -> MailClient {
        MailClient::with_token_fn(&self.base, Box::new(|| Ok(TOKEN.to_string()))).expect("a loopback base")
    }

    /// The requests so far as `"METHOD /route"`, with a state change's word appended.
    pub fn calls(&self) -> Vec<String> {
        let st = self.st.lock().unwrap();
        st.log
            .iter()
            .map(|(m, p, b)| match serde_json::from_str::<Value>(b).ok().and_then(|v| v["state"].as_str().map(str::to_string)) {
                Some(s) => format!("{m} {p} {s}"),
                None => format!("{m} {p}"),
            })
            .collect()
    }

    pub fn bodies(&self, route: &str) -> Vec<Value> {
        let st = self.st.lock().unwrap();
        st.log.iter().filter(|(_, p, _)| p == route).map(|(_, _, b)| serde_json::from_str(b).unwrap_or(Value::Null)).collect()
    }
}

fn answer(st: &Mutex<SvcState>, method: &str, path: &str, body: &str) -> (u16, String) {
    let route = path.split("/mail").nth(1).unwrap_or("").to_string();
    let mut s = st.lock().unwrap();
    s.log.push((method.to_string(), route.clone(), body.to_string()));
    match (method, route.as_str()) {
        ("GET", "/addresses") => (200, json!({ "addresses": s.rows }).to_string()),
        ("POST", "/addresses") => (201, json!({ "address_id": ID, "address": KNOWLU, "status": "pending" }).to_string()),
        ("DELETE", p) if p.starts_with("/addresses/") => match s.delete_status {
            0 => (200, r#"{"deleted":true}"#.to_string()),
            code => (code, r#"{"error":"failed"}"#.to_string()),
        },
        ("POST", p) if p.ends_with("/hint") || p.ends_with("/state") => (200, r#"{"ok":true}"#.to_string()),
        ("POST", "/canary") => (201, json!({ "canary_id": "c-1", "sent_at": "2026-10-02T10:00:00Z" }).to_string()),
        ("POST", "/reconcile") => (200, json!({ "address_id": ID, "key": FPKEY_B64, "held": [] }).to_string()),
        ("POST", "/batches") => (201, json!({ "batch_id": "batch-1", "nonce": NONCE, "phase": "recover" }).to_string()),
        ("POST", p) if p.starts_with("/batches/") && p.ends_with("/sent") => (200, r#"{"ok":true}"#.to_string()),
        _ => (404, r#"{"error":"no such route"}"#.to_string()),
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> Option<(String, String, String)> {
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
    Some((method, path, String::from_utf8_lossy(&buf[head_end..]).to_string()))
}

// ---- the fake mailbox window --------------------------------------------------------------------------

pub struct HostState {
    pub signed_in: bool,
    pub window: bool,
    /// Every window opened: (profile, visible).
    pub opens: Vec<(PathBuf, bool)>,
    pub closed: u32,
    pub back: u32,
    /// Every script run (name, args), and every place opened as `open <Place>`.
    pub log: Vec<(String, Value)>,
    /// What the forwarding read answers now; a set that works turns it on to ours.
    pub fwd: Value,
    /// The forwarding set cannot find its control.
    pub set_fails: bool,
    /// The mailbox: (sender, subject, instant).
    pub msgs: Vec<(String, String, i64)>,
    /// Gmail: a non-delivery report landed in the Inbox at this instant; a look reading from before it finds it.
    pub report_at: Option<i64>,
}

pub fn fwd(forwarding: &str, to: &str) -> Value {
    json!({ "result": "ok", "forwarding": forwarding, "to": to, "keep_copy": true })
}

pub struct FakeHost {
    pub st: Arc<Mutex<HostState>>,
}

impl FakeHost {
    pub fn new(fwd_now: Value) -> FakeHost {
        FakeHost {
            st: Arc::new(Mutex::new(HostState {
                signed_in: true,
                window: false,
                opens: vec![],
                closed: 0,
                back: 0,
                log: vec![],
                fwd: fwd_now,
                set_fails: false,
                msgs: vec![],
                report_at: None,
            })),
        }
    }

    pub fn scripts(&self) -> Vec<String> {
        self.st.lock().unwrap().log.iter().filter(|(p, _)| p.contains(".js")).map(|(p, _)| p.clone()).collect()
    }

    pub fn calls(&self, name: &str) -> Vec<Value> {
        self.st.lock().unwrap().log.iter().filter(|(p, _)| p == name).map(|(_, a)| a.clone()).collect()
    }

    pub fn opens(&self) -> Vec<(PathBuf, bool)> {
        self.st.lock().unwrap().opens.clone()
    }
}

impl Mailbox for FakeHost {
    fn open(&self, place: Place) -> Result<(), Refusal> {
        self.st.lock().unwrap().log.push((format!("open {place:?}"), Value::Null));
        Ok(())
    }

    fn script(&self, path: &str, args: &Value) -> Result<Value, Refusal> {
        let mut s = self.st.lock().unwrap();
        s.log.push((path.to_string(), args.clone()));
        Ok(match path {
            "m365/signin.js" => json!({ "result": "ok", "signed_in": true, "mailbox_hint": "student.one@school.example.invalid" }),
            "m365/forwarding_read.js" | "gmail/forwarding_read.js" => s.fwd.clone(),
            "m365/forwarding_set.js" | "gmail/forwarding_set.js" if s.set_fails => json!({ "result": "not found", "control": "switch" }),
            "m365/forwarding_set.js" | "gmail/forwarding_set.js" => {
                s.fwd = fwd("on", "ours");
                json!({ "result": "ok", "forwarding": "on", "changed": true })
            }
            "m365/forwarding_off.js" | "gmail/forwarding_off.js" => {
                s.fwd = fwd("off", "none");
                json!({ "result": "ok", "forwarding": "off", "changed": true })
            }
            "m365/search.js" | "gmail/search.js" => json!({ "result": "ok", "count": s.msgs.len() }),
            "m365/list.js" => {
                let rows: Vec<Value> = s
                    .msgs
                    .iter()
                    .map(|(who, subject, at)| json!({ "sender": who, "subject": subject, "received": format!("at-{at}"), "received_at": iso(*at) }))
                    .collect();
                json!({ "result": "ok", "total": rows.len(), "rows": rows })
            }
            "m365/forward.js" => json!({ "result": "ok", "sent": true }),
            "gmail/list.js" => {
                let rows: Vec<Value> = s
                    .msgs
                    .iter()
                    .map(|(who, subject, at)| json!({ "sender": who, "subject": subject, "received": format!("at-{at}"), "received_at": iso(*at) }))
                    .collect();
                let n = rows.len();
                json!({ "result": "ok", "page": { "from": 1, "to": n, "total": n }, "moved": false, "rows": rows })
            }
            "gmail/forward.js" => json!({ "result": "ok", "sent": true, "count": args["rows"].as_array().map_or(1, |r| r.len()) }),
            "m365/sent_cleanup.js" | "gmail/sent_cleanup.js" => json!({ "result": "ok", "deleted": 1 }),
            // Gmail looks in the Inbox for a non-delivery report between sends and once at the end.
            "gmail/bounce.js" => {
                let since = args["since"].as_str().and_then(|t| t.parse::<jiff::Timestamp>().ok()).map_or(0, |t| t.as_second());
                if s.report_at.is_some_and(|t| t >= since) {
                    json!({ "result": "refused", "reason": "non-delivery report" })
                } else {
                    json!({ "result": "ok", "rows": 0 })
                }
            }
            "ready.js" => json!("complete"),
            other => panic!("the mail step ran {other}"),
        })
    }
}

impl Host for FakeHost {
    fn use_provider(&self, _: Provider) {}

    fn open_window(&self, profile: &Path, visible: bool) -> Result<(), String> {
        std::fs::create_dir_all(profile.join("Default")).unwrap();
        std::fs::write(profile.join("Default").join("Cookies"), "synthetic").unwrap();
        let mut s = self.st.lock().unwrap();
        s.opens.push((profile.to_path_buf(), visible));
        s.window = true;
        Ok(())
    }

    fn window_open(&self) -> bool {
        self.st.lock().unwrap().window
    }

    fn signed_in(&self) -> bool {
        self.st.lock().unwrap().signed_in
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
        self.st.lock().unwrap().back += 1;
        Ok(())
    }

    fn close_window(&self) {
        let mut s = self.st.lock().unwrap();
        s.window = false;
        s.closed += 1;
    }
}
