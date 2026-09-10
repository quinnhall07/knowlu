//! `report.rs` against a fixed set of scrub cases, a real vault shape, real log files, and a
//! loopback `POST /issues` — never the network (the 3a rule: `127.0.0.1` only).
use knowlu::report::{log_tail, scrub, vault_shape};
use std::io::{Read, Write};

#[test]
fn the_seven_things_a_report_must_never_carry() {
    // The same six the cloud's `_shared/scrub.ts` refuses, in the same order, producing the same
    // text — the two ends are a pair and this test and its TypeScript twin are how they stay one.
    // (R-C1-45: the cloud wins where the brief's literal expectation differs — see the two cases
    // called out below, both changed from `<token>` to `<secret>` to match `_shared/scrub.ts`'s
    // `CREDENTIAL` rule, which claims the whole `keyword[:=]value` (and an optional `Bearer `)
    // as one redaction before the bare-token rule ever runs.)
    assert_eq!(scrub("mailed a.student@crimson.ua.edu twice"), "mailed <email> twice");
    assert_eq!(scrub("fetching https://lms.example.invalid/feed/abc123.ics failed"), "fetching <url> failed");
    assert_eq!(scrub("http://10.0.0.1/x"), "<url>");
    // Changed from the brief's `"authorization: Bearer <token>"`: the cloud's CREDENTIAL rule
    // matches `authorization` itself (it is one of the keywords), so it claims the whole
    // `authorization: Bearer <jwt>` span before TOKEN ever sees it, and its replacement is
    // `$1=<secret>` — not the bare-token replacement.
    assert_eq!(
        scrub("authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef"),
        "authorization=<secret>"
    );
    assert_eq!(scrub("1 unreadable: read-chapter-3-of-calculus.md)"), "1 unreadable: <note>)");
    // Changed from the brief's `"key=<token>"`: the cloud's CREDENTIAL rule claims `key=...` before
    // the bare-token rule runs, and its replacement is `key=<secret>`, not `key=<token>`.
    assert_eq!(scrub("key=sk_live_51Hxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), "key=<secret>");
    // The Windows account name is usually a person's name, and the log lines are full of paths.
    assert_eq!(
        scrub("could not open C:\\Users\\Ada\\Knowlu\\Fall 2026\\tasks\\a.md"),
        "could not open C:\\Users\\<user>\\Knowlu\\Fall 2026\\tasks\\<note>"
    );
    // …and ordinary prose survives, or the preview is unreadable and nobody sends anything.
    assert_eq!(scrub("the run at 12:00 exited 1 after 4 steps"), "the run at 12:00 exited 1 after 4 steps");
}

#[test]
fn the_r_c1_26_shapes_are_claimed_whole() {
    // A capability URL of any scheme, not only the two web ones.
    assert_eq!(scrub("subscribe webcal://cal.example.edu/feed.ics now"), "subscribe <url> now");
    // A two-word Windows account name, consumed whole, with a forward slash.
    assert_eq!(
        scrub("C:/Users/Ada Lovelace/Knowlu/tasks/a.md"),
        "C:/Users/<user>/Knowlu/tasks/<note>"
    );
    // A named credential is a secret whatever its length — the token rule's 20-character floor is
    // not what decides this.
    assert_eq!(scrub("key=sk_live_51Hxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), "key=<secret>");
    // `Authorization: Bearer <short>` is claimed whole by CREDENTIAL, even though the token itself
    // is far short of TOKEN's 20-character floor — TOKEN never gets a look at it.
    assert_eq!(scrub("Authorization: Bearer abc"), "Authorization=<secret>");
}

#[test]
fn the_two_ends_of_the_scrub_are_written_against_the_same_cases() {
    // There is no way to run TypeScript from here, so the pin is that the cloud's twin exists, names
    // the same six classes, and orders them the same way: email, then url, then the windows user,
    // then note, then credential, then the bare token (R-C1-45).
    let ts = std::fs::read_to_string("../cloud/supabase/functions/_shared/scrub.ts").expect("scrub.ts");
    for marker in ["<email>", "<url>", "<user>", "<note>", "<secret>", "<token>"] {
        assert!(ts.contains(marker), "the cloud scrub has no {marker}");
    }
    // The order that matters is the `.replace()` chain itself, not any doc comment that happens to
    // mention a marker in passing (line 25's comment names `<token>` while explaining the credential
    // rule, well before that rule's own `.replace()` call) — so the pin anchors on the chain's own
    // start and reads the six calls' order from there.
    let chain = &ts[ts.find(".replace(EMAIL").expect("the cloud scrub's own replace chain")..];
    let order: Vec<usize> =
        ["<email>", "<url>", "<user>", "<note>", "<secret>", "<token>"].iter().map(|m| chain.find(m).unwrap()).collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "the cloud scrub applies its rules in another order");
}

#[test]
fn the_vault_shape_is_counts_and_never_names() {
    let v = std::env::temp_dir().join(format!("knowlu-report-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&v);
    for d in ["tasks", "approvals", "courses", "info", "issues", "archive"] {
        std::fs::create_dir_all(v.join(d)).unwrap();
    }
    std::fs::write(v.join("tasks").join("read-chapter-3.md"), "---\ntitle: Read chapter 3\n---\n").unwrap();
    std::fs::write(v.join("tasks").join("write-the-essay.md"), "---\ntitle: Write the essay\n---\n").unwrap();
    std::fs::write(v.join("approvals").join("a.md"), "x").unwrap();
    let shape = vault_shape(&v);
    assert_eq!(shape.iter().find(|(k, _)| k == "tasks").unwrap().1, 2);
    assert_eq!(shape.iter().find(|(k, _)| k == "approvals").unwrap().1, 1);
    assert_eq!(shape.iter().find(|(k, _)| k == "courses").unwrap().1, 0);
    // Nothing in the shape is a filename, and a filename is a title.
    let rendered = format!("{shape:?}");
    assert!(!rendered.contains("read-chapter-3") && !rendered.contains("essay"), "{rendered}");
    let _ = std::fs::remove_dir_all(&v);
}

#[test]
fn the_log_tail_is_the_last_lines_of_the_newest_logs_and_is_scrubbed() {
    let d = std::env::temp_dir().join(format!("knowlu-report-logs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("logs")).unwrap();
    std::fs::write(d.join("logs").join("slot-1-0-coursework.txt"), "old line\n").unwrap();
    let many: String = (0..300).map(|i| format!("line {i} https://lms.example.invalid/x{i}.ics\n")).collect();
    std::fs::write(d.join("logs").join("slot-2-0-rank.txt"), many).unwrap();
    let tail = log_tail(&d, 200);
    assert_eq!(tail.len(), 200);
    assert!(tail.iter().all(|l| !l.contains("https://")), "a capability URL survived into the tail");
    assert!(tail.last().unwrap().contains("line 299"));
    let _ = std::fs::remove_dir_all(&d);
}

/// Serves exactly one request on `127.0.0.1:0`, then stops — the small helper from
/// `app/tests/account.rs`, copied rather than shared (that file's helper is private to its crate
/// and this is the one seam both need).
fn loopback_once(status: u16, body: &str) -> (String, std::thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.set_nonblocking(true).expect("nonblocking listener");
    let port = listener.local_addr().expect("addr").port();
    let status = status;
    let body = body.to_string();
    let handle = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        panic!("loopback: no client connected within 10s");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("loopback: accept failed: {e}"),
            }
        };
        stream.set_nonblocking(false).expect("blocking stream");
        stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).expect("read timeout");
        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 1024];
        let head_end = loop {
            let n = stream.read(&mut chunk).unwrap_or(0);
            if n == 0 {
                break buf.len();
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
            let n = stream.read(&mut chunk).unwrap_or(0);
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        let seen = String::from_utf8_lossy(&buf).to_string();
        let reason = if (200..300).contains(&status) { "OK" } else { "Bad Request" };
        let resp = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
        seen
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

#[test]
fn send_at_posts_compact_json_with_the_bearer_and_reads_the_id_back() {
    let (base, handle) = loopback_once(201, r#"{"id":"iss_1"}"#);
    let id = knowlu::report::send_at(&base, "tok-abc", "hello world", "profile-1").expect("send_at");
    assert_eq!(id, "iss_1");
    let req = handle.join().expect("server thread");
    assert!(req.starts_with("POST /issues"), "{req}");
    assert!(req.to_ascii_lowercase().contains("content-type: application/json"), "{req}");
    assert!(req.contains("authorization: Bearer tok-abc"), "{req}");
    // Compact JSON, not pretty-printed: no embedded newline inside the body.
    let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &req[body_start..];
    assert!(!body.contains('\n'), "the body was not sent compact: {body}");
    assert!(body.contains(r#""body":"hello world""#), "{body}");
    assert!(body.contains(r#""profile_id":"profile-1""#), "{body}");
}

#[test]
fn send_at_reports_a_non_2xx_status_as_a_named_refusal() {
    let (base, handle) = loopback_once(400, r#"{"error":"bad"}"#);
    let err = knowlu::report::send_at(&base, "tok-abc", "hello", "profile-1").unwrap_err();
    assert!(err.contains("400"), "{err}");
    let _ = handle.join();
}

#[test]
fn send_at_refuses_a_non_https_non_loopback_api_base() {
    let err = knowlu::report::send_at("http://example.com", "tok-abc", "hello", "profile-1").unwrap_err();
    assert!(err.contains("https://"), "{err}");
}
