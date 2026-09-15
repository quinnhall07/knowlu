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

/// `sorted(folder.glob("*.md"))` — `knowlu_engine::approvals::sorted_md` does exactly this but is
/// `pub(crate)` and unreachable from an integration test (F9), so this is a local copy rather than
/// a widened visibility.
fn sorted_md(folder: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<std::path::PathBuf> = match std::fs::read_dir(folder) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "md") == Some(true))
            .collect(),
        Err(_) => return Vec::new(),
    };
    paths.sort();
    paths
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

#[test]
fn the_ics_fetch_is_a_get_that_sends_no_url_of_its_own() {
    let feed = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
    let body = knowlu_engine::ledger::dumps_value(&serde_json::json!({ "ics": feed, "courses": 0 }));
    let mut server = loopback(vec![(200, body)]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    // R-C2-E8: `fetch_ics` takes `first_run` and returns `(ics, past_due_uids)`; this reply carries
    // no `past_due_uids` field, so the second half of the tuple is an empty `Vec` (Task 8a's field).
    let (got, past_due) = knowlu_engine::cloudmodel::fetch_ics(&client, false).expect("the service answered");
    assert!(got.contains("BEGIN:VCALENDAR"));
    assert!(past_due.is_empty());
    let sent = server.requests().remove(0);
    assert!(sent.starts_with("GET /functions/v1/ingest-ics HTTP/1.1"));
    // The device does not know the URL any more and must not be able to name one.
    assert!(!sent.contains("ics_url"), "{sent}");
}

/// R-C2-E20 fix 1: the wire shape R-C2-E8 froze, pinned directly — `first_run` reaches the query
/// string, and a reply's `past_due_uids` (Task 8a's field) comes back through the tuple whole.
#[test]
fn a_first_run_asks_first_run_1_and_gets_back_both_past_due_uids() {
    let feed = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
    let body = knowlu_engine::ledger::dumps_value(
        &serde_json::json!({ "ics": feed, "courses": 0, "past_due_uids": ["a", "b"] }),
    );
    let mut server = loopback(vec![(200, body)]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let (got, past_due) = knowlu_engine::cloudmodel::fetch_ics(&client, true).expect("the service answered");
    assert!(got.contains("BEGIN:VCALENDAR"));
    assert_eq!(past_due, vec!["a".to_string(), "b".to_string()]);
    let sent = server.requests().remove(0);
    assert!(sent.starts_with("GET /functions/v1/ingest-ics?first_run=1 HTTP/1.1"), "{sent}");
}

/// R-OB-3: the first page a student ever sees must show the future.
///
/// A feed's window reaches backwards, and a vault born today has no history to reconcile against —
/// so Quinn's first slot imported four overdue items, one from 2025. On a FIRST ingest (no
/// `today.md`, R-C2-9) a past-due item is recorded as seen and written straight into
/// `archive/`; on every later run it is created normally, because an item that goes past due while
/// the vault is watching it is exactly the item the system exists to shout about.
#[test]
fn ingest_on_a_fresh_vault_archives_past_due_items() {
    let dir = std::env::temp_dir().join(format!("knowlu-c2-firstrun-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["config", "tasks", "state", "archive"] {
        std::fs::create_dir_all(dir.join(sub)).expect("scratch vault");
    }
    std::fs::write(
        dir.join("config").join("ingest.yaml"),
        "ics_url: \"https://example.invalid/learn.ics\"\ntimezone: America/Chicago\ncourse_map:\n  CS-100: cs-100\n",
    )
    .expect("write ingest.yaml");
    // One item long past, one comfortably ahead. `parse_ics` reads `DTSTART` as the due date.
    let feed = "BEGIN:VCALENDAR\r\n\
        BEGIN:VEVENT\r\nUID:bb-old\r\nSUMMARY:CS-100 Homework 1\r\nDTSTART:20250902T045900Z\r\nEND:VEVENT\r\n\
        BEGIN:VEVENT\r\nUID:bb-new\r\nSUMMARY:CS-100 Homework 9\r\nDTSTART:20991002T045900Z\r\nEND:VEVENT\r\n\
        END:VCALENDAR\r\n";
    let fetch = |_: &str| Ok(feed.to_string());

    // First run is the absence of `today.md`, NOT of the seen-ledger (R-C2-9) — and this test
    // proves the distinction by writing the ledger first, exactly as `coursework` does at
    // `coursework.rs:310` earlier in the same slot.
    std::fs::write(
        dir.join("state").join("ingest-seen.md"),
        "# header\n- zybooks:UACS100Fall2026:1.2 · Section 1.2 · first seen 2026-09-09\n",
    )
    .expect("coursework got here first");
    assert!(!dir.join("state").join("today.md").exists());
    let (code, lines) = knowlu_engine::ingest::run_lines(&dir, "cli", None, Some(&fetch));
    assert_eq!(code, 0, "{lines:?}");
    assert!(
        lines.iter().any(|l| l.contains("archived (imported-past)") && l.contains("bb-old") == false),
        "the past-due item is archived and named by its stem: {lines:?}"
    );

    // The future item is a task; the past one is in `archive/` and in nothing else.
    let tasks: Vec<_> = sorted_md(&dir.join("tasks"));
    assert_eq!(tasks.len(), 1, "only the future item is a task");
    assert!(std::fs::read_to_string(&tasks[0]).unwrap().contains("Homework 9"));
    let archived: Vec<_> = sorted_md(&dir.join("archive"));
    assert_eq!(archived.len(), 1);
    let note = std::fs::read_to_string(&archived[0]).unwrap();
    assert!(note.contains("Homework 1"));
    assert!(note.contains("status: archived"));
    assert!(note.contains("archived_reason: imported-past"));
    // Recorded as seen, so a second run does not resurrect it.
    let seen = std::fs::read_to_string(dir.join("state").join("ingest-seen.md")).unwrap();
    assert!(seen.contains("bb-old") && seen.contains("bb-new"));

    // `rank` has since finished the slot and written `today.md`, which is what makes the next
    // ingest not-first. A LATER run is unchanged: a new past-due item is created as a task, loudly.
    std::fs::write(dir.join("state").join("today.md"), "# Today\n").expect("rank wrote today.md");
    let later = "BEGIN:VCALENDAR\r\n\
        BEGIN:VEVENT\r\nUID:bb-late\r\nSUMMARY:CS-100 Homework 2\r\nDTSTART:20250903T045900Z\r\nEND:VEVENT\r\n\
        END:VCALENDAR\r\n";
    let fetch_later = |_: &str| Ok(later.to_string());
    let (code, lines) = knowlu_engine::ingest::run_lines(&dir, "cli", None, Some(&fetch_later));
    assert_eq!(code, 0, "{lines:?}");
    assert_eq!(sorted_md(&dir.join("tasks")).len(), 2, "{lines:?}");
    assert_eq!(sorted_md(&dir.join("archive")).len(), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

/// R-C2-9: the predicate is the absence of `today.md`, not of `state/ingest-seen.md`.
///
/// The slot order is `coursework → ingest → judge → rank`, and `coursework` calls
/// `ingest::record_seen` (`coursework.rs:19`, `:310`, `:415`) — so on the very first slot the
/// seen-ledger already exists by the time `ingest` runs, and a `first_run` derived from it is
/// `false` on exactly the run R-OB-3 was written for. `today.md` is `rank`'s, the last step, so the
/// first `ingest` always sees it absent and every later one sees it present. It is also the
/// predicate the app already uses (`app/src/scheduler.rs::needs_first_run`).
#[test]
fn first_run_is_the_absence_of_today_md_not_of_ingest_seen() {
    let dir = std::env::temp_dir().join(format!("knowlu-c2-firstpred-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("state")).expect("scratch vault");

    // What `coursework` leaves behind on the first slot, before `ingest` has run at all.
    std::fs::write(dir.join("state").join("ingest-seen.md"), "# header\n").unwrap();
    assert!(
        knowlu_engine::ingest::is_first_run(&dir),
        "the seen-ledger is shared with coursework and cannot be the predicate"
    );

    // What `rank` leaves behind at the end of that same slot.
    std::fs::write(dir.join("state").join("today.md"), "# Today\n").unwrap();
    assert!(!knowlu_engine::ingest::is_first_run(&dir));

    let _ = std::fs::remove_dir_all(&dir);
}

/// A cloud vault whose `ics_url` is blank must still refuse by naming `ics_url`, on the one arm
/// this test can actually reach.
///
/// **What this proves, and what it does not.** On this machine there is no credential at
/// `knowlu/c2-test/session`, so `cloudmodel::resolve` fails before any HTTP call is made and the
/// run falls all the way through to the `(None, None)` fallback arm — the same one this vault hit
/// before H3, byte for byte. That arm's message and the cloud arm's own blank-url message
/// (`"ingest: no feed — the service is unavailable (…) and no ics_url is configured"`) both
/// mention `ics_url`, which is what this test asserts, so it stays green on either arm rather than
/// pinning the fallback's exact wording. **The cloud arm's end-to-end proof — that a vault with a
/// real session and an `lms_ics` row reaches `/ingest-ics` and never hits this refusal at all — is
/// the staging smoke against a real account (Task 8 step 7/13), not a unit test**: per ruling F10,
/// this test spawns no loopback listener, because with no session `resolve` never attempts a
/// connection and a listener here would block forever in `accept()`.
#[test]
fn a_cloud_vault_reaches_the_service_before_it_refuses_a_blank_ics_url() {
    let dir = std::env::temp_dir().join(format!("knowlu-c2-ics-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("config")).expect("scratch vault");
    std::fs::create_dir_all(dir.join("tasks")).expect("scratch vault");
    std::fs::write(dir.join("config").join("ingest.yaml"), "timezone: America/Chicago\nics_url: \"\"\n")
        .expect("write ingest.yaml");
    std::fs::write(
        dir.join("config").join("cloud.yaml"),
        "api_base: 'https://cloud.example.invalid/functions/v1'\nanon_key: 'anon-not-a-secret'\nsession_credential_target: 'knowlu/c2-test/session'\naccount_id: 'acct-1'\n",
    )
    .expect("write cloud.yaml");

    let (code, lines) = knowlu_engine::ingest::run_lines(&dir, "cli", None, None);
    assert_eq!(code, 1);
    assert!(lines[0].contains("ics_url"), "the refusal must still say what is missing: {lines:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_calendar_fetch_is_a_get_that_names_the_feed_and_no_address() {
    let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:g1\r\nSUMMARY:Seminar\r\n\
               DTSTART:20260909T140000Z\r\nDTEND:20260909T150000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let body = knowlu_engine::ledger::dumps_value(
        &serde_json::json!({ "ics": ics, "source": "google_calendar" }),
    );
    let mut server = loopback(vec![(200, body)]);
    let client = CloudClient::new(&config(&server.base), "jwt-not-a-secret");
    let got = knowlu_engine::cloudmodel::fetch_calendar(&client, "google").expect("the service answered");
    assert!(got.contains("BEGIN:VEVENT"));
    let sent = server.requests().remove(0);
    assert!(sent.starts_with("GET /functions/v1/ingest-calendar?name=google HTTP/1.1"), "{sent}");
    // The device does not know the secret address any more and must not be able to name one.
    assert!(!sent.contains("ics_url") && !sent.contains("calendar_ics"), "{sent}");
}
