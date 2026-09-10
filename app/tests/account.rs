//! `account.rs` against a loopback server, never the network (the 3a rule: `127.0.0.1` only, and the
//! serving thread is joined before the test returns, so a test can never outlive its own socket).
use knowlu::account::{auth_base, check_api_base, sign_in_at, sign_up_at, Session};
use std::io::{Read, Write};

/// Serves exactly `responses.len()` requests on `127.0.0.1:0`, then stops. Returns the base URL and
/// a handle whose `join()` yields the raw request text of each one — so a test can assert what went
/// on the wire without ever leaving the machine. Bounded on both ends: `accept` polls a nonblocking
/// listener against a 10-second deadline and panics past it, and the accepted stream carries its own
/// read timeout — so a test that forgets to send a request fails loudly in seconds instead of hanging
/// the suite.
fn loopback(responses: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.set_nonblocking(true).expect("nonblocking listener");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for (status, body) in responses {
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
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .expect("read timeout");
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
    // A loopback-looking prefix with an `@` in the authority is a different host after the `@` —
    // the classic userinfo trick — and must be refused even though the string starts with the
    // literal allowed prefix.
    assert!(check_api_base("http://127.0.0.1:x@evil.com/functions/v1").is_err());
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
    assert!(req.to_lowercase().contains("content-type: application/json"), "{req}");
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

#[test]
fn a_sessions_debug_output_redacts_both_tokens() {
    let s = Session {
        access_token: "super-secret-access-token".into(),
        refresh_token: "super-secret-refresh-token".into(),
        expires_at: 42,
        email: "a@example.invalid".into(),
    };
    let out = format!("{s:?}");
    assert!(!out.contains("super-secret-access-token"), "{out}");
    assert!(!out.contains("super-secret-refresh-token"), "{out}");
    assert!(out.contains("<redacted>"), "{out}");
    assert!(out.contains("a@example.invalid"), "{out}");
    assert!(out.contains("42"), "{out}");
}

/// Deletes every named Credential Manager target when the test ends, on any exit path — a passing
/// assertion, a failing one, or a panic. Constructed before the first credential a test writes, so
/// nothing written can outlive the test that wrote it.
#[cfg(windows)]
struct Cleanup(Vec<String>);

#[cfg(windows)]
impl Drop for Cleanup {
    fn drop(&mut self) {
        for target in &self.0 {
            let _ = knowlu::credentials::delete(target);
        }
    }
}

/// Credential Manager is real on this machine, so this test uses two targets of its own naming and
/// removes them itself. No secret is asserted on: only the account id and the fact of the move.
#[cfg(windows)]
#[test]
fn a_session_moves_from_the_pending_target_to_the_profiles_own() {
    use knowlu::account::{load_session, move_session, save_session, Session};
    let tag = format!("knowlu/test-{}-{}", std::process::id(), line!());
    let (from, to) = (format!("{tag}/pending"), format!("{tag}/profile_1"));
    let _cleanup = Cleanup(vec![from.clone(), to.clone()]);
    let s = Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 42, email: "a@example.invalid".into() };
    save_session(&from, "acc-1", &s).expect("write the pending session");
    move_session(&from, &to).expect("move");
    let (id, back) = load_session(&to).expect("read the moved session");
    assert_eq!(id, "acc-1");
    assert_eq!(back, s);
    assert!(load_session(&from).is_err(), "the pending entry must be gone");
}

/// A `move_session` retried after it already succeeded — `from` gone, `to` already holding the
/// session — must not fail: the caller could not tell "already moved" from "never existed" any other
/// way, and treating it as an error would turn a harmless retry into a lost session.
#[cfg(windows)]
#[test]
fn moving_an_already_moved_session_is_a_no_op_not_an_error() {
    use knowlu::account::{load_session, move_session, save_session, Session};
    let tag = format!("knowlu/test-{}-{}", std::process::id(), line!());
    let (from, to) = (format!("{tag}/pending"), format!("{tag}/profile_1"));
    let _cleanup = Cleanup(vec![from.clone(), to.clone()]);
    let s = Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 1, email: "a@example.invalid".into() };
    save_session(&to, "acc-1", &s).expect("write directly to the destination, as if an earlier call already moved it");
    move_session(&from, &to).expect("a missing source with an already-populated destination is not an error");
    let (id, back) = load_session(&to).expect("the destination is untouched");
    assert_eq!(id, "acc-1");
    assert_eq!(back, s);
}

