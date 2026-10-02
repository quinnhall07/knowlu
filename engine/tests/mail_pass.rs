//! The mail pass over loopback: `pull_mail` (email spec 2026-09-30 §4.8 steps 1, 4 to 7, §4.10; plan T6.3).
//!
//! Every test binds `127.0.0.1:0`, serves its own replies from a second thread and joins it; every vault
//! is a temporary directory. Nothing leaves the machine and no test reads a real vault (CLAUDE.md).
//! `today` is the vault clock's (`cli::vault_zone`), so a vault's day 1 is today and its day 2 is
//! tomorrow whatever date the test runs on, and every due is written relative to it.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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

    /// The `ack` array of the n-th request's body.
    fn ack_of(&mut self, n: usize) -> Vec<String> {
        let requests = self.requests();
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

/// The mail-pull replies in order, then an empty pull.
fn pulls(replies: Vec<Value>) -> Loop {
    let bodies: Vec<String> = replies.iter().map(knowlu_engine::ledger::dumps_value).collect();
    serve(move |_, n| (200, bodies.get(n).cloned().unwrap_or_else(|| pull(vec![], json!([]), false).to_string())))
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
