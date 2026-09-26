//! `report.rs` against a fixed set of scrub cases, a real vault shape, real log files, and a
//! loopback `POST /issues` — never the network (the 3a rule: `127.0.0.1` only).
use knowlu::report::{log_tail, preview_text, scrub, vault_shape};
use knowlu::state::ConsoleState;
use std::io::{Read, Write};
use std::path::PathBuf;

#[test]
fn the_seven_things_a_report_must_never_carry() {
    // The name is the brief's, and the cloud's own `Deno.test` carries the same name over the same
    // eight assertions (M5, fix round 1: the name stays as the brief wrote it; a comment claiming
    // "the same six" alongside eight assertions was the thing worth fixing, not the name) — in the
    // same order, producing the same text: the two ends are a pair and this test and its
    // TypeScript twin are how they stay one. (R-C1-45: the cloud wins where the brief's literal
    // expectation differs — see the two cases called out below, both changed from `<token>` to
    // `<secret>` to match `_shared/scrub.ts`'s `CREDENTIAL` rule, which claims the whole
    // `keyword[:=]value` (and an optional `Bearer `) as one redaction before the bare-token rule
    // ever runs.)
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

/// Fix round 1 (C1, I1; R-C1-46): the hand-written scanners committed in round 1 disagreed with the
/// cloud on these exact eight inputs — every one pinned here with the cloud's own output (computed
/// by running its actual `scrub()` under Deno, per the ruling; the same eight are added to
/// `cloud/supabase/functions/_shared/scrub_test.ts` so the two ends are pinned against the same
/// cases, not just re-implemented against the same intent).
#[test]
fn fix_round_1_the_reviews_divergent_shapes_are_pinned() {
    // C1: a capability URL right after a punctuation character that is a scheme-continuation
    // character (`+`, `.`, `-`) but not a `\w` — a `\b` holds one character later than a
    // single-backtrack scanner ever looked, so the URL was left on the wire entirely unredacted.
    assert_eq!(scrub("-https://lms.example.invalid/feed"), "-<url>");
    assert_eq!(scrub(".https://lms.example.invalid/feed"), ".<url>");
    assert_eq!(scrub("+https://x/y"), "+<url>");
    assert_eq!(scrub("2026-https://x/y"), "2026-<url>");
    // I1.1: `\S+` needs at least one character — a scheme with nothing after `://` is not a URL.
    assert_eq!(scrub("x://"), "x://");
    // I1.1: this also changes which rule wins — CREDENTIAL claims `token://` before URL_RE's
    // (fixed) empty-`\S+` check would ever let URL_RE match nothing here.
    assert_eq!(scrub("token://"), "token=<secret>");
    // I1.2: `NOTE`'s `\b` sits at the first `\w`, not at the start of the `[\w.-]` run — a run that
    // opens on `-` or `.` keeps that leading punctuation outside the match.
    assert_eq!(scrub("-a.md"), "-<note>");
    // I1.3: `(?:bearer\s+)?` backtracks when the value after it would be empty — the trailing space
    // here leaves nothing for `\S+`, so JS drops the optional group and matches `\S+` = `Bearer`
    // instead, claiming the header but leaving the trailing space outside the match.
    assert_eq!(scrub("authorization: Bearer "), "authorization=<secret> ");
}

/// Fix round 2 (N1, ruling R-C1-49): round 1's ASCII-restricted `\s` (positive, in `CREDENTIAL`)
/// and Unicode-default `\S` (negated, in `URL_RE` and `CREDENTIAL`) each disagreed with the cloud's
/// actual JavaScript `\s`/`\S` in a different direction. A non-breaking space (U+00A0) next to a
/// credential keyword made the whole match fail to fire — the password or bearer token survived
/// **completely unredacted** — and NEL (U+0085) inside a URL's tail ended the match one character
/// early, leaving the rest of the value on the wire. All three cases pinned here with the cloud's
/// own computed output (run under Deno, not reasoned by hand); the same three are added to
/// `cloud/supabase/functions/_shared/scrub_test.ts`.
#[test]
fn fix_round_2_a_non_breaking_space_and_nel_are_javascripts_whitespace_not_rusts() {
    // A non-breaking space before the `:` separator — routine in text pasted from a web page or a
    // Word document — must not let the password through.
    assert_eq!(scrub("password\u{a0}: hunter2"), "password=<secret>");
    // A non-breaking space inside the `Bearer ` prefix — the whole header is still claimed whole.
    assert_eq!(
        scrub("authorization:\u{a0}Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abcdef"),
        "authorization=<secret>"
    );
    // NEL (U+0085) inside a URL's tail: JavaScript's `\s` does not include it, so the whole
    // capability URL — NEL and all — is claimed, not just the part before it.
    assert_eq!(scrub("see https://x.invalid/secret\u{85}tail more"), "see <url> more");
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

/// Fix round 1, review M4. `last_error` alone leaves a lapsed subscription or a signed-out machine
/// invisible in an issue report — a skip carries no `last_error`, so a support report from exactly
/// the student who most needs help (their copy has not reached the account in weeks) said nothing
/// about sync at all.
#[test]
fn preview_text_names_a_skipped_sync_as_well_as_a_failed_one() {
    let vault = PathBuf::from("../engine/tests/fixtures/vault-full");
    let data = std::env::temp_dir().join(format!("knowlu-report-preview-skip-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    let cs = ConsoleState::open(vault, data.clone());
    *cs.sync.lock().unwrap() = knowlu_engine::sync::SyncStatus {
        ok: false,
        at: Some(knowlu_engine::journal::now_ts(None)),
        lines: vec![],
        last_error: None,
        skipped: Some("no entitlement".to_string()),
    };
    let text = preview_text(&cs, "today");
    assert!(text.contains("sync skipped: no entitlement"), "a lapsed subscription must reach the issue report: {text}");
    let _ = std::fs::remove_dir_all(&data);
}

/// I2, fix round 1: the cloud refuses a `body` over 8,192 characters
/// (`cloud/supabase/functions/issues/handler.ts`), and 200 real log lines routinely add up to more
/// than that on a busy install. `preview_text` has to fit under that cap on its own, leaving room
/// for the sentence the student is about to type, or every report is refused before a human ever
/// reads it.
#[test]
fn preview_text_stays_under_the_clouds_body_cap_even_with_a_flood_of_logs() {
    let vault = PathBuf::from("../engine/tests/fixtures/vault-full");
    let data = std::env::temp_dir().join(format!("knowlu-report-preview-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(data.join("logs")).unwrap();
    // On its own, several times over the cloud's cap — a slot that ran into trouble and logged a
    // long, ordinary sentence on every line gets here easily. Prose, not one long opaque run: a
    // run of 20+ token characters would be scrubbed down to `<token>` before length is ever
    // measured, which would defeat the point of this fixture.
    let filler = "the quick brown fox jumps over the lazy dog and back again ".repeat(4);
    let many: String = (0..500).map(|i| format!("{filler}line {i}\n")).collect();
    std::fs::write(data.join("logs").join("slot-1-0-rank.txt"), many).unwrap();
    let cs = ConsoleState::open(vault, data.clone());
    let text = preview_text(&cs, "today");
    // The cloud's own MAX_BODY (`cloud/supabase/functions/issues/handler.ts:5`) — not imported
    // (nothing in `app/` reaches into `cloud/`), the same number `report.rs`'s own `MAX_BODY`
    // mirrors by hand.
    const CLOUD_MAX_BODY: usize = 8192;
    assert!(text.len() < CLOUD_MAX_BODY, "the preview exceeded the cloud's own cap: {} bytes", text.len());
    assert!(text.contains("earlier log lines dropped"), "no trim marker in:\n{text}");
    // The newest line is what a reader wants kept — the oldest lines are what get dropped.
    assert!(text.contains("line 499"), "the newest log line did not survive the trim");
    assert!(!text.contains("line 0 "), "an oldest log line survived when it should have been dropped");
    let _ = std::fs::remove_dir_all(&data);
}

/// N2, fix round 2: `runs_panel`'s `recent` is most-recent-first, so `run_lines[0]` (built by
/// `preview_text` from it) is the *newest* of the up-to-three rows taken. The run-row trim loop
/// must drop from the **end** of `run_lines` (the oldest), matching the log-line trim right above
/// it and this function's own doc comment — dropping from the front would keep stale rows and
/// throw away the one a reader most wants.
///
/// No log files at all (so the first trim loop has nothing to do and never fires) and three real
/// run records — written through `knowlu_engine::runs::{start_run, end_run}`, not hand-built — each
/// with a long, ordinary summary and its own marker, so the header-plus-three-rows total alone
/// clears the cloud's cap and the second trim loop has to run.
#[test]
fn preview_text_trims_the_oldest_run_row_first_not_the_newest() {
    let vault = std::env::temp_dir().join(format!("knowlu-report-runtrim-vault-{}", std::process::id()));
    let data = std::env::temp_dir().join(format!("knowlu-report-runtrim-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&vault);
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::create_dir_all(&data).unwrap(); // deliberately no `logs/` subdirectory

    let now = jiff::Timestamp::now();
    let filler = "the quick brown fox jumps over the lazy dog and back again ".repeat(60);
    let make_run = |minutes_ago: i64, marker: &str| {
        let ts = now - jiff::SignedDuration::from_secs(minutes_ago * 60);
        let id = knowlu_engine::runs::start_run(&vault, "local", Some(ts), Some("d"), None);
        let summary = format!("{filler}{marker}");
        knowlu_engine::runs::end_run(&vault, &id, "ok", &summary, 0, Some(ts), None, &[], Some("local"));
    };
    make_run(3, "MARKER-OLDEST-RUN");
    make_run(2, "MARKER-MIDDLE-RUN");
    make_run(1, "MARKER-NEWEST-RUN");

    let cs = ConsoleState::open(vault.clone(), data.clone());
    let text = preview_text(&cs, "today");
    assert!(text.contains("MARKER-NEWEST-RUN"), "the newest run row should survive trimming:\n{text}");
    assert!(!text.contains("MARKER-OLDEST-RUN"), "the oldest run row should be trimmed first:\n{text}");
    let _ = std::fs::remove_dir_all(&vault);
    let _ = std::fs::remove_dir_all(&data);
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
    // I3, fix round 1: the cloud's own sentence comes back, not just the status digits — every
    // error body in `cloud/supabase/functions/_shared/http.ts` is `{"error":"<one sentence>"}` and
    // the four sentences a student can actually hit are all actionable.
    let (base, handle) = loopback_once(400, r#"{"error":"bad"}"#);
    let err = knowlu::report::send_at(&base, "tok-abc", "hello", "profile-1").unwrap_err();
    assert!(err.contains("bad"), "{err}");
    assert!(err.contains("not accepted"), "{err}");
    let _ = handle.join();
}

#[test]
fn send_at_falls_back_to_the_status_when_the_reply_carries_no_error_sentence() {
    let (base, handle) = loopback_once(500, "not json");
    let err = knowlu::report::send_at(&base, "tok-abc", "hello", "profile-1").unwrap_err();
    assert!(err.contains("500"), "{err}");
    let _ = handle.join();
}

#[test]
fn send_at_refuses_a_non_https_non_loopback_api_base() {
    let err = knowlu::report::send_at("http://example.com", "tok-abc", "hello", "profile-1").unwrap_err();
    assert!(err.contains("https://"), "{err}");
}

/// I2, fix round 1: `send_at` refuses an over-cap edited text itself, before it ever reaches the
/// network — `preview_text` already trims to fit, but the student can still edit the textarea back
/// over the cloud's line. No loopback server is stood up at all: the refusal has to happen before
/// any connection is attempted, and this proves it does not need one.
#[test]
fn send_at_refuses_an_over_cap_text_without_ever_touching_the_network() {
    let oversized = "x ".repeat(5000); // well past the cloud's 8,192-character MAX_BODY
    let err = knowlu::report::send_at("https://127.0.0.1:1", "tok-abc", &oversized, "profile-1").unwrap_err();
    assert!(err.contains("8192"), "{err}");
    assert!(err.to_ascii_lowercase().contains("shorten"), "{err}");
}
