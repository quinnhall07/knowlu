//! `lms_link.rs`'s pure half, and the two promises the impure half makes. Nothing here opens a
//! window or reaches a network: the fetch is a closure, and the window rules are checked by reading
//! the source, which is the only way to assert "this path is not the app's" without a live WebView2.
use knowlu::lms_link::{first_ics_link, looks_like_ics, session_dir, summarise, validate, CaptureError};

/// Read a whole HTTP request off a loopback socket: the head, then exactly as many body bytes as
/// `content-length` promised. A single `read` truncates a request that arrives in two segments —
/// the flake `app/tests/account.rs` documents at its own loopback helper — and both tests below
/// assert on the body.
fn read_request(stream: &mut std::net::TcpStream) -> String {
    use std::io::Read;
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let head_end = buf.windows(4).position(|w| w == b"\r\n\r\n");
        if let Some(end) = head_end {
            let head = String::from_utf8_lossy(&buf[..end]).to_ascii_lowercase();
            let want: usize = head
                .split("content-length:")
                .nth(1)
                .and_then(|s| s.split("\r\n").next())
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            if buf.len() >= end + 4 + want { break; }
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

#[test]
fn a_calendar_link_is_recognised_by_shape_and_nothing_else_is() {
    assert!(looks_like_ics("https://lms.example.invalid/webapps/calendar/feed/abc/learn.ics"));
    assert!(looks_like_ics("https://example.instructure.com/feeds/calendars/user_abc.ics"));
    assert!(looks_like_ics("https://lms.example.invalid/calendar/export?format=ics&token=abc"));
    assert!(!looks_like_ics("http://lms.example.invalid/x.ics"), "https only — a feed URL is a capability");
    assert!(!looks_like_ics("https://lms.example.invalid/calendar"));
    assert!(!looks_like_ics("nonsense"));
}

#[test]
fn the_first_link_is_taken_out_of_whatever_the_page_gave_us() {
    let html = r#"<a href="https://lms.example.invalid/feed/a.ics">Copy</a> and https://other.invalid/b.ics"#;
    assert_eq!(first_ics_link(html).unwrap(), "https://lms.example.invalid/feed/a.ics");
    assert_eq!(first_ics_link("  https://lms.example.invalid/feed/a.ics  ").unwrap(), "https://lms.example.invalid/feed/a.ics");
    assert_eq!(first_ics_link("no link here"), None);
}

/// **The shape Task 13's spike actually saw** (outcome B): a plain `GET` of the campus feed
/// endpoint, carrying the student's own session cookies, answers 200 with the feed URL as the
/// *entire body* — no HTML, no JSON, no trailing newline guaranteed. That body is what
/// `capture_calendar_link` hands to `first_ics_link`, so it is worth its own case; the Canvas half
/// of the same function is the documented `calendar.ics` field of `GET /api/v1/users/self`, which
/// is JSON and has never been run (no Canvas login was available to the spike).
#[test]
fn the_feed_endpoint_answers_with_the_bare_url_and_that_is_the_link() {
    let body = "https://ualearn.blackboard.com/webapps/calendar/calendarFeed/0123456789abcdef/learn.ics";
    assert_eq!(first_ics_link(body).unwrap(), body);
    assert_eq!(first_ics_link(&format!("{body}\r\n")).unwrap(), body);
    let canvas = r#"{"id":42,"calendar":{"ics":"https://example.instructure.com/feeds/calendars/user_abc.ics"}}"#;
    assert_eq!(first_ics_link(canvas).unwrap(), "https://example.instructure.com/feeds/calendars/user_abc.ics");
}

#[test]
fn a_feed_is_summarised_by_events_and_course_codes() {
    let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:MATH 125 Homework 4\r\nEND:VEVENT\r\n\
               BEGIN:VEVENT\r\nSUMMARY:MATH 125 Quiz 2\r\nEND:VEVENT\r\n\
               BEGIN:VEVENT\r\nSUMMARY:SPAN 101 Lectura\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    assert_eq!(summarise(ics), (3, 2));
    assert_eq!(summarise("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n"), (0, 0));
}

#[test]
fn validate_reports_what_the_student_will_see_and_refuses_an_empty_feed() {
    let good = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:MATH 125 Homework 4\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let link = validate("https://lms.example.invalid/a.ics", &|_| Ok(good.to_string())).expect("validate");
    assert_eq!(link.url, "https://lms.example.invalid/a.ics");
    assert_eq!((link.events, link.courses), (1, 1));
    // An empty parse is a failure, never an empty semester — the same rule `coursework` follows.
    assert!(matches!(validate("https://lms.example.invalid/a.ics", &|_| Ok(String::new())), Err(CaptureError::Empty)));
    assert!(matches!(
        validate("https://lms.example.invalid/a.ics", &|_| Err("connection refused".into())),
        Err(CaptureError::Unreachable(_))
    ));
    assert!(matches!(validate("http://lms.example.invalid/a.ics", &|_| Ok(good.to_string())), Err(CaptureError::NotACalendarLink(_))));
}

/// **One vocabulary, three copies** (Interfaces with C2, item 3): the SQL check constraint, the
/// endpoint's `SOURCE_KINDS`, and this crate's. C2 adds no kind of its own — `google_calendar` is
/// already in all three — so the only way they drift is a careless edit, which is exactly what a
/// cheap cross-language read catches. The same trick
/// `the_action_vocabulary_is_the_engines_on_both_sides_of_the_wire` uses.
#[test]
fn the_source_kind_vocabulary_is_one_list_in_three_places() {
    use knowlu::lms_link::{DEVICE_KINDS, SOURCE_KINDS};
    let sql = std::fs::read_to_string("../cloud/supabase/migrations/20260910000100_accounts.sql").expect("the accounts migration");
    let ts = std::fs::read_to_string("../cloud/supabase/functions/account/handler.ts").expect("the account handler");
    // **Anchor on the table before looking for the constraint.** `consents` also has a
    // `check (kind in (…))`, and it is declared first — a split on the check alone reads `('tos',
    // 'privacy', 'age_18', 'auto_renew')` and fails on `lms_ics` with a message about the wrong table.
    // Slice `create table public.sources` … its terminating `);` first, then look inside that.
    let sources_block = sql
        .split("create table public.sources")
        .nth(1)
        .and_then(|s| s.split("\n);").next())
        .expect("the sources table");
    let constraint = sources_block
        .split("check (kind in (")
        .nth(1)
        .and_then(|s| s.split(')').next())
        .expect("the sources check constraint");
    // Anchored on the declaration, not on the name: `SOURCE_KINDS` is also *used* further down the
    // file, and a split on the bare name would take whichever came first if the two ever swapped.
    let listed = ts
        .split("const SOURCE_KINDS = [")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .expect("the endpoint's SOURCE_KINDS declaration");
    // The Rust copy needs no extraction at all — this test imports the constant itself, which is
    // stronger than reading its own crate's source and is why there is no third slice here.
    for kind in SOURCE_KINDS {
        assert!(constraint.contains(kind), "the check constraint does not allow {kind}");
        assert!(listed.contains(kind), "the endpoint does not accept {kind}");
    }
    assert_eq!(listed.matches('"').count() / 2, SOURCE_KINDS.len(), "the endpoint lists a kind this crate does not: {listed}");
    assert_eq!(constraint.matches('\'').count() / 2, SOURCE_KINDS.len(), "the constraint allows a kind this crate does not: {constraint}");
    // …and the device writes only two of the three: `google_calendar` is reserved and written by
    // nobody (R-X-9) — the Google grant lives in C2's `google_accounts` — so nothing here may produce one.
    assert_eq!(DEVICE_KINDS.len(), 2);
    assert!(!DEVICE_KINDS.contains(&"google_calendar"));
}

/// The two rules `validate_for` adds on top of `validate`, driven directly: an unknown kind is refused
/// before anything is fetched, and an **empty personal calendar is a connection, not a failure** —
/// while an empty school feed still is one ("an empty parse is a failure, never an empty semester").
#[test]
fn an_empty_personal_calendar_connects_and_an_empty_school_feed_does_not() {
    use knowlu::lms_link::validate_for;
    let empty = |_: &str| Ok("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n".to_string());
    let one = |_: &str| Ok("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Dentist\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n".to_string());
    let url = "https://calendar.google.com/calendar/ical/x/private-def/basic.ics";
    let link = validate_for("calendar_ics", url, &empty).expect("an empty calendar is still connected");
    assert_eq!((link.events, link.courses), (0, 0));
    assert_eq!(link.url, url);
    assert_eq!(validate_for("calendar_ics", url, &one).unwrap().events, 1);
    assert!(matches!(validate_for("lms_ics", url, &empty), Err(CaptureError::Empty)));
    // `google_calendar` is a real kind of the account's, and not one this app may write.
    assert!(matches!(validate_for("google_calendar", url, &one), Err(CaptureError::NotACalendarLink(_))));
    assert!(matches!(validate_for("nonsense", url, &one), Err(CaptureError::NotACalendarLink(_))));
}

/// The account copy of the link, on the wire (Interfaces with C2, item 3). Loopback only: the server
/// is a real socket on `127.0.0.1` and its thread is joined before this returns.
#[test]
fn a_validated_link_is_put_to_the_accounts_sources() {
    use knowlu::lms_link::put_source_at;
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let req = read_request(&mut stream);
        let body = r#"{"kind":"lms_ics"}"#;
        let _ = stream.write_all(
            format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes(),
        );
        req
    });
    let out = put_source_at(&format!("http://127.0.0.1:{port}/functions/v1"), "the-access-token", "lms_ics", "https://lms.example.invalid/feed/a.ics");
    let req = handle.join().expect("server thread");
    assert!(out.is_ok(), "{:?}", out.err());
    assert!(req.starts_with("PUT /functions/v1/account/sources "), "{req}");
    assert!(req.to_lowercase().contains("authorization: bearer the-access-token"), "{req}");
    assert!(req.contains("\"kind\":\"lms_ics\""), "{req}");
    assert!(req.contains("https://lms.example.invalid/feed/a.ics"), "{req}");
}