/// A destination that already holds a (stale) session loses to the one being moved — the moved
/// session always wins, never the one it is replacing.
#[cfg(windows)]
#[test]
fn moving_into_an_existing_destination_overwrites_it_with_the_moved_session() {
    use knowlu::account::{load_session, move_session, save_session, Session};
    let tag = format!("knowlu/test-{}-{}", std::process::id(), line!());
    let (from, to) = (format!("{tag}/pending"), format!("{tag}/profile_1"));
    let _cleanup = Cleanup(vec![from.clone(), to.clone()]);
    let stale = Session { access_token: "stale-at".into(), refresh_token: "stale-rt".into(), expires_at: 1, email: "old@example.invalid".into() };
    save_session(&to, "acc-old", &stale).expect("an old session already sits at the destination");
    let fresh = Session { access_token: "fresh-at".into(), refresh_token: "fresh-rt".into(), expires_at: 2, email: "new@example.invalid".into() };
    save_session(&from, "acc-new", &fresh).expect("the session actually being moved");
    move_session(&from, &to).expect("move");
    let (id, back) = load_session(&to).expect("read the destination");
    assert_eq!(id, "acc-new");
    assert_eq!(back, fresh);
    assert!(load_session(&from).is_err());
}

/// Only when there is nothing anywhere — no source, no destination — is `move_session` a real
/// failure, and the message names the source so whoever reads it knows what was being looked for.
#[cfg(windows)]
#[test]
fn moving_a_session_that_does_not_exist_to_a_destination_that_also_does_not_names_the_source() {
    use knowlu::account::move_session;
    let tag = format!("knowlu/test-{}-{}", std::process::id(), line!());
    let (from, to) = (format!("{tag}/pending"), format!("{tag}/profile_1"));
    let _cleanup = Cleanup(vec![from.clone(), to.clone()]);
    let err = move_session(&from, &to).unwrap_err();
    assert!(err.contains(&from), "{err}");
}

/// `sign_out` addresses a real profile's own session target, not just the pending one, and its reply
/// says whether there was anything to sign out of — a repeated sign-out is a visible no-op, not a
/// silent "ok" both times. No loopback needed: with `KNOWLU_API_BASE` unset in this test process,
/// `auth_base` refuses the placeholder default before any network call would be attempted.
#[cfg(windows)]
#[test]
fn signing_out_a_profile_reports_whether_it_had_a_session_and_removes_it() {
    use knowlu::account::{save_session, session_target, sign_out, Session};
    let id = format!("test-signout-{}-{}", std::process::id(), line!());
    let target = session_target(&id);
    let _cleanup = Cleanup(vec![target.clone()]);

    let out = sign_out(Some(id.clone())).expect("sign out with nothing to sign out of");
    assert_eq!(out["ok"], true);
    assert_eq!(out["had_session"], false);

    let s = Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 99, email: "a@example.invalid".into() };
    save_session(&target, "acc-1", &s).expect("write a session to sign out of");
    let out = sign_out(Some(id)).expect("sign out with a real session");
    assert_eq!(out["ok"], true);
    assert_eq!(out["had_session"], true);
    assert!(!knowlu::credentials::exists(&target), "the credential must be gone after sign-out");
}

/// Credential Manager's blob is UTF-16 and Windows caps it well below what a 1,300-character token
/// would need; `save_session` must refuse before ever calling into Credential Manager, and nothing
/// must land there.
#[cfg(windows)]
#[test]
fn a_session_too_large_for_the_credential_blob_is_refused_before_anything_is_written() {
    use knowlu::account::{save_session, Session};
    let target = format!("knowlu/test-toolarge-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    let s = Session {
        access_token: "x".repeat(1_300),
        refresh_token: "rt".into(),
        expires_at: 1,
        email: "a@example.invalid".into(),
    };
    let err = save_session(&target, "acc-1", &s).unwrap_err();
    assert!(err.contains("too large"), "{err}");
    assert!(!knowlu::credentials::exists(&target), "nothing should have been written");
}

/// A blob that Credential Manager holds but this process did not write as a `Session` (corrupted, or
/// from a future version) must not leak serde's own parse error to a user — `load_session` reports
/// one fixed, actionable sentence.
#[cfg(windows)]
#[test]
fn a_corrupt_session_blob_reads_back_as_a_fixed_sentence_not_serdes_own_message() {
    use knowlu::account::load_session;
    let target = format!("knowlu/test-corrupt-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    knowlu::credentials::write(&target, "acc-x", "not json").expect("write a raw, non-session blob");
    let err = load_session(&target).unwrap_err();
    assert_eq!(err, "the stored session is unreadable; sign in again");
}
