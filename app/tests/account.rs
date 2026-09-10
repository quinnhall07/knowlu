//! `account.rs` against a loopback server, never the network (the 3a rule: `127.0.0.1` only, and the
//! serving thread is joined before the test returns, so a test can never outlive its own socket).
use knowlu::account::{auth_base, check_api_base, sign_in_at, sign_up_at, Session};
use std::io::{Read, Write};

/// Serves exactly `responses.len()` requests on `127.0.0.1:0`, then stops. Returns the base URL and
/// a handle whose `join()` yields the raw request text of each one — so a test can assert what went
/// on the wire without ever leaving the machine.
fn loopback(responses: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("accept");
            // Read the head, then exactly as many body bytes as Content-Length promised: a request
            // can arrive split across packets, and a single `read` would truncate it at random.
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
            seen.push(String::from_utf8_lossy(&buf).to_string());
            let reason = if (200..300).contains(&status) { "OK" } else { "Bad Request" };
            let resp = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
        seen
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

#[test]
fn the_auth_base_is_derived_from_the_functions_base_and_nothing_else() {
    // `config/cloud.yaml` carries four keys and the C2 contract fixes them, so the auth host is
    // derived rather than stored — and derived strictly, so a typo is a refusal and not a request
    // to somewhere unexpected.
    assert_eq!(auth_base("https://abc.supabase.co/functions/v1").unwrap(), "https://abc.supabase.co/auth/v1");
    assert_eq!(auth_base("https://abc.supabase.co/functions/v1/").unwrap(), "https://abc.supabase.co/auth/v1");
    assert_eq!(auth_base("http://127.0.0.1:9/functions/v1").unwrap(), "http://127.0.0.1:9/auth/v1");
    assert!(auth_base("https://abc.supabase.co").is_err());
    assert!(auth_base("https://abc.supabase.co/rest/v1").is_err());
}

#[test]
fn only_https_and_the_loopback_test_seam_are_accepted_as_an_api_base() {
    assert!(check_api_base("https://abc.supabase.co/functions/v1").is_ok());
    assert!(check_api_base("http://127.0.0.1:5051/functions/v1").is_ok());
    assert!(check_api_base("http://abc.supabase.co/functions/v1").is_err());
    assert!(check_api_base("ftp://abc/functions/v1").is_err());
}

#[test]
fn a_successful_sign_in_returns_a_session_and_sends_the_anon_key() {
    let body = r#"{"access_token":"at1","refresh_token":"rt1","expires_in":3600,"user":{"id":"acc-1","email":"a@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let out = sign_in_at(&format!("{base}/auth/v1"), "anon-key", "a@example.invalid", "pw", 1_760_000_000);
    let seen = handle.join().expect("server thread");
    let s: (String, Session) = out.expect("sign in");
    assert_eq!(s.0, "acc-1");
    assert_eq!(s.1.access_token, "at1");
    assert_eq!(s.1.refresh_token, "rt1");
    assert_eq!(s.1.email, "a@example.invalid");
    assert_eq!(s.1.expires_at, 1_760_000_000 + 3600);
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/token?grant_type=password "), "{req}");
    assert!(req.to_lowercase().contains("apikey: anon-key"), "{req}");
    // The password is on the wire because that is what signing in is — but it is never in a log,
    // a message or this assertion. Only the field NAME is checked.
    assert!(req.contains("\"password\""), "{req}");
}

/// The magic link's second half, which is what makes the button on the panel honest.
#[test]
fn a_six_digit_code_from_the_email_becomes_a_session_on_this_machine() {
    use knowlu::account::verify_email_code_at;
    let body = r#"{"access_token":"at9","refresh_token":"rt9","expires_in":3600,"user":{"id":"acc-9","email":"c@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let out = verify_email_code_at(&format!("{base}/auth/v1"), "anon-key", "c@example.invalid", " 123456 ", 0);
    let seen = handle.join().expect("server thread");
    let (id, sess) = out.expect("verify");
    assert_eq!(id, "acc-9");
    assert_eq!(sess.access_token, "at9");
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/verify "), "{req}");
    assert!(req.contains("\"type\":\"magiclink\""), "{req}");
    // Trimmed: a code pasted out of a mail client arrives with whitespace around it more often than not.
    assert!(req.contains("\"token\":\"123456\""), "{req}");
}

#[test]
fn a_refused_sign_in_is_the_providers_sentence_and_never_a_status_code() {
    let (base, handle) = loopback(vec![(400, r#"{"error_description":"Invalid login credentials"}"#.to_string())]);
    let out = sign_in_at(&format!("{base}/auth/v1"), "anon-key", "a@example.invalid", "pw", 0);
    let _ = handle.join().expect("server thread");
    assert_eq!(out.unwrap_err(), "Invalid login credentials");
}

#[test]
fn signing_up_sends_the_attestation_and_both_policy_versions_as_user_metadata() {
    let body = r#"{"access_token":"at1","refresh_token":"rt1","expires_in":3600,"user":{"id":"acc-2","email":"b@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let out = sign_up_at(&format!("{base}/auth/v1"), "anon-key", "b@example.invalid", "pw", "2026-09-10", "2026-09-10", 0);
    let seen = handle.join().expect("server thread");
    assert!(out.is_ok(), "{:?}", out.err());
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/signup "), "{req}");
    // The trigger in migration 20260910000100 refuses a sign-up without all three, so a client that
    // forgot one would fail at the database with a message nobody could act on. This is the pin.
    assert!(req.contains("\"age_attested\":\"true\""), "{req}");
    assert!(req.contains("\"tos_version\":\"2026-09-10\""), "{req}");
    assert!(req.contains("\"privacy_version\":\"2026-09-10\""), "{req}");
}

/// Credential Manager is real on this machine, so this test uses two targets of its own naming and
/// removes them itself. No secret is asserted on: only the account id and the fact of the move.
#[cfg(windows)]
#[test]
fn a_session_moves_from_the_pending_target_to_the_profiles_own() {
    use knowlu::account::{load_session, move_session, save_session, Session};
    let tag = format!("knowlu/test-{}-{}", std::process::id(), line!());
    let (from, to) = (format!("{tag}/pending"), format!("{tag}/profile_1"));
    let s = Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 42, email: "a@example.invalid".into() };
    save_session(&from, "acc-1", &s).expect("write the pending session");
    move_session(&from, &to).expect("move");
    let (id, back) = load_session(&to).expect("read the moved session");
    assert_eq!(id, "acc-1");
    assert_eq!(back, s);
    assert!(load_session(&from).is_err(), "the pending entry must be gone");
    let _ = knowlu::credentials::delete(&to);
}
