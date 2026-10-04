//! The mail pass over loopback: `pull_mail` (email spec 2026-09-30 §4.8 steps 1, 4 to 7, §4.10; plan T6.3).
//!
//! Every test binds `127.0.0.1:0`, serves its own replies from a second thread and joins it; every vault
//! is a temporary directory. Nothing leaves the machine and no test reads a real vault (CLAUDE.md).
//! `today` is the vault clock's (`cli::vault_zone`), so a vault's day 1 is today and its day 2 is
//! tomorrow whatever date the test runs on, and every due is written relative to it.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use jiff::civil::Date;
use jiff::Timestamp;
use knowlu_engine::cloudmodel::{CloudClient, CloudConfig};
use knowlu_engine::enrich::{Options, BATCH_BUDGET};
use knowlu_engine::mail::{pull_mail_on, MAIL_ACTOR};
use serde_json::{json, Value};

/// A loopback server answering `replies` in order, then an empty pull; it records every request and
/// never blocks a test that makes fewer requests than it has replies.
struct Loop {
    base: String,
    stop: Arc<AtomicBool>,
    seen: Arc<Mutex<Vec<String>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Loop {
    fn requests(&mut self) -> Vec<String> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            handle.join().expect("the listener thread did not panic");
        }
        self.seen.lock().unwrap().clone()
    }

    fn client(&self) -> CloudClient {
        let cfg = CloudConfig {
            api_base: self.base.clone(),
            anon_key: "anon-not-a-secret".to_string(),
            session_credential_target: "knowlu/test-profile/session".to_string(),
            account_id: "acct-1".to_string(),
        };
        CloudClient::new(&cfg, "jwt-not-a-secret")
    }

    /// The requests made to one endpoint (`mail-pull`, `mail-resolve`), in order.
    fn calls(&mut self, endpoint: &str) -> Vec<String> {
        let line = format!("POST /functions/v1/{endpoint} ");
        self.requests().into_iter().filter(|r| r.starts_with(&line)).collect()
    }

    /// The `ack` array of the n-th `mail-pull` request's body.
    fn ack_of(&mut self, n: usize) -> Vec<String> {
        let requests = self.calls("mail-pull");
        let body: Value = serde_json::from_str(body_of(&requests[n])).expect("a JSON body");
        body["ack"].as_array().expect("an ack array").iter().map(|u| u.as_str().unwrap().to_string()).collect()
    }
}

pub fn body_of(request: &str) -> &str {
    request.split("\r\n\r\n").nth(1).unwrap_or("")
}

fn read_request(stream: &std::net::TcpStream) -> String {
    let mut reader = BufReader::new(stream.try_clone().expect("clone the accepted stream"));
    let (mut head, mut length) = (String::new(), 0usize);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = rest.trim().parse().unwrap_or(0);
        }
        let blank = line == "\r\n" || line == "\n";
        head.push_str(&line);
        if blank {
            break;
        }
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        let _ = reader.read_exact(&mut body);
    }
    format!("{head}{}", String::from_utf8_lossy(&body))
}