/// …and an unsubscribed account gets a sentence a student can act on, not a status code — and the
/// panel is not blocked by it, because the vault copy is what `ingest` reads until C2 ships.
#[test]
fn a_402_from_sources_is_a_sentence_and_not_a_dead_end() {
    use knowlu::lms_link::put_source_at;
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let _ = read_request(&mut stream);
        let body = r#"{"error":"this account has no active subscription"}"#;
        let _ = stream.write_all(
            format!("HTTP/1.1 402 Payment Required\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).as_bytes(),
        );
    });
    let out = put_source_at(&format!("http://127.0.0.1:{port}/functions/v1"), "t", "lms_ics", "https://lms.example.invalid/a.ics");
    handle.join().expect("server thread");
    assert!(out.unwrap_err().contains("finish subscribing"));
}

/// Spec §11a and the standing rule. Two promises, asserted against the source because there is no
/// other way to assert them without a live WebView2 — and both are the kind of thing a refactor
/// breaks silently.
#[test]
fn the_sign_in_window_keeps_nothing_and_lives_nowhere_near_the_app_data() {
    let src = std::fs::read_to_string("src/lms_link.rs").expect("src/lms_link.rs");
    // Its data directory is the system temp folder's, never `%LOCALAPPDATA%\knowlu`.
    assert!(src.contains("std::env::temp_dir()"), "the session directory must be the temp folder's");
    assert!(!src.contains("app_data_root"), "a campus session must never land in the app's own data");
    assert!(!src.contains("profile_dir"), "…nor in a profile folder");
    // …and it is deleted.
    assert!(src.contains("remove_dir_all"), "close_and_wipe must delete the session directory");
    // Nothing here reads, stores or transmits a campus credential. These words appear in this file
    // only inside the sentence that says so.
    for word in ["password", "set_password", "credentials::write"] {
        let hits = src.matches(word).count();
        assert!(hits <= 1, "{word} appears {hits} times in lms_link.rs — it may only appear in the promise");
    }
    // The session directory is per capture, so two captures never share cookies.
    assert_ne!(session_dir(), session_dir());
    // The same promise driven, not read (R-C1-40, M1): under the system temp folder, and under
    // neither the app's own data root nor anything inside a profile. `app_data_root_in` is not
    // called and neither is `app_data_root` — the first creates the folder, the second runs the
    // flat-layout migration against the machine's live app data, and a test has no business doing
    // either; the path it decides is one `join`, repeated here rather than provoked.
    let dir = session_dir();
    assert!(dir.starts_with(std::env::temp_dir()), "{}", dir.display());
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        let root = std::path::Path::new(&base).join("knowlu");
        assert!(!dir.starts_with(&root), "a campus session must never land in {}", root.display());
    }
    assert!(!dir.to_string_lossy().contains("profiles"), "…nor in a profile folder: {}", dir.display());
}

