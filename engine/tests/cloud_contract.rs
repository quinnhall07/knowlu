//! What the engine actually sends the judgment service, and what it does with every answer.
//!
//! Every test here binds a `TcpListener` to `127.0.0.1:0`, serves its own request from a second
//! thread, and joins that thread before returning. That is not egress: no DNS, no route off the
//! machine, no listener on a routable interface (CLAUDE.md; plan 3a's loopback rule). No test
//! reads a real vault, and none touches Credential Manager — `CloudClient::new` takes the token,
//! so `resolve` is exercised only where it can fail without one.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use knowlu_engine::cloudmodel::{CloudClient, CloudConfig, CloudModel};
use knowlu_engine::judge::{self, Heuristics, Item, Model, Verdict};

/// A loopback server that answers `replies` in order and hands back everything it was sent.
struct Loopback {
    base: String,
    handle: Option<std::thread::JoinHandle<Vec<String>>>,
}

impl Loopback {
    fn requests(&mut self) -> Vec<String> {
        self.handle.take().expect("joined once").join().expect("the listener thread did not panic")
    }
}

fn read_request(stream: &std::net::TcpStream) -> String {
    let mut reader = BufReader::new(stream.try_clone().expect("clone the accepted stream"));
    let mut head = String::new();
    let mut length = 0usize;
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

fn loopback(replies: Vec<(u16, String)>) -> Loopback {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the loopback listener");
    let port = listener.local_addr().expect("the listener has an address").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (code, body) in replies {
            let Ok((mut stream, _)) = listener.accept() else { break };
            seen.push(read_request(&stream));
            let response = format!(
                "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
        seen
    });
    Loopback { base: format!("http://127.0.0.1:{port}/functions/v1"), handle: Some(handle) }
}

fn config(base: &str) -> CloudConfig {
    CloudConfig {
        api_base: base.to_string(),
        anon_key: "anon-not-a-secret".to_string(),
        session_credential_target: "knowlu/test-profile/session".to_string(),
        account_id: "acct-1".to_string(),
    }
}

fn item() -> Item {
    Item {
        id: "task-abc123".to_string(),
        rel_path: "tasks/cs-100-hw-01.md".to_string(),
        title: "CS 100 HW 01".to_string(),
        body: "25 zyBooks sections. TRIPWIRE-9f2c.".to_string(),
        source_uid: "zybooks:1839992".to_string(),
        created_by: "zybooks".to_string(),
        // F11: without a course, `task_request`'s weight lookup (which keys on `seed.course`) finds
        // nothing and the "Homework 20%" assertion below cannot pass — the heuristics' course_map
        // fragment `CS-100` does not match the title `CS 100 HW 01`.
        course: Some("cs-100".to_string()),
        due: Some("2026-09-18T23:59".to_string()),
        effort_hours: 1.0,
        effort_source: "inferred".to_string(),
    }
}

fn heuristics() -> Heuristics {
    let mut weights = std::collections::BTreeMap::new();
    weights.insert("cs-100".to_string(), "Homework 20%".to_string());
    Heuristics {
        course_map: vec![("CS-100".to_string(), "cs-100".to_string())],
        slice_hours: 1.5,
        weights,
        preferences: "I work best in the morning.".to_string(),
    }
}

const ANSWERED: &str = r#"{"verdict":{"course":"cs-100","effort_hours":900,"importance":9,"importance_reason":"twenty percent of the grade","confidence":0.82},"tier":3,"outcome":"answered","model":"claude-haiku-4-5","prompt_version":"task-1","grammar_version":"task-1"}"#;

#[test]
fn the_task_request_carries_the_note_and_its_grounding_and_nothing_else() {
    let mut server = loopback(vec![(200, ANSWERED.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let _ = model.judge(&item(), &heuristics(), &seed);
    let sent = server.requests().remove(0);

    // Plan 3a fidelity row R6, moved to the boundary that now exists: the request carries the
    // note, the course's weights and the preferences, and nothing else at all.
    for forbidden in ["ingest.yaml", "credential", "credential_target", "C:\\", "knowlu/test-profile", "CS-100"] {
        assert!(!sent.contains(forbidden), "the request body carried {forbidden:?}:\n{sent}");
    }
    assert!(sent.contains("POST /functions/v1/judge-task HTTP/1.1"));
    // `http::HeaderName` normalises every header name to lowercase on the wire regardless of the
    // case passed to `.header(...)`, so the request line reads `authorization:`, not
    // `Authorization:` — this checks the actual bytes sent, not the call-site spelling.
    assert!(sent.contains("authorization: Bearer jwt-not-a-secret"));
    assert!(sent.contains("\"title\": \"CS 100 HW 01\""));
    assert!(sent.contains("Homework 20%"));
    assert!(sent.contains("I work best in the morning."));
    // The course_map's SLUGS travel (they are already in every note's `course:` field); its KEYS
    // — the fragments out of `config/ingest.yaml` — never do.
    assert!(sent.contains("\"cs-100\""));
}

#[test]
fn the_reply_goes_through_parse_reply_so_the_clamps_still_apply() {
    let mut server = loopback(vec![(200, ANSWERED.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let got = model.judge(&item(), &heuristics(), &seed).expect("the service answered");
    assert_eq!(got.effort_hours, Some(40.0), "900 hours is a bad answer, not a broken one");
    assert_eq!(got.importance, Some(5));
    assert_eq!(got.tier, 3);
    let _ = server.requests();
}

#[test]
fn a_401_says_it_is_the_session_and_not_a_bad_answer() {
    // Ruling R-3a-25 at the two boundaries a cloud judge adds. A 401 that reads as "low
    // confidence" twice a day forever is exactly the failure the structural-cause split exists
    // to prevent — the words have to say "sign in again".
    let mut server = loopback(vec![(401, r#"{"error":"invalid jwt"}"#.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let err = model.judge(&item(), &heuristics(), &seed).expect_err("a 401 is an error");
    assert!(err.to_string().contains("no session"), "{err}");
    assert_eq!(model.fatal(), Some("no session"));
    let _ = server.requests();
}

#[test]
fn a_402_stops_the_batch_after_one_call() {
    // One reply is scripted, and the second `judge` must not reach the wire at all: an account
    // without an entitlement answers every item identically, and fifty round trips to learn that
    // is fifty wasted calls and fifty log lines that all say the same thing.
    let mut server = loopback(vec![(402, r#"{"error":"no active subscription"}"#.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let _ = model.judge(&item(), &heuristics(), &seed);
    let second = model.judge(&item(), &heuristics(), &seed).expect_err("still an error");
    assert!(second.to_string().contains("no entitlement"), "{second}");
    assert_eq!(server.requests().len(), 1, "the second item must not have been sent");
}

#[test]
fn a_5xx_survives_to_the_error_the_engine_prints() {
    let mut server = loopback(vec![(503, r#"{"error":"upstream overloaded"}"#.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let err = model.judge(&item(), &heuristics(), &seed).expect_err("a 503 is an error");
    assert!(err.to_string().contains("503"), "{err}");
    assert!(err.to_string().contains("upstream overloaded"), "{err}");
    assert_eq!(model.fatal(), None, "a 503 is worth retrying the next item");
    let _ = server.requests();
}

#[test]
fn the_session_token_never_reaches_an_error_string() {
    // A transport error's text is a third party's, and `state/runner-log.md` is written from it.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener); // nothing is listening on that port now
    let client = CloudClient::new(&config(&format!("http://127.0.0.1:{port}/functions/v1")), "jwt-super-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let err = model.judge(&item(), &heuristics(), &seed).expect_err("nothing is listening");
    assert!(!err.to_string().contains("jwt-super-secret"), "{err}");
}

#[test]
fn a_vault_with_no_cloud_config_is_no_account_and_not_an_error() {
    let dir = std::env::temp_dir().join(format!("knowlu-c2-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("config")).expect("scratch vault");
    assert_eq!(knowlu_engine::cloudmodel::load(&dir), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_verdict_the_service_refused_is_an_error_that_names_the_cause() {
    let refused = r#"{"verdict":null,"tier":3,"outcome":"low confidence","cause":"below floor"}"#;
    let mut server = loopback(vec![(200, refused.to_string())]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let model = CloudModel::new(&client);
    let seed = judge::tier1(&item(), &heuristics());
    let err = model.judge(&item(), &heuristics(), &seed).expect_err("no verdict is an error");
    assert!(err.to_string().contains("below floor"), "{err}");
    let _ = server.requests();
}

#[test]
fn tier1_still_answers_without_the_service_being_reached_at_all() {
    // The seam is unchanged (cloud design §3.2): a vendor-stated effort plus a pinned course is a
    // complete tier-1 answer, and `judge_task` returns before any model, cloud or otherwise.
    let mut vendor = item();
    vendor.effort_source = "vendor".to_string();
    vendor.effort_hours = 1.52;
    vendor.course = Some("cs-100".to_string());
    let seed: Verdict = judge::tier1(&vendor, &heuristics());
    assert_eq!(seed.course.as_deref(), Some("cs-100"));
    assert_eq!(seed.effort_hours, Some(1.52));
    assert_eq!(seed.tier, 1);
}