/// `answer` is called with each request's text and the number of requests before it.
fn serve(answer: impl Fn(&str, usize) -> (u16, String) + Send + 'static) -> Loop {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
    listener.set_nonblocking(true).expect("a non-blocking listener");
    let port = listener.local_addr().expect("the listener has an address").port();
    let (stop, seen) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(Vec::new())));
    let (stop_in, seen_in) = (stop.clone(), seen.clone());
    let handle = std::thread::spawn(move || {
        while !stop_in.load(Ordering::SeqCst) {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(4));
                continue;
            };
            stream.set_nonblocking(false).expect("a blocking stream");
            let request = read_request(&stream);
            let (code, body) = answer(&request, seen_in.lock().unwrap().len());
            seen_in.lock().unwrap().push(request);
            let response = format!(
                "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    Loop { base: format!("http://127.0.0.1:{port}/functions/v1"), stop, seen, handle: Some(handle) }
}

/// The mail-pull replies in order, then an empty pull. A `mail-resolve` call is answered with no answers
/// (so a claim that needed one stays ambiguous) and takes no reply from the pulls.
fn pulls(replies: Vec<Value>) -> Loop {
    resolving(replies, |_| json!({ "answers": [] }))
}

/// [`pulls`] with `resolve` answering each `mail-resolve` request body.
fn resolving(replies: Vec<Value>, resolve: impl Fn(&Value) -> Value + Send + 'static) -> Loop {
    let bodies: Vec<String> = replies.iter().map(knowlu_engine::ledger::dumps_value).collect();
    let pulled = AtomicUsize::new(0);
    serve(move |request, _| {
        if request.starts_with("POST /functions/v1/mail-resolve ") {
            let body: Value = serde_json::from_str(body_of(request)).unwrap_or(Value::Null);
            return (200, knowlu_engine::ledger::dumps_value(&resolve(&body)));
        }
        let n = pulled.fetch_add(1, Ordering::SeqCst);
        (200, bodies.get(n).cloned().unwrap_or_else(|| pull(vec![], json!([]), false).to_string()))
    })
}

fn pull(rows: Vec<Value>, mailboxes: Value, more: bool) -> Value {
    json!({ "messages": rows, "mailboxes": mailboxes, "more": more })
}

// ---------------------------------------------------------------------------
// Vaults, rows and claims
// ---------------------------------------------------------------------------

fn scratch(tag: &str) -> PathBuf {
    let vault = std::env::temp_dir().join(format!("knowlu-mailpass-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&vault);
    for folder in ["tasks", "approvals", "archive", "info", "state", "config"] {
        std::fs::create_dir_all(vault.join(folder)).unwrap();
    }
    vault
}

fn opts() -> Options<'static> {
    Options { via: "local-runner", run_id: None, runtime: None, model: None, log_dir: None, limit: 10, budget: BATCH_BUDGET }
}

fn vault_today(vault: &Path) -> Date {
    Timestamp::now().to_zoned(knowlu_engine::cli::vault_zone(vault)).date()
}

/// A day in the future, as a due date.
fn ahead(today: Date, days: i64) -> String {
    today.checked_add(jiff::Span::new().days(days)).unwrap().to_string()
}

fn agent_ctx(actor: &str) -> knowlu_engine::write::WriteContext {
    knowlu_engine::write::WriteContext::new(actor, "local-runner")
}

/// The vault's one human token (`config/actor.yaml`, else the default).
fn human(vault: &Path) -> &'static str {
    knowlu_engine::journal::read_human_actor(vault).expect("a readable human actor")
}

/// A vault on its second day: its journal's first record is today, and the pass is run for tomorrow.
fn day_two(tag: &str) -> (PathBuf, Date) {
    let vault = scratch(tag);
    file(&vault, "tasks/seed.md", "---\ntitle: Seed\ncourse: CS 101\ndue: 2030-01-01\nstatus: active\n---\n\nSeed.\n", "agent:knowlu.test");
    let tomorrow = vault_today(&vault).tomorrow().unwrap();
    (vault, tomorrow)
}

fn file(vault: &Path, rel: &str, text: &str, actor: &str) {
    let mut journal = knowlu_engine::journal::Journal::new(vault);
    knowlu_engine::write::create(vault, rel, text, &agent_ctx(actor), &mut journal, None).expect("the note files");
}

fn read(vault: &Path, rel: &str) -> String {
    knowlu_engine::pystr::read_text(&vault.join(rel)).unwrap_or_else(|_| panic!("{rel} is missing"))
}

fn field(vault: &Path, rel: &str, key: &str) -> Option<String> {
    let (meta, _) = knowlu_engine::models::split_frontmatter(&read(vault, rel)).unwrap();
    knowlu_engine::yaml::opt_text(knowlu_engine::yaml::get(&meta, key))
}

fn files(vault: &Path, folder: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(vault.join(folder))
        .map(|dir| dir.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

fn journal_bytes(vault: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(vault.join("state/journal"))
        .map(|dir| dir.flatten().map(|e| (e.file_name().to_string_lossy().to_string(), std::fs::read(e.path()).unwrap())).collect())
        .unwrap_or_default();
    out.sort();
    out
}

fn row(uid: &str, ordering: &str, phase: &str, claims: Vec<Value>) -> Value {
    json!({
        "uid": uid, "message_date": ordering, "ordering_date": ordering,
        "sender_name": "Prof Example", "sender_addr": "prof@example.edu",
        "phase": phase, "authenticity": "aligned", "claims": claims, "judgment_id": null,
    })
}

fn claim(kind: &str, course: Option<&str>) -> Value {
    json!({
        "kind": kind, "course": course, "target": null, "field": null, "value": null, "item": null,
        "topic": null, "evidence": "Lab 5 is now due Monday", "why": "The instructor moved the due date",
        "confidence": 0.9, "source": "model", "resolution": null,
    })
}

fn new_item(title: &str, course: &str, due: Option<&str>, item_kind: &str) -> Value {
    let mut c = claim("new_item", Some(course));
    c["item"] = json!({ "title": title, "due": due, "item_kind": item_kind, "effort_hours": 2.5, "importance": 4 });
    c
}

fn change(target: &str, course: &str, value: &str) -> Value {
    let mut c = claim("change", Some(course));
    c["target"] = json!(target);
    c["field"] = json!("due");
    c["value"] = json!(value);
    c
}

fn course_info(course: &str) -> Value {
    let mut c = claim("course_info", Some(course));
    c["topic"] = json!("room");
    c
}

fn receipt(target: &str, course: &str) -> Value {
    let mut c = claim("receipt", Some(course));
    c["target"] = json!(target);
    c["source"] = json!("template");
    c
}

fn run(vault: &Path, server: &Loop, today: Date) -> Vec<String> {
    pull_mail_on(vault, &server.client(), &opts(), BATCH_BUDGET, today)
}

const T1: &str = "2026-09-20T10:00:00Z";
const T2: &str = "2026-09-21T10:00:00Z";
const T3: &str = "2026-09-22T10:00:00Z";

// ---------------------------------------------------------------------------
// The line, the ack and the seen ledger (spec test 28)
// ---------------------------------------------------------------------------

/// §4.8 step 7's signed line, exact. A residual `NotAmendable` counts in `U` (PQ7, answered
/// 2026-10-02), so the line is unchanged. Two pulls, so a claim can be superseded by a stamp an
/// earlier pull wrote.
#[test]
fn the_mail_run_line_is_exact() {
    let vault = scratch("line");
    let today = vault_today(&vault);
    file(
        &vault,
        "archive/old-lab.md",
        &format!("---\ntitle: Old Lab\ncourse: CS 101\ndue: {}\nstatus: archived\n---\n\nThe student archived it.\n", ahead(today, 3)),
        human(&vault),
    );
    let mut ghost = change("Ghost", "CS 101", &ahead(today, 9));
    ghost["resolution"] = json!({ "target_id": null });
    let mailboxes = json!([{ "address_id": "addr-1", "status": "active", "reenabled_at": null, "unbound": 0, "unauthenticated": 0, "expired": 0 }]);
    let first = pull(
        vec![
            row("mail:aa11", T1, "backfill", vec![
                new_item("Lab 1", "CS 101", Some(&ahead(today, 10)), "task"),
                new_item("Lab 2", "CS 101", Some(&ahead(today, 12)), "task"),
            ]),
            row("mail:bb22", T2, "backfill", vec![change("Lab 1", "CS 101", &format!("{}T23:59", ahead(today, 14)))]),
        ],
        mailboxes.clone(),
        true,
    );
    let second = pull(
        vec![
            row("mail:cc33", T3, "live", vec![
                new_item("Project", "CS 101", Some(&ahead(today, 20)), "task"),
                change("Lab 2", "CS 101", &ahead(today, 15)),
                course_info("CS 101"),
                ghost,
                new_item("Talk", "CS 101", Some(&ahead(today, -3)), "event"),
                change("Old Lab", "CS 101", &ahead(today, 9)),
            ]),
            row("mail:dd44", "2026-09-10T10:00:00Z", "live", vec![change("Lab 1", "CS 101", &ahead(today, 30))]),
        ],
        mailboxes,
        false,
    );
    let server = pulls(vec![first, second]);
    let lines = run(&vault, &server, today);
    assert_eq!(
        lines,
        ["mail: 2 new, 1 changed, 2 proposed, 1 notices, 1 past, 1 superseded, 2 unmatched".to_string()],
        "N, M, K, J, P, S, U; the not-amendable change and the unmatched claim are both U"
    );
    assert_eq!(files(&vault, "tasks"), ["cs-101-lab-1.md", "cs-101-lab-2.md"]);
    assert_eq!(field(&vault, "tasks/cs-101-lab-1.md", "due").unwrap(), format!("{}T23:59", ahead(today, 14)));
    assert_eq!(field(&vault, "tasks/cs-101-lab-1.md", "created_by").unwrap(), "mail");
    let _ = std::fs::remove_dir_all(&vault);
}

/// A receipt counts in `K` when `propose_done` files a card, and nowhere when it has already asked.
#[test]
fn a_receipt_counts_as_proposed_once_and_never_twice() {
    let vault = scratch("receipt");
    let today = vault_today(&vault);
    file(&vault, "tasks/essay-1.md", &format!("---\ntitle: Essay 1\ncourse: CS 101\ndue: {}\nstatus: active\n---\n\nWrite.\n", ahead(today, 4)), "agent:knowlu.test");
    let ping = |uid: &str| pulls(vec![pull(vec![row(uid, T1, "live", vec![receipt("Essay 1", "CS 101")])], json!([]), false)]);
    let lines = run(&vault, &ping("mail:aa11"), today);
    assert_eq!(lines, ["mail: 0 new, 0 changed, 1 proposed, 0 notices, 0 past, 0 superseded, 0 unmatched".to_string()]);
    assert_eq!(files(&vault, "approvals").len(), 1);
    let lines = run(&vault, &ping("mail:bb22"), today);
    assert_eq!(lines, ["mail: 0 new, 0 changed, 0 proposed, 0 notices, 0 past, 0 superseded, 0 unmatched".to_string()]);
    assert_eq!(files(&vault, "approvals").len(), 1, "it asks once per task");
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn no_connected_mailbox_prints_nothing() {
    let vault = scratch("silent");
    let server = pulls(vec![pull(vec![], json!([]), false)]);
    let lines = run(&vault, &server, vault_today(&vault));
    assert!(lines.is_empty(), "{lines:?}");
    assert_eq!(server.seen.lock().unwrap().len(), 1, "one pull, and nothing to acknowledge");
    assert!(journal_bytes(&vault).is_empty(), "nothing was written");
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn seen_records_mail_uids_and_the_ack_sends_them() {
    let vault = scratch("ack");
    let today = vault_today(&vault);
    let claims = vec![new_item("Lab 1", "CS 101", Some(&ahead(today, 10)), "task")];
    let mut server = pulls(vec![pull(
        vec![row("mail:aa11", T1, "backfill", claims), row("mail:bb22", T2, "live", vec![claim("noise", None)])],
        json!([]),
        false,
    )]);
    let lines = run(&vault, &server, today);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let seen = knowlu_engine::ingest::load_seen(&vault);
    assert!(seen.contains("mail:aa11") && seen.contains("mail:bb22"), "{seen:?}");
    let ledger = read(&vault, "state/ingest-seen.md");
    assert!(ledger.contains("- mail:aa11 \u{b7} (email) \u{b7} first seen "), "{ledger}");
    assert_eq!(server.ack_of(0), Vec::<String>::new(), "the first pull acknowledges nothing");
    assert_eq!(server.ack_of(1), ["mail:aa11", "mail:bb22"], "the written uids go back as the ack");
    let _ = std::fs::remove_dir_all(&vault);
}

fn lab_row(n: usize, today: Date, phase: &str) -> Value {
    let title = format!("Lab {n}");
    row(&format!("mail:r{n:02}"), &format!("2026-09-{:02}T10:00:00Z", 10 + n), phase, vec![new_item(&title, "CS 101", Some(&ahead(today, 10 + n as i64)), "task")])
}

/// Live mail takes up to `PULL_ROUNDS`; a closed batch (backfill) drains past them.
#[test]
fn a_closed_batch_drains_past_three_rounds_until_the_budget_is_spent() {
    let vault = scratch("drain");
    let today = vault_today(&vault);
    let rounds: Vec<Value> = (1..=5).map(|n| pull(vec![lab_row(n, today, "backfill")], json!([]), n < 5)).collect();
    let mut server = pulls(rounds);
    let lines = run(&vault, &server, today);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(files(&vault, "tasks").len(), 5, "all five rounds of the batch were applied");
    assert_eq!(server.requests().len(), 6, "five pulls and the last ack");

    let live = scratch("drain-live");
    let rounds: Vec<Value> = (1..=5).map(|n| pull(vec![lab_row(n, today, "live")], json!([]), true)).collect();
    let mut server = pulls(rounds);
    let lines = run(&live, &server, today);
    assert_eq!(server.requests().len(), 4, "three live rounds and the last ack");
    assert_eq!(files(&live, "approvals").len(), 3);
    assert!(lines.iter().any(|l| l.contains("left for the next slot")), "{lines:?}");

    // The budget is checked before each round: with less than one call's time left, no request is made.
    let none = scratch("drain-budget");
    let mut server = pulls(vec![pull(vec![lab_row(1, today, "backfill")], json!([]), false)]);
    let lines = pull_mail_on(&none, &server.client(), &opts(), Duration::from_secs(1), today);
    assert!(lines.is_empty(), "{lines:?}");
    assert!(server.requests().is_empty(), "no budget, no request");
    for v in [&vault, &live, &none] {
        let _ = std::fs::remove_dir_all(v);
    }
}

/// Every note of the vault's folders but `info/` (its file names are random) and the seen ledger, with
/// the random `id:` lines removed.
fn snapshot(vault: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for folder in ["tasks", "approvals", "archive", "state"] {
        for name in files(vault, folder) {
            let rel = format!("{folder}/{name}");
            if !name.ends_with(".md") {
                continue;
            }
            let text: Vec<String> = read(vault, &rel).lines().filter(|l| !l.starts_with("id: ")).map(str::to_string).collect();
            out.push((rel, text.join("\n")));
        }
    }
    out
}

#[test]
fn a_backfill_split_across_pulls_gives_the_same_vault_as_one_pull() {
    let (one, split) = (scratch("split-one"), scratch("split-two"));
    let today = vault_today(&one);
    let mut cancel = change("Lab 5", "CS 101", "true");
    cancel["field"] = json!("cancelled");
    let rows = vec![
        row("mail:aa11", T1, "backfill", vec![new_item("Lab 5", "CS 101", Some(&ahead(today, 10)), "task"), new_item("Quiz 2", "CS 101", None, "task")]),
        row("mail:bb22", T2, "backfill", vec![change("Lab 5", "CS 101", &ahead(today, 14)), course_info("CS 101")]),
        row("mail:cc33", T3, "backfill", vec![cancel, change("Quiz 2", "CS 101", &ahead(today, 20))]),
    ];
    let whole = pulls(vec![pull(rows.clone(), json!([]), false)]);
    run(&one, &whole, today);
    let parts = pulls(vec![pull(rows[..1].to_vec(), json!([]), true), pull(rows[1..].to_vec(), json!([]), false)]);
    run(&split, &parts, today);
    let (a, b) = (snapshot(&one), snapshot(&split));
    assert!(a.iter().any(|(rel, _)| rel.starts_with("tasks/")) && a.iter().any(|(rel, _)| rel.starts_with("approvals/")), "{a:?}");
    assert_eq!(a, b, "the same messages give the same vault however the pulls split them");
    assert_eq!((files(&one, "info").len(), files(&split, "info").len()), (1, 1), "one notice, whole or split");
    for v in [&one, &split] {
        let _ = std::fs::remove_dir_all(v);
    }
}

// ---------------------------------------------------------------------------
// Two desktops, no device judgment, the hostile title (22e, 28c, R11)
// ---------------------------------------------------------------------------

/// §4.8 step 4: a uid already on a note or card is another desktop's write, which sync brought here.
#[test]
fn a_claim_already_on_a_synced_note_or_card_writes_nothing() {
    let vault = scratch("synced");
    let today = vault_today(&vault);
    let note = format!(
        "---\ntitle: Lab 5\ncourse: CS 101\ndue: {}\nstatus: active\ncreated_by: mail\nsource_uid: mail:aa11:0\n---\n\nFrom email.\n",
        ahead(today, 10)
    );
    file(&vault, "tasks/cs-101-lab-5.md", &note, MAIL_ACTOR);
    let card = "---\ntype: approval\nkind: task\ntitle: Lab 6\nstatus: pending\nproposed_at: 2026-09-30\nfirst_proposed_at: 2026-09-30\n\
                expires: 2026-10-14\nsnooze_until: null\ncreated_by: mail\nsource_uid: mail:aa11:1\n---\n\nFrom email.\n\n\
                ```task\n---\ntitle: Lab 6\ncourse: CS 101\ndue: 2030-01-01\nstatus: active\n---\n\nFrom email.\n```\n";
    file(&vault, "approvals/task-lab-6.md", card, MAIL_ACTOR);
    let (tasks, approvals, journal) = (files(&vault, "tasks"), files(&vault, "approvals"), journal_bytes(&vault));
    let claims = vec![new_item("Lab 5", "CS 101", Some(&ahead(today, 10)), "task"), new_item("Lab 6", "CS 101", Some("2030-01-01"), "task")];
    for phase in ["backfill", "live"] {
        let mut server = pulls(vec![pull(vec![row("mail:aa11", T1, phase, claims.clone())], json!([]), false)]);
        let lines = run(&vault, &server, today);
        assert_eq!(lines, ["mail: 0 new, 0 changed, 0 proposed, 0 notices, 0 past, 0 superseded, 0 unmatched".to_string()]);
        assert_eq!((files(&vault, "tasks"), files(&vault, "approvals")), (tasks.clone(), approvals.clone()), "{phase}");
        assert_eq!(journal_bytes(&vault), journal, "{phase}: not one journal record");
        assert_eq!(server.ack_of(1), ["mail:aa11"], "{phase}: it is acknowledged");
        let _ = std::fs::remove_file(vault.join("state/ingest-seen.md"));
    }
    let _ = std::fs::remove_dir_all(&vault);
}

fn spy_answer(request: &str, _: usize) -> (u16, String) {
    const ANSWERED: &str = r#"{"verdict":{"course":"cs-101","effort_hours":3,"importance":3,"importance_reason":"x","confidence":0.8},"tier":3,"outcome":"answered","model":"m","prompt_version":"task-1","grammar_version":"task-1"}"#;
    let line = request.lines().next().unwrap_or("");
    if line.starts_with("GET /functions/v1/judge-rules") {
        (200, r#"{"proposals":[]}"#.to_string())
    } else if line.starts_with("POST /functions/v1/gmail-read") {
        (200, r#"{"items":[],"read":0,"quiet":true,"reason":"no_gmail_scope","more":false}"#.to_string())
    } else if line.starts_with("POST /functions/v1/mail-pull") {
        (200, pull(vec![], json!([]), false).to_string())
    } else if line.starts_with("POST /functions/v1/judge-task") {
        (200, ANSWERED.to_string())
    } else {
        (404, r#"{"error":"not found"}"#.to_string())
    }
}

/// Spec test 28c: a mail note is written `needs_enrichment: false`, so `judge` never sends its title or
/// body to `judge-task`. A control note that does need judging proves the spy would have seen one.
#[test]
fn judge_makes_no_judge_task_call_for_a_mail_note() {
    let vault = scratch("spy");
    let today = vault_today(&vault);
    let server = pulls(vec![pull(vec![row("mail:aa11", T1, "backfill", vec![new_item("Lab Nine", "CS 101", Some(&ahead(today, 10)), "task")])], json!([]), false)]);
    run(&vault, &server, today);
    let rel = "tasks/cs-101-lab-nine.md";
    assert_eq!(field(&vault, rel, "needs_enrichment").unwrap(), "false");
    let control = "---\ntitle: Control essay\ncourse: cs-101\ndue: 2030-01-01\nstatus: active\ncreated_by: blackboard\nsource_uid: bb:1\n\
                   effort_hours: 1.0\neffort_source: inferred\nneeds_enrichment: true\n---\n\nWrite it.\n";
    file(&vault, "tasks/control-essay.md", control, "agent:knowlu.test");
    let before = read(&vault, rel);

    let mut spy = serve(spy_answer);
    let (code, _) = knowlu_engine::enrich::run_lines_with(&vault, &opts(), Some(&spy.client()));
    assert_eq!(code, 0);
    let requests = spy.requests();
    let judged: Vec<&String> = requests.iter().filter(|r| r.starts_with("POST /functions/v1/judge-task")).collect();
    assert_eq!(judged.len(), 1, "only the control note is judged: {requests:?}");
    assert!(judged[0].contains("Control essay"));
    assert!(requests.iter().all(|r| !r.contains("Lab Nine")), "no request carries the mail note's title");
    assert_eq!(read(&vault, rel), before, "the mail note is untouched");
    let _ = std::fs::remove_dir_all(&vault);
}

/// Plan §10, R11: a claim titled with a line break and a forged ledger line is noise; the uid is the only
/// thing recorded, under the fixed title `(email)`, so it cannot add a line.
#[test]
fn the_seen_ledger_records_mail_uids_as_email_and_a_hostile_title_cannot_add_a_line() {
    let vault = scratch("hostile");
    let today = vault_today(&vault);
    let hostile = new_item("Quiz\u{2028}- bb-uid-123 \u{b7} x", "CS 101", Some(&ahead(today, 5)), "task");
    let rows = vec![
        row("mail:aa11", T1, "live", vec![hostile]),
        row("mail:bb22", T2, "live", vec![new_item("Clean title", "CS 101", Some(&ahead(today, 6)), "task")]),
    ];
    let server = pulls(vec![pull(rows, json!([]), false)]);
    run(&vault, &server, today);
    let seen = knowlu_engine::ingest::load_seen(&vault);
    assert!(seen.contains("mail:aa11") && seen.contains("mail:bb22"), "{seen:?}");
    assert!(!seen.contains("bb-uid-123"), "{seen:?}");
    let ledger = read(&vault, "state/ingest-seen.md");
    assert!(!ledger.contains("Quiz") && !ledger.contains("Clean title") && !ledger.contains("bb-uid-123"), "{ledger}");
    let entries: Vec<&str> = knowlu_engine::pystr::splitlines(&ledger).into_iter().filter(|l| l.starts_with("- ")).collect();
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(files(&vault, "approvals").len(), 1, "the hostile claim files nothing");
    let _ = std::fs::remove_dir_all(&vault);
}

// ---------------------------------------------------------------------------
// Heads-up items (§4.10, spec test 36's engine half)
// ---------------------------------------------------------------------------

fn mailbox(id: &str, status: &str, reenabled: Option<&str>) -> Value {
    json!({ "address_id": id, "status": status, "reenabled_at": reenabled, "unbound": 0, "unauthenticated": 0, "expired": 0 })
}

/// The open `info/` items with this `close_key`, as (path, kind, opened_by, body).
fn open_items(vault: &Path, key: &str) -> Vec<(String, String, String, String)> {
    knowlu_engine::info::list_info(vault)
        .into_iter()
        .filter(|m| knowlu_engine::yaml::opt_text(knowlu_engine::yaml::get(m, "close_key")).as_deref() == Some(key))
        .map(|m| {
            let text = |k: &str| knowlu_engine::yaml::opt_text(knowlu_engine::yaml::get(&m, k)).unwrap_or_default();
            let path = text("path");
            let (_, body) = knowlu_engine::models::split_frontmatter(&read(vault, &path)).unwrap();
            (path, text("kind"), text("opened_by"), body.trim().to_string())
        })
        .collect()
}

fn only_boxes(vault: &Path, today: Date, boxes: Value) -> Vec<String> {
    run(vault, &pulls(vec![pull(vec![], boxes, false)]), today)
}

#[test]
fn a_non_active_mailbox_keeps_one_heads_up_and_names_its_line() {
    let vault = scratch("heads");
    let today = vault_today(&vault);
    let both = json!([mailbox("addr-1", "quiet", None), mailbox("addr-2", "active", None)]);
    let lines = only_boxes(&vault, today, both.clone());
    assert!(lines[0].starts_with("mail: 0 new, 0 changed"), "{lines:?}");
    let named: Vec<&String> = lines.iter().filter(|l| l.contains("addr-1")).collect();
    assert!(named.len() == 1 && named[0].contains("quiet"), "{lines:?}");
    assert!(!lines.iter().any(|l| l.contains("addr-2")), "an active mailbox adds no line: {lines:?}");
    let items = open_items(&vault, "mail-source:addr-1");
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!((items[0].1.as_str(), items[0].2.as_str()), ("heads-up", MAIL_ACTOR));
    assert!(open_items(&vault, "mail-source:addr-2").is_empty());

    let (info, journal) = (files(&vault, "info"), journal_bytes(&vault));
    only_boxes(&vault, today, both);
    assert_eq!((files(&vault, "info"), journal_bytes(&vault)), (info, journal), "a second slot keeps the one item and writes nothing");

    let lines = only_boxes(&vault, today, json!([mailbox("addr-1", "active", None)]));
    assert!(lines.iter().all(|l| !l.contains("addr-1")), "{lines:?}");
    assert!(open_items(&vault, "mail-source:addr-1").is_empty(), "closed when the mailbox is active again");
    assert_eq!(files(&vault, "archive").len(), 1, "the closed item is settled into archive/");
    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn a_reenabled_mailbox_opens_one_heads_up_with_the_exact_text() {
    let vault = scratch("reenabled");
    let today = vault_today(&vault);
    let boxes = json!([mailbox("addr-1", "active", Some("2026-10-01T12:00:00Z"))]);
    let lines = only_boxes(&vault, today, boxes.clone());
    assert_eq!(lines.len(), 1, "an active mailbox adds no named line: {lines:?}");
    let items = open_items(&vault, "mail-reenabled:addr-1");
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].1, "heads-up");
    assert_eq!(items[0].2, MAIL_ACTOR);
    assert_eq!(items[0].3, "Email forwarding was off; Knowlu turned it back on. Turn off email in Settings to stop this.");
    assert!(open_items(&vault, "mail-source:addr-1").is_empty());

    only_boxes(&vault, today, boxes.clone());
    assert_eq!(open_items(&vault, "mail-reenabled:addr-1").len(), 1, "the same stamp opens it once");

    // The student dismisses it; the same `reenabled_at` does not open it again.
    let mut journal = knowlu_engine::journal::Journal::new(&vault);
    let ctx = knowlu_engine::write::WriteContext::new(human(&vault), "dashboard");
    knowlu_engine::info::close_info(&vault, Some("mail-reenabled:addr-1"), None, human(&vault), &ctx, Some(&mut journal), None).unwrap();
    only_boxes(&vault, today, boxes);
    assert!(open_items(&vault, "mail-reenabled:addr-1").is_empty(), "a dismissed heads-up stays dismissed");
    // A later re-enable is a new stamp, so it opens one.
    only_boxes(&vault, today, json!([mailbox("addr-1", "active", Some("2026-10-05T08:00:00Z"))]));
    assert_eq!(open_items(&vault, "mail-reenabled:addr-1").len(), 1);
    let _ = std::fs::remove_dir_all(&vault);
}

// ---------------------------------------------------------------------------
// The deck: the budget, a rule forward, a claim waiting on resolve
// ---------------------------------------------------------------------------

/// Plan §10, R3: mail cards are charged to the 15-a-day budget like every card, so `defer_over_budget`
/// snoozes the overflow to tomorrow and deletes none.
#[test]
fn twenty_mail_cards_are_charged_and_overflow_is_snoozed_by_defer_over_budget() {
    let (vault, today) = day_two("twenty");
    let claims: Vec<Value> = (1..=20).map(|n| new_item(&format!("Task {n:02}"), "CS 101", Some(&ahead(today, 10)), "task")).collect();
    let server = pulls(vec![pull(vec![row("mail:aa11", T1, "live", claims)], json!([]), false)]);
    let lines = run(&vault, &server, today);
    assert_eq!(lines, ["mail: 0 new, 0 changed, 20 proposed, 0 notices, 0 past, 0 superseded, 0 unmatched".to_string()]);
    let cards = files(&vault, "approvals");
    assert_eq!(cards.len(), 20);
    for name in &cards {
        let rel = format!("approvals/{name}");
        assert_eq!(field(&vault, &rel, "status").unwrap(), "pending", "{rel}: a day-2 card shows today");
        assert_eq!(field(&vault, &rel, "proposed_at").unwrap(), today.to_string());
    }
    let mut journal = knowlu_engine::journal::Journal::new(&vault);
    let deferred = knowlu_engine::approvals::defer_over_budget(&vault, today, 15, &agent_ctx("agent:rank"), &mut journal);
    assert_eq!(deferred.len(), 5);
    assert_eq!(files(&vault, "approvals"), cards, "none is deleted");
    let tomorrow = today.tomorrow().unwrap().to_string();
    let snoozed: Vec<&String> = cards.iter().filter(|n| field(&vault, &format!("approvals/{n}"), "status").unwrap() == "snoozed").collect();
    assert_eq!(snoozed.len(), 5);
    for name in snoozed {
        let rel = format!("approvals/{name}");
        assert_eq!(field(&vault, &rel, "proposed_at").unwrap(), tomorrow);
        assert_eq!(field(&vault, &rel, "snooze_until").unwrap(), tomorrow);
        assert_eq!(field(&vault, &rel, "first_proposed_at").unwrap(), today.to_string(), "first_proposed_at never moves");
    }
    let _ = std::fs::remove_dir_all(&vault);
}

/// T5.6 against real card files (S0 capture 5's shape, synthetic): the view must carry a card's `kind` and
/// its course from the fenced payload, or the rule forward files a second card.
fn a_rule_forward(vault: &Path, today: Date, day: &str) {
    let first = pulls(vec![pull(vec![row("mail:aa11", T1, "live", vec![new_item("Lab 5", "CS 101", Some(&ahead(today, 10)), "task")])], json!([]), false)]);
    run(vault, &first, today);
    let card = "approvals/task-lab-5.md";
    assert_eq!(field(vault, card, "created_by").unwrap(), "mail", "{day}");
    assert_eq!(field(vault, card, "status").unwrap(), if day == "day 1" { "snoozed" } else { "pending" }, "{day}");
    let bytes = read(vault, card);

    let forwarded = vec![new_item("Lab 5", "CS 101", Some(&ahead(today, 10)), "task"), new_item("Lab 5", "MATH 220", Some(&ahead(today, 10)), "task")];
    let second = pulls(vec![pull(vec![row("mail:bb22", T2, "live", forwarded)], json!([]), false)]);
    let lines = run(vault, &second, today);
    assert_eq!(lines, ["mail: 0 new, 0 changed, 1 proposed, 0 notices, 0 past, 0 superseded, 0 unmatched".to_string()], "{day}");
    assert_eq!(files(vault, "approvals"), ["task-lab-5-2.md", "task-lab-5.md"], "{day}: exactly two mail task cards");
    assert_eq!(read(vault, card), bytes, "{day}: the first card is unchanged byte for byte");
    assert!(read(vault, "approvals/task-lab-5-2.md").contains("course: \"MATH 220\""), "{day}");
}

#[test]
fn a_rule_forward_in_a_later_pull_files_no_second_card() {
    let one = scratch("forward-1");
    a_rule_forward(&one, vault_today(&one), "day 1");
    let (two, today) = day_two("forward-2");
    a_rule_forward(&two, today, "day 2");
    for v in [&one, &two] {
        let _ = std::fs::remove_dir_all(v);
    }
}

/// A claim still ambiguous with a course waits on `mail-resolve` (T6.4): it writes nothing, is not counted
/// as unmatched, and its message is neither acknowledged nor recorded, so it comes back.
#[test]
fn an_ambiguous_claim_holds_its_message_unacknowledged_and_unrecorded() {
    let vault = scratch("ambiguous");
    let today = vault_today(&vault);
    for title in ["Lab 5", "Lab 6"] {
        let name = title.to_lowercase().replace(' ', "-");
        file(&vault, &format!("tasks/{name}.md"), &format!("---\ntitle: {title}\ncourse: CS 101\ndue: 2030-01-01\nstatus: active\n---\n\nx\n"), "agent:knowlu.test");
    }
    let rows = vec![
        row("mail:aa11", T1, "live", vec![change("Lab", "CS 101", &ahead(today, 9))]),
        row("mail:bb22", T2, "live", vec![course_info("CS 101")]),
    ];
    let mut server = pulls(vec![pull(rows, json!([]), false)]);
    let lines = run(&vault, &server, today);
    assert_eq!(lines, ["mail: 0 new, 0 changed, 0 proposed, 1 notices, 0 past, 0 superseded, 0 unmatched".to_string()]);
    let seen = knowlu_engine::ingest::load_seen(&vault);
    assert!(seen.contains("mail:bb22") && !seen.contains("mail:aa11"), "{seen:?}");
    assert_eq!(server.ack_of(1), ["mail:bb22"]);
    assert!(files(&vault, "approvals").is_empty());
    let _ = std::fs::remove_dir_all(&vault);
}

/// Two vault files named `Lab 5` and `Lab 6`, so a claim naming `Lab` is ambiguous.
fn two_labs(vault: &Path) {
    for title in ["Lab 5", "Lab 6"] {
        let name = title.to_lowercase().replace(' ', "-");
        file(vault, &format!("tasks/{name}.md"), &format!("---\ntitle: {title}\ncourse: CS 101\ndue: 2030-01-01\nstatus: active\n---\n\nx\n"), "agent:knowlu.test");
    }
}

fn journal_lines(vault: &Path) -> usize {
    journal_bytes(vault).iter().map(|(_, bytes)| bytes.iter().filter(|b| **b == b'\n').count()).sum()
}

/// A message held on `mail-resolve` comes back on every later pull, so the pass plans it once: one open
/// notice, the same `info/` files and one `J`, however many rounds deliver it (plan §9's T5.5 notices row).
#[test]
fn a_held_message_delivered_in_two_rounds_is_planned_once() {
    let held_row = |today: Date| row("mail:aa11", T1, "live", vec![change("Lab", "CS 101", &ahead(today, 9)), course_info("CS 101")]);
    let control = scratch("held-control");
    let today = vault_today(&control);
    two_labs(&control);
    let lines = run(&control, &pulls(vec![pull(vec![held_row(today)], json!([]), false)]), today);
    assert_eq!(lines, ["mail: 0 new, 0 changed, 0 proposed, 1 notices, 0 past, 0 superseded, 0 unmatched".to_string()]);
    let (one_info, one_journal) = (files(&control, "info").len(), journal_lines(&control));

    let vault = scratch("held");
    two_labs(&vault);
    let rounds = vec![
        pull(vec![held_row(today)], json!([]), true),
        pull(vec![held_row(today)], json!([]), true),
        pull(vec![held_row(today)], json!([]), false),
    ];
    let mut server = pulls(rounds);
    let lines = run(&vault, &server, today);
    assert_eq!(lines[0], "mail: 0 new, 0 changed, 0 proposed, 1 notices, 0 past, 0 superseded, 0 unmatched", "{lines:?}");
    assert_eq!(files(&vault, "info").len(), one_info, "one notice file, never closed and reopened");
    assert!(files(&vault, "archive").is_empty(), "no notice was closed into archive/");
    assert_eq!(journal_lines(&vault), one_journal, "the same journal records as one round");
    assert_eq!(open_items(&vault, "mail:CS 101:room").len(), 1);
    assert!(!knowlu_engine::ingest::load_seen(&vault).contains("mail:aa11"));
    assert_eq!(server.calls("mail-pull").len(), 2, "the second round held nothing new, so the pass stopped");
    for v in [&control, &vault] {
        let _ = std::fs::remove_dir_all(v);
    }
}

/// A page of nothing but held messages makes no progress (the server returns it until it is acked), so the
/// drain stops rather than spend its budget re-reading it.
#[test]
fn a_page_of_only_held_messages_ends_the_drain() {
    let vault = scratch("held-page");
    two_labs(&vault);
    let today = vault_today(&vault);
    let held_row = row("mail:aa11", T1, "backfill", vec![change("Lab", "CS 101", &ahead(today, 9))]);
    let page = pull(vec![held_row], json!([]), true);
    let mut server = pulls(vec![page.clone(), page.clone(), page.clone(), page.clone(), page]);
    let lines = run(&vault, &server, today);
    assert_eq!(server.calls("mail-pull").len(), 2, "the second page held nothing new, so the drain stopped: {lines:?}");
    assert!(lines.iter().any(|l| l.contains("more is waiting")), "{lines:?}");
    let _ = std::fs::remove_dir_all(&vault);
}

/// §4.11 step 2: a heads-up that failed to write must not stamp `reenabled_at` as seen, or the student is
/// never told. Here `info/` is a plain file, so the write fails; the next pass, with `info/` back, opens it.
#[test]
fn a_reenabled_heads_up_that_failed_to_write_is_not_stamped_seen() {
    let vault = scratch("reenabled-fail");
    let today = vault_today(&vault);
    std::fs::remove_dir_all(vault.join("info")).unwrap();
    std::fs::write(vault.join("info"), "not a folder").unwrap();
    let boxes = json!([mailbox("addr-1", "active", Some("2026-10-01T12:00:00Z"))]);
    let lines = only_boxes(&vault, today, boxes.clone());
    assert!(lines.iter().any(|l| l.contains("heads-up could not be written")), "{lines:?}");
    assert!(!knowlu_engine::ingest::load_seen(&vault).contains("mail-reenabled:addr-1:2026-10-01T12:00:00Z"), "an unwritten item is never stamped");

    std::fs::remove_file(vault.join("info")).unwrap();
    std::fs::create_dir_all(vault.join("info")).unwrap();
    only_boxes(&vault, today, boxes);
    assert_eq!(open_items(&vault, "mail-reenabled:addr-1").len(), 1, "the next pass opens it");
    let _ = std::fs::remove_dir_all(&vault);
}

// ---------------------------------------------------------------------------
// T6.4: ambiguity resolved per course (spec test 25), labels (28b), `judge` (30)
// ---------------------------------------------------------------------------

/// Two mail notes in each of two courses, so a claim naming `Lab` or `Quiz` is ambiguous and one naming
/// `Ghost` matches nothing. Notes mail wrote are `created_by: mail`, so a backfill change is a direct set.
fn labs_and_quizzes(vault: &Path) {
    for (n, (course, title)) in [("CS 101", "Lab 5"), ("CS 101", "Lab 6"), ("MATH 220", "Quiz 1"), ("MATH 220", "Quiz 2")].into_iter().enumerate() {
        let name = title.to_lowercase().replace(' ', "-");
        let text = format!("---\ntitle: {title}\ncourse: {course}\ndue: 2030-01-01\nstatus: active\ncreated_by: mail\nsource_uid: \"mail:s{n}:0\"\n---\n\nx\n");
        file(vault, &format!("tasks/{name}.md"), &text, "agent:knowlu.test");
    }
}

/// The scripted resolver for [`ambiguous_message`]: `Lab 6` for its claim 0, `Quiz 2` for its claim 2, and
/// "none" for claim 1.
fn scripted(body: &Value) -> Value {
    let course = body["course"].as_str().unwrap_or_default().to_string();
    let pick = |title: &str| {
        let found = body["candidates"].as_array().into_iter().flatten().find(|c| c["title"] == title);
        found.map_or(Value::Null, |c| c["id"].clone())
    };
    let answers: Vec<Value> = body["claims"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| {
            let target = match (course.as_str(), c["index"].as_u64()) {
                ("CS 101", Some(0)) => pick("Lab 6"),
                ("MATH 220", Some(2)) => pick("Quiz 2"),
                _ => Value::Null,
            };
            json!({ "uid": c["uid"], "index": c["index"], "target_id": target })
        })
        .collect();
    json!({ "answers": answers })
}

fn ambiguous_message(today: Date) -> Value {
    row("mail:aa11", T1, "backfill", vec![
        change("Lab", "CS 101", &ahead(today, 9)),
        change("Ghost", "CS 101", &ahead(today, 9)),
        change("Quiz", "MATH 220", &ahead(today, 11)),
    ])
}

/// §4.8 step 3, D12: one `mail-resolve` call per course, carrying references and the course's items
/// (id, title, due) and none of the message's text; the answers land the claims, the "none" is a notice,
/// and the message is acknowledged because every claim was settled.
#[test]
fn one_resolve_call_per_course() {
    let vault = scratch("resolve");
    labs_and_quizzes(&vault);
    let today = vault_today(&vault);
    let mut server = resolving(vec![pull(vec![ambiguous_message(today)], json!([]), false)], scripted);
    let lines = run(&vault, &server, today);
    assert_eq!(lines, ["mail: 0 new, 2 changed, 0 proposed, 0 notices, 0 past, 0 superseded, 1 unmatched".to_string()], "{lines:?}");
    assert_eq!(field(&vault, "tasks/lab-6.md", "due").unwrap(), ahead(today, 9));
    assert_eq!(field(&vault, "tasks/quiz-2.md", "due").unwrap(), ahead(today, 11));
    assert_eq!(field(&vault, "tasks/lab-5.md", "due").unwrap(), "2030-01-01");
    assert!(knowlu_engine::ingest::load_seen(&vault).contains("mail:aa11"), "every claim settled, so the message is recorded");
    assert_eq!(server.ack_of(1), ["mail:aa11"]);
    let calls = server.calls("mail-resolve");
    assert_eq!(calls.len(), 2, "one call per course: {calls:?}");
    let bodies: Vec<Value> = calls.iter().map(|c| serde_json::from_str(body_of(c)).unwrap()).collect();
    assert_eq!(bodies[0]["course"], "CS 101");
    assert_eq!(bodies[1]["course"], "MATH 220");
    assert_eq!(bodies[0]["claims"], json!([{ "uid": "mail:aa11", "index": 0 }, { "uid": "mail:aa11", "index": 1 }]));
    assert_eq!(bodies[1]["claims"], json!([{ "uid": "mail:aa11", "index": 2 }]));
    let titles = |body: &Value| -> Vec<String> { body["candidates"].as_array().unwrap().iter().map(|c| c["title"].as_str().unwrap().to_string()).collect() };
    assert_eq!(titles(&bodies[0]), ["Lab 5", "Lab 6"], "only the course's own items are offered");
    assert_eq!(titles(&bodies[1]), ["Quiz 1", "Quiz 2"]);
    assert_eq!(bodies[0]["candidates"][1]["due"], "2030-01-01");
    for call in &calls {
        assert!(!call.contains("Lab 5 is now due Monday") && !call.contains("prof@example.edu"), "no message text or address travels: {call}");
    }
    let _ = std::fs::remove_dir_all(&vault);
}

/// D12: "none" is an info notice keyed by the claim's own uid, never a guess; nothing else is written.
#[test]
fn a_none_answer_is_a_notice() {
    let vault = scratch("resolve-none");
    labs_and_quizzes(&vault);
    let today = vault_today(&vault);
    let server = resolving(
        vec![pull(vec![row("mail:aa11", T1, "backfill", vec![change("Ghost", "CS 101", &ahead(today, 9))])], json!([]), false)],
        |body| {
            let answers: Vec<Value> = body["claims"].as_array().unwrap().iter().map(|c| json!({ "uid": c["uid"], "index": c["index"], "target_id": null })).collect();
            json!({ "answers": answers })
        },
    );
    let lines = run(&vault, &server, today);
    assert_eq!(lines, ["mail: 0 new, 0 changed, 0 proposed, 0 notices, 0 past, 0 superseded, 1 unmatched".to_string()]);
    let notices = open_items(&vault, "mail:aa11:0");
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert_eq!(notices[0].2, "agent:knowlu.mail");
    assert_eq!(field(&vault, &notices[0].0, "title").unwrap(), "Knowlu could not match this to an item");
    assert_eq!(field(&vault, "tasks/lab-5.md", "due").unwrap(), "2030-01-01");
    assert_eq!(field(&vault, "tasks/lab-6.md", "due").unwrap(), "2030-01-01");
    assert!(files(&vault, "approvals").is_empty());
    let _ = std::fs::remove_dir_all(&vault);
}

/// D12: an answer the service stored comes back on the claim, so a re-pull replays it with no call: a
/// stored id lands the change, a stored "none" is the notice.
#[test]
fn a_stored_resolution_replays_with_zero_calls() {
    let vault = scratch("resolve-stored");
    labs_and_quizzes(&vault);
    let today = vault_today(&vault);
    let lab_6 = field(&vault, "tasks/lab-6.md", "id").unwrap();
    let mut chosen = change("Lab", "CS 101", &ahead(today, 9));
    chosen["resolution"] = json!({ "target_id": lab_6 });
    let mut none = change("Ghost", "CS 101", &ahead(today, 9));
    none["resolution"] = json!({ "target_id": null });
    let mut server = resolving(vec![pull(vec![row("mail:aa11", T1, "backfill", vec![chosen, none])], json!([]), false)], scripted);
    let lines = run(&vault, &server, today);
    assert_eq!(lines, ["mail: 0 new, 1 changed, 0 proposed, 0 notices, 0 past, 0 superseded, 1 unmatched".to_string()], "{lines:?}");
    assert_eq!(field(&vault, "tasks/lab-6.md", "due").unwrap(), ahead(today, 9));
    assert_eq!(field(&vault, "tasks/lab-5.md", "due").unwrap(), "2030-01-01");
    assert_eq!(open_items(&vault, "mail:aa11:1").len(), 1);
    assert!(server.calls("mail-resolve").is_empty(), "a stored answer is replayed, never asked again");
    let _ = std::fs::remove_dir_all(&vault);
}

/// No answer (the call failed) leaves every claim asked: nothing is written for them, the message is held
/// unacknowledged so it comes back, and the failure is one named line that carries no message text.
#[test]
fn a_failed_resolve_holds_the_message_and_is_named() {
    let vault = scratch("resolve-down");
    labs_and_quizzes(&vault);
    let today = vault_today(&vault);
    let body = knowlu_engine::ledger::dumps_value(&pull(vec![ambiguous_message(today)], json!([]), false));
    let mut server = serve(move |request, _| {
        if request.starts_with("POST /functions/v1/mail-resolve ") {
            (503, r#"{"error":"down"}"#.to_string())
        } else {
            (200, body.clone())
        }
    });
    let lines = run(&vault, &server, today);
    assert_eq!(server.calls("mail-resolve").len(), 1, "a failed call ends the asking: the rest would fail the same way");
    assert!(lines.iter().any(|l| l == "mail: could not ask which item (the service refused); left for the next slot"), "{lines:?}");
    assert!(lines[0].starts_with("mail: 0 new, 0 changed, 0 proposed, 0 notices, 0 past, 0 superseded, 0 unmatched"), "{lines:?}");
    assert!(!knowlu_engine::ingest::load_seen(&vault).contains("mail:aa11"), "held, so it comes back");
    assert_eq!(field(&vault, "tasks/lab-6.md", "due").unwrap(), "2030-01-01");
    let _ = std::fs::remove_dir_all(&vault);
}

/// Head of line: a course the service refuses for itself (a 4xx that is not about the session, the
/// entitlement or the rate) is skipped and the next course is still asked, so one course never starves the
/// courses behind it in the order. Its claims stay asked: the message is held and the line names the refusal.
#[test]
fn a_refused_course_is_skipped_and_the_next_is_asked() {
    let vault = scratch("resolve-refused");
    labs_and_quizzes(&vault);
    let today = vault_today(&vault);
    let pulled = knowlu_engine::ledger::dumps_value(&pull(vec![ambiguous_message(today)], json!([]), false));
    let mut server = serve(move |request, _| {
        if request.starts_with("POST /functions/v1/mail-resolve ") {
            let body: Value = serde_json::from_str(body_of(request)).unwrap_or(Value::Null);
            if body["course"] == "CS 101" {
                return (422, r#"{"error":"too many candidates"}"#.to_string());
            }
            return (200, knowlu_engine::ledger::dumps_value(&scripted(&body)));
        }
        (200, pulled.clone())
    });
    let lines = run(&vault, &server, today);
    assert_eq!(server.calls("mail-resolve").len(), 2, "the refused course does not end the asking");
    assert_eq!(field(&vault, "tasks/quiz-2.md", "due").unwrap(), ahead(today, 11), "the later course was answered");
    assert_eq!(field(&vault, "tasks/lab-6.md", "due").unwrap(), "2030-01-01", "the refused course wrote nothing");
    assert!(lines.iter().any(|l| l == "mail: could not ask which item (the service refused); left for the next slot"), "{lines:?}");
    assert!(!knowlu_engine::ingest::load_seen(&vault).contains("mail:aa11"), "its claims are unanswered, so it is held");
    let _ = std::fs::remove_dir_all(&vault);
}

/// Spec test 28b: the device never reports a label on a mail card. A rejected `mail_claim` card is no row
/// (`labels_to_report` is unchanged), while a rejected `task` card beside it still is, so the test is not
/// vacuous.
#[test]
fn a_rejected_mail_card_is_never_reported() {
    const MAIL_JUDGMENT: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";
    const TASK_JUDGMENT: &str = "7c9e6679-7425-40de-944b-e07fc1f90ae7";
    let (vault, today) = day_two("labels");
    let mut mail = row("mail:aa11", T1, "live", vec![new_item("Lab 5", "CS 101", Some(&ahead(today, 10)), "task")]);
    mail["judgment_id"] = json!(MAIL_JUDGMENT);
    run(&vault, &pulls(vec![pull(vec![mail], json!([]), false)]), today);
    let card = "approvals/task-lab-5.md";
    assert_eq!(field(&vault, card, "judgment_kind").unwrap(), "mail_claim");
    assert_eq!(field(&vault, card, "judgment_id").unwrap(), MAIL_JUDGMENT, "the card is judged, so only its kind keeps it unreported");

    let student = knowlu_engine::write::WriteContext::new(human(&vault), "dashboard");
    let mut journal = knowlu_engine::journal::Journal::new(&vault);
    let reject = |rel: &str, journal: &mut knowlu_engine::journal::Journal| {
        let status = [("status".to_string(), "rejected".to_string())];
        knowlu_engine::write::write_literals(&vault, rel, &status, &student, journal, &knowlu_engine::write::WriteOpts::default()).expect("the answer is written");
    };
    reject(card, &mut journal);
    assert!(!knowlu_engine::enrich::labels_waiting(&vault), "a rejected mail card is no label");

    let control = format!(
        "---\ntype: approval\nkind: task\ntitle: A judged task\nstatus: pending\nsource_uid: \"gmail:1\"\nproposed_at: {today}\n\
         first_proposed_at: {today}\nexpires: 2030-01-01\nsnooze_until: null\ncreated_by: gmail\njudgment_id: {TASK_JUDGMENT}\n\
         judgment_kind: task\n---\n\nbody\n"
    );
    file(&vault, "approvals/task-control.md", &control, "agent:knowlu.enrich");
    reject("approvals/task-control.md", &mut journal);
    assert!(knowlu_engine::enrich::labels_waiting(&vault), "a rejected task card is");
    let mut server = serve(|_, _| (200, r#"{"corrections":1,"unowned":0,"refused":0}"#.to_string()));
    knowlu_engine::enrich::report_labels(&vault, &server.client(), &opts(), BATCH_BUDGET);
    let sent = server.calls("telemetry");
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert!(sent[0].contains(TASK_JUDGMENT) && !sent[0].contains(MAIL_JUDGMENT), "only the task card's label travels: {}", sent[0]);
    let _ = std::fs::remove_dir_all(&vault);
}

/// The cloud arm's endpoints for the `judge` tests, `mail_pull` answering as the test says.
fn judge_server(mail_pull: impl Fn(usize) -> (u16, String) + Send + 'static) -> Loop {
    let pulled = AtomicUsize::new(0);
    serve(move |request, _| {
        let line = request.lines().next().unwrap_or("");
        if line.starts_with("GET /functions/v1/judge-rules") {
            (200, r#"{"proposals":[]}"#.to_string())
        } else if line.starts_with("POST /functions/v1/gmail-read") {
            (200, r#"{"items":[],"read":0,"quiet":true,"reason":"no_gmail_scope","more":false}"#.to_string())
        } else if line.starts_with("POST /functions/v1/mail-pull") {
            mail_pull(pulled.fetch_add(1, Ordering::SeqCst))
        } else {
            (404, r#"{"error":"not found"}"#.to_string())
        }
    })
}

/// Spec test 30's `judge` half: the cloud arm runs the mail pass after the Gmail pull, inside the same
/// gate, and exits 0 whatever the mail service answers; a refusal, a throttle, a server error and an
/// unreadable body are each a named line, never a failed slot.
#[test]
fn judge_runs_the_mail_pass_after_the_gmail_pull_and_exits_zero_on_every_mail_failure() {
    let vault = scratch("judge-mail");
    let today = vault_today(&vault);
    let first = pull(vec![row("mail:aa11", T1, "backfill", vec![new_item("Lab Nine", "CS 101", Some(&ahead(today, 10)), "task")])], json!([]), false);
    let mut server = judge_server(move |n| (200, if n == 0 { first.to_string() } else { pull(vec![], json!([]), false).to_string() }));
    let (code, lines) = knowlu_engine::enrich::run_lines_with(&vault, &opts(), Some(&server.client()));
    assert_eq!(code, 0, "{lines:?}");
    assert!(lines.iter().any(|l| l == "mail: 1 new, 0 changed, 0 proposed, 0 notices, 0 past, 0 superseded, 0 unmatched"), "{lines:?}");
    assert_eq!(field(&vault, "tasks/cs-101-lab-nine.md", "created_by").unwrap(), "mail");
    let requests = server.requests();
    let at = |what: &str| requests.iter().position(|r| r.contains(what)).unwrap_or_else(|| panic!("no {what} request: {requests:?}"));
    assert!(at("/functions/v1/gmail-read") < at("/functions/v1/mail-pull"), "the Gmail pull comes first");
    let _ = std::fs::remove_dir_all(&vault);

    for (code, body) in [(500, r#"{"error":"x"}"#), (401, r#"{"error":"x"}"#), (402, r#"{"error":"x"}"#), (403, r#"{"error":"x"}"#), (429, r#"{"error":"x"}"#), (200, "not json")] {
        let vault = scratch("judge-mail-down");
        let server = judge_server(move |_| (code, body.to_string()));
        let (exit, lines) = knowlu_engine::enrich::run_lines_with(&vault, &opts(), Some(&server.client()));
        assert_eq!(exit, 0, "{code}: {lines:?}");
        assert!(lines.iter().any(|l| l.starts_with("mail: skipped (")), "{code}: a named line, {lines:?}");
        let _ = std::fs::remove_dir_all(&vault);
    }
}

// ---------------------------------------------------------------------------
// PQ14 (Quinn, 2026-10-04): a claim `mail-resolve` never answers is held three slots, then given up
// ---------------------------------------------------------------------------

/// `state/mail-resolve-held.json`: each claim uid still unanswered, with the slots that asked and got
/// no answer. `Null` when the file is absent.
fn held_slots(vault: &Path) -> Value {
    std::fs::read_to_string(vault.join("state/mail-resolve-held.json"))
        .map(|text| serde_json::from_str(&text).expect("the held-slot file is JSON"))
        .unwrap_or(Value::Null)
}

fn lab_claim_row(uid: &str, today: Date) -> Value {
    row(uid, T1, "live", vec![change("Lab", "CS 101", &ahead(today, 9))])
}

const GAVE_UP: &str = "mail: gave up asking which item for 1 claim after 3 slots; each is an unmatched notice";

/// A scripted `mail-resolve` that fails (503) and a pull that returns `pulled` every time.
fn resolve_down(pulled: &Value) -> Loop {
    let body = knowlu_engine::ledger::dumps_value(pulled);
    serve(move |request, _| {
        if request.starts_with("POST /functions/v1/mail-resolve ") {
            (503, r#"{"error":"down"}"#.to_string())
        } else {
            (200, body.clone())
        }
    })
}

/// Held at most 3 slots: the first two slots hold the message (nothing acked, nothing written, one more
/// slot counted); the third asks once more, gets no answer, acks the message and opens the unmatched
/// notice keyed by the claim's own uid. The run line names the give-up.
#[test]
fn an_unanswered_claim_is_held_twice_and_given_up_on_the_third_slot() {
    let vault = scratch("giveup");
    two_labs(&vault);
    let today = vault_today(&vault);
    for (slot, count) in [(1, 1), (2, 2)] {
        let mut server = pulls(vec![pull(vec![lab_claim_row("mail:aa11", today)], json!([]), false)]);
        let lines = run(&vault, &server, today);
        assert!(lines[0].ends_with("0 unmatched"), "slot {slot}: {lines:?}");
        assert!(!lines.iter().any(|l| l.contains("gave up")), "slot {slot}: {lines:?}");
        assert!(!knowlu_engine::ingest::load_seen(&vault).contains("mail:aa11"), "slot {slot}: held");
        assert!(open_items(&vault, "mail:aa11:0").is_empty(), "slot {slot}: no notice yet");
        assert_eq!(held_slots(&vault), json!({ "mail:aa11:0": count }), "slot {slot}");
        assert_eq!(server.calls("mail-resolve").len(), 1, "slot {slot}");
    }
    let mut server = pulls(vec![pull(vec![lab_claim_row("mail:aa11", today)], json!([]), false)]);
    let lines = run(&vault, &server, today);
    assert_eq!(lines[0], "mail: 0 new, 0 changed, 0 proposed, 0 notices, 0 past, 0 superseded, 1 unmatched", "{lines:?}");
    assert!(lines.iter().any(|l| l == GAVE_UP), "{lines:?}");
    assert!(!lines.iter().any(|l| l.contains("could not ask")), "nothing is left for the next slot: {lines:?}");
    let notices = open_items(&vault, "mail:aa11:0");
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert_eq!(notices[0].2, "agent:knowlu.mail");
    assert_eq!(field(&vault, &notices[0].0, "title").unwrap(), "Knowlu could not match this to an item");
    assert_eq!(field(&vault, "tasks/lab-5.md", "due").unwrap(), "2030-01-01", "never a guess");
    assert_eq!(field(&vault, "tasks/lab-6.md", "due").unwrap(), "2030-01-01", "never a guess");
    assert!(knowlu_engine::ingest::load_seen(&vault).contains("mail:aa11"), "given up on, so recorded");
    assert_eq!(server.calls("mail-resolve").len(), 1, "the third slot still asked once");
    assert_eq!(server.ack_of(1), ["mail:aa11"], "and acked");
    assert_eq!(held_slots(&vault), Value::Null, "acked, so nothing left to count");
    // The fourth slot: the server no longer returns it, and nothing is asked or opened again.
    let mut server = pulls(vec![]);
    let lines = run(&vault, &server, today);
    assert!(!lines.iter().any(|l| l.contains("gave up")), "{lines:?}");
    assert!(server.calls("mail-resolve").is_empty());
    assert_eq!(open_items(&vault, "mail:aa11:0").len(), 1);
    let _ = std::fs::remove_dir_all(&vault);
}

/// An answer on slot 2 resolves the claim and drops only its own count: a second claim that stays
/// unanswered keeps its two slots and is given up on at its own third.
#[test]
fn an_answer_on_slot_two_resolves_and_leaves_the_other_claims_count() {
    let vault = scratch("giveup-answer");
    two_labs(&vault);
    let today = vault_today(&vault);
    let both = || pull(vec![lab_claim_row("mail:aa11", today), lab_claim_row("mail:bb22", today)], json!([]), false);
    let lines = run(&vault, &pulls(vec![both()]), today);
    assert!(lines[0].ends_with("0 unmatched"), "{lines:?}");
    assert_eq!(held_slots(&vault), json!({ "mail:aa11:0": 1, "mail:bb22:0": 1 }));
    let lab_6 = field(&vault, "tasks/lab-6.md", "id").unwrap();
    let server = resolving(vec![both()], move |body| {
        let answers: Vec<Value> = body["claims"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["uid"] == "mail:aa11")
            .map(|c| json!({ "uid": c["uid"], "index": c["index"], "target_id": lab_6 }))
            .collect();
        json!({ "answers": answers })
    });
    let lines = run(&vault, &server, today);
    assert_eq!(lines[0], "mail: 0 new, 0 changed, 1 proposed, 0 notices, 0 past, 0 superseded, 0 unmatched", "{lines:?}");
    assert_eq!(files(&vault, "approvals").len(), 1, "the answer landed as the one card for Lab 6");
    assert!(knowlu_engine::ingest::load_seen(&vault).contains("mail:aa11"));
    assert_eq!(held_slots(&vault), json!({ "mail:bb22:0": 2 }), "only the answered claim's count is dropped");
    let lines = run(&vault, &pulls(vec![pull(vec![lab_claim_row("mail:bb22", today)], json!([]), false)]), today);
    assert!(lines.iter().any(|l| l == GAVE_UP), "{lines:?}");
    assert_eq!(open_items(&vault, "mail:bb22:0").len(), 1);
    assert!(open_items(&vault, "mail:aa11:0").is_empty(), "the answered claim opens no notice");
    let _ = std::fs::remove_dir_all(&vault);
}

/// The count is generated state on disk, so a restart (a new process, nothing in memory) carries it: a
/// file already at two slots gives up on the next failed one; a call that fails (503) counts as no answer.
#[test]
fn the_count_survives_a_restart_and_a_failed_call_counts() {
    let vault = scratch("giveup-restart");
    two_labs(&vault);
    let today = vault_today(&vault);
    std::fs::write(vault.join("state/mail-resolve-held.json"), "{\"mail:aa11:0\": 2}\n").unwrap();
    let mut server = resolve_down(&pull(vec![lab_claim_row("mail:aa11", today)], json!([]), false));
    let lines = run(&vault, &server, today);
    assert_eq!(server.calls("mail-resolve").len(), 1);
    assert!(lines.iter().any(|l| l == GAVE_UP), "{lines:?}");
    assert!(!lines.iter().any(|l| l.contains("could not ask")), "{lines:?}");
    assert_eq!(open_items(&vault, "mail:aa11:0").len(), 1);
    assert!(knowlu_engine::ingest::load_seen(&vault).contains("mail:aa11"));
    let _ = std::fs::remove_dir_all(&vault);
}

/// A claim the asking never reached (a failed call ended it earlier in the pass) was not tried, so it
/// is not counted.
#[test]
fn a_claim_never_asked_this_pass_is_not_counted() {
    let vault = scratch("giveup-untried");
    labs_and_quizzes(&vault);
    let today = vault_today(&vault);
    let server = resolve_down(&pull(vec![ambiguous_message(today)], json!([]), false));
    let lines = run(&vault, &server, today);
    assert!(lines.iter().any(|l| l.contains("could not ask which item")), "the claims still wait: {lines:?}");
    assert_eq!(held_slots(&vault), json!({ "mail:aa11:0": 1, "mail:aa11:1": 1 }), "CS 101's claims were tried; MATH 220's were not");
    let _ = std::fs::remove_dir_all(&vault);
}

/// The drain is unblocked: a page of one held message ends the drain (above), but once the claim is
/// given up the message is acked and the next page is read in the same pass.
#[test]
fn the_drain_continues_past_a_claim_given_up_on() {
    let vault = scratch("giveup-drain");
    two_labs(&vault);
    let today = vault_today(&vault);
    std::fs::write(vault.join("state/mail-resolve-held.json"), "{\"mail:aa11:0\": 2}\n").unwrap();
    let held = row("mail:aa11", T1, "backfill", vec![change("Lab", "CS 101", &ahead(today, 9))]);
    let next = row("mail:cc33", T2, "backfill", vec![course_info("CS 101")]);
    let mut server = pulls(vec![pull(vec![held], json!([]), true), pull(vec![next], json!([]), false)]);
    let lines = run(&vault, &server, today);
    assert_eq!(lines[0], "mail: 0 new, 0 changed, 0 proposed, 1 notices, 0 past, 0 superseded, 1 unmatched", "{lines:?}");
    assert!(!lines.iter().any(|l| l.contains("more is waiting")), "{lines:?}");
    let seen = knowlu_engine::ingest::load_seen(&vault);
    assert!(seen.contains("mail:aa11") && seen.contains("mail:cc33"), "{seen:?}");
    assert_eq!(server.ack_of(1), ["mail:aa11"]);
    let _ = std::fs::remove_dir_all(&vault);
}

/// The give-up is durable until the message is acked (PQ14 fix round): when the ack of the give-up slot
/// fails the marker stays at the cap, and a message that comes back with no record of having been
/// recorded (the crash window between the notice and the seen ledger) is given up on again without being
/// asked, with no second notice. Once its ack goes through the marker is dropped.
#[test]
fn a_give_up_stays_marked_until_the_message_is_acked() {
    let vault = scratch("giveup-durable");
    two_labs(&vault);
    let today = vault_today(&vault);
    std::fs::write(vault.join("state/mail-resolve-held.json"), "{\"mail:aa11:0\": 2}\n").unwrap();
    let body = knowlu_engine::ledger::dumps_value(&pull(vec![lab_claim_row("mail:aa11", today)], json!([]), false));
    let pulled = AtomicUsize::new(0);
    let mut server = serve(move |request, _| {
        if request.starts_with("POST /functions/v1/mail-resolve ") {
            (200, r#"{"answers":[]}"#.to_string())
        } else if pulled.fetch_add(1, Ordering::SeqCst) == 0 {
            (200, body.clone())
        } else {
            (503, r#"{"error":"down"}"#.to_string())
        }
    });
    let lines = run(&vault, &server, today);
    assert!(lines.iter().any(|l| l == GAVE_UP), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("written but not acknowledged")), "{lines:?}");
    assert_eq!(server.calls("mail-resolve").len(), 1);
    assert_eq!(held_slots(&vault), json!({ "mail:aa11:0": 3 }), "given up on, but not yet acked: the marker stays");
    assert_eq!(open_items(&vault, "mail:aa11:0").len(), 1);
    // The record that it was seen is lost; the message comes back.
    let _ = std::fs::remove_file(vault.join("state/ingest-seen.md"));
    let mut server = pulls(vec![pull(vec![lab_claim_row("mail:aa11", today)], json!([]), false)]);
    let lines = run(&vault, &server, today);
    assert!(lines.iter().any(|l| l == GAVE_UP), "{lines:?}");
    assert!(server.calls("mail-resolve").is_empty(), "a claim already given up on is not asked again");
    assert_eq!(open_items(&vault, "mail:aa11:0").len(), 1, "one notice, never two");
    assert_eq!(server.ack_of(1), ["mail:aa11"], "and acked this time");
    assert_eq!(held_slots(&vault), Value::Null, "acked, so the marker goes");
    let _ = std::fs::remove_dir_all(&vault);
}