/// **The §11a invariant, pinned** (R-C1-40, M3). Two edits would silently hand the campus's own page
/// an IPC bridge into this app: a capability file that names the sign-in window, and a `remote`
/// urls block on any capability at all — `tauri 2.11.5` gates a non-local origin on exactly those
/// two (`webview/mod.rs`'s ACL extension for remote origins, and `ipc/authority.rs`'s
/// `origin.matches(&cmd.context)`). Neither exists, and this is what keeps it that way.
#[test]
fn no_capability_names_the_sign_in_window() {
    let dir = std::path::Path::new("capabilities");
    let mut seen = 0usize;
    for entry in std::fs::read_dir(dir).expect("app/capabilities").flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        seen += 1;
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for key in ["windows", "webviews"] {
            let labels = v.get(key).and_then(|x| x.as_array()).cloned().unwrap_or_default();
            for label in labels {
                assert_ne!(
                    label.as_str().unwrap_or_default(),
                    knowlu::lms_link::WINDOW,
                    "{} grants capabilities to the sign-in window through `{key}`",
                    path.display()
                );
            }
        }
        assert!(!text.contains("\"remote\""), "{} carries a remote urls block", path.display());
    }
    assert!(seen > 0, "no capability files were read — the walk found nothing to check");
}

/// The recursive delete cannot be aimed anywhere but at a directory this module made (R-C1-40, I2).
/// `close_lms_window` no longer takes a path at all; this is the second lock on the same door, and
/// the one a refactor that reintroduces a parameter would still have to get past.
#[test]
fn only_a_session_directory_of_ours_can_be_wiped() {
    use knowlu::lms_link::is_session_dir;
    let temp = std::env::temp_dir();
    assert!(is_session_dir(&session_dir()));
    assert!(!is_session_dir(&temp), "the temp folder itself is not a session directory");
    assert!(!is_session_dir(&temp.join("something-else")));
    assert!(!is_session_dir(&temp.join("sub").join("knowlu-lms-session-1-cap")), "only directly under the temp folder");
    assert!(!is_session_dir(std::path::Path::new("C:\\Users")));
    assert!(!is_session_dir(std::path::Path::new("")));
}

/// The sweep that cleans up after a crash (R-C1-40, I1c). Driven against a scratch folder with an
/// explicit `now`, so it needs neither a second process nor a wait: a directory this process owns
/// stays whatever its age, a stranger's stays until it is old enough, and a folder that is not a
/// session directory at all is never touched.
#[test]
fn the_sweep_takes_only_stale_sessions_that_are_not_ours() {
    use knowlu::lms_link::sweep_stale_sessions_in;
    let scratch = std::env::temp_dir().join(format!("knowlu-sweep-test-{}-{}", std::process::id(), knowlu_engine::ids::new_id("sw")));
    let mine = "knowlu-lms-session-4242-";
    let ours = scratch.join("knowlu-lms-session-4242-capA");
    let theirs = scratch.join("knowlu-lms-session-9999-capB");
    let stranger = scratch.join("some-other-folder");
    for d in [&ours, &theirs, &stranger] { std::fs::create_dir_all(d).expect("scratch dirs"); }
    std::fs::write(theirs.join("cookies.db"), b"x").expect("a file inside the profile");

    // An hour has not passed, so nothing is old enough yet.
    let now = std::time::SystemTime::now();
    assert_eq!(sweep_stale_sessions_in(&scratch, mine, now, std::time::Duration::from_secs(3600)), 0);
    assert!(theirs.exists());

    // …and once it has, the stranger's session goes and ours does not.
    let later = now + std::time::Duration::from_secs(7200);
    assert_eq!(sweep_stale_sessions_in(&scratch, mine, later, std::time::Duration::from_secs(3600)), 1);
    assert!(!theirs.exists(), "a stale session directory from a dead process must be swept");
    assert!(ours.exists(), "this process's own open session must never be swept");
    assert!(stranger.exists(), "the sweep must not touch anything that is not a session directory");
    let _ = std::fs::remove_dir_all(&scratch);
}
