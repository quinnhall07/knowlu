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

/// Fix round 1 stabilization: Windows Credential Manager is one shared, machine-wide store, and
/// this file's tests hit it from several threads at once (`cargo test` runs a binary's tests in
/// parallel by default). Under enough concurrent `CredWriteW`/`CredReadW`/`CredDeleteW` traffic —
/// crossing a threshold this file's two new `refresh_entitlement` tests (fix round 1, item 5) pushed
/// it over — a read for one target has been observed to spuriously report `ERROR_NOT_FOUND` for a
/// target no other thread ever touched, surfacing minutes later as an unrelated 10-second loopback
/// timeout. This serializes every test that reads, writes or deletes a real credential, the same way
/// `ENGINE_ENV_LOCK` in `tests/scheduler.rs` serializes process-global env var access.
#[cfg(windows)]
static CREDMAN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Credential Manager is real on this machine, so this test uses two targets of its own naming and
/// removes them itself. No secret is asserted on: only the account id and the fact of the move.
#[cfg(windows)]
#[test]
fn a_session_moves_from_the_pending_target_to_the_profiles_own() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::load_session;
    let target = format!("knowlu/test-corrupt-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    knowlu::credentials::write(&target, "acc-x", "not json").expect("write a raw, non-session blob");
    let err = load_session(&target).unwrap_err();
    assert_eq!(err, "the stored session is unreadable; sign in again");
}

#[test]
fn the_grace_is_seventy_two_hours_and_a_missing_cache_is_never_entitled() {
    use knowlu::account::{decide, EntitlementCache, EntitlementState};
    let at = |s: &str| EntitlementCache {
        status: s.to_string(),
        current_period_end: None,
        plan: Some("monthly".into()),
        checked_at: "2026-09-10T12:00:00.000Z".to_string(),
    };
    let now = |s: &str| s.parse::<jiff::Timestamp>().unwrap();
    // No account at all — a vault with no config/cloud.yaml. Not a failure: a named skipped step.
    assert_eq!(decide(false, Some(&at("active")), now("2026-09-10T12:00:00Z")), EntitlementState::NoAccount);
    // Never checked. The page still ranks; the cloud steps stand down.
    assert_eq!(decide(true, None, now("2026-09-10T12:00:00Z")), EntitlementState::NotEntitled);
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-10T12:00:01Z")), EntitlementState::Entitled);
    assert_eq!(decide(true, Some(&at("trialing")), now("2026-09-10T12:00:01Z")), EntitlementState::Entitled);
    // Spec §5.1: a dead hotel wifi must never blank today's page. Seventy-two hours, to the second.
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-13T11:59:59Z")), EntitlementState::Entitled);
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-13T12:00:01Z")), EntitlementState::NotEntitled);
    for bad in ["past_due", "canceled", "none", "whatever"] {
        assert_eq!(decide(true, Some(&at(bad)), now("2026-09-10T12:00:01Z")), EntitlementState::NotEntitled, "{bad}");
    }
    // Fix round 1, item 1: a clock a few minutes fast is not a reason to disentitle someone forever
    // — each refresh would otherwise write another "future" `checked_at` and the student never
    // recovers. A future stamp up to an hour ahead is treated as age 0.
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-10T11:55:00Z")), EntitlementState::Entitled);
    // …but a clock that is materially wrong is still not a licence: 24 hours ahead stays NotEntitled,
    // exactly as before this fix.
    assert_eq!(decide(true, Some(&at("active")), now("2026-09-09T12:00:00Z")), EntitlementState::NotEntitled);
}

#[test]
fn fetching_the_entitlement_parses_the_four_keys_and_sends_the_bearer_token() {
    use knowlu::account::fetch_entitlement_at;
    let body = r#"{"status":"trialing","current_period_end":"2026-09-17T00:00:00+00:00","plan":"monthly","checked_at":"2026-09-10T12:00:00.000Z"}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let got = fetch_entitlement_at(&format!("{base}/functions/v1"), "the-access-token");
    let seen = handle.join().expect("server thread");
    let c = got.expect("entitlement");
    assert_eq!(c.status, "trialing");
    assert_eq!(c.plan.as_deref(), Some("monthly"));
    assert_eq!(c.checked_at, "2026-09-10T12:00:00.000Z");
    assert!(seen[0].starts_with("GET /functions/v1/entitlement "), "{}", seen[0]);
    assert!(seen[0].to_lowercase().contains("authorization: bearer the-access-token"), "{}", seen[0]);
}

#[test]
fn a_cache_round_trips_through_the_profile_folder() {
    use knowlu::account::{cache_path, load_cache, save_cache, EntitlementCache};
    let dir = std::env::temp_dir().join(format!("knowlu-ent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert!(load_cache(&dir).is_none(), "a fresh profile has no cache");
    let c = EntitlementCache { status: "active".into(), current_period_end: None, plan: None, checked_at: "2026-09-10T12:00:00.000Z".into() };
    save_cache(&dir, &c).expect("save");
    assert_eq!(load_cache(&dir).as_ref(), Some(&c));
    assert_eq!(cache_path(&dir), dir.join("entitlement.json"));
    // A hand-mangled cache is "not entitled", never a panic and never a default that grants access.
    std::fs::write(cache_path(&dir), b"{").unwrap();
    assert!(load_cache(&dir).is_none(), "a corrupt cache file must never parse as Some");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fix round 1, item 5a: a profile with no session credential at all — the ordinary state for a
/// signed-out or never-signed-in install — must fail the refresh outright and leave no
/// `entitlement.json` behind. No loopback needed: `valid_access_token_at` refuses before any
/// network call would be attempted, exactly as `signing_out_a_profile_reports_whether_it_had_a_session`
/// relies on above.
#[cfg(windows)]
#[test]
fn refresh_entitlement_with_no_session_fails_and_writes_no_cache() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::{load_cache, refresh_entitlement};
    let target = format!("knowlu/test-refresh-nosession-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    assert!(!knowlu::credentials::exists(&target), "nothing must be there to begin with");

    let vault = std::env::temp_dir().join(format!("knowlu-refresh-nosession-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&vault);
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::write(
        vault.join("config").join("cloud.yaml"),
        format!("api_base: 'https://example.supabase.co/functions/v1'\nanon_key: 'anon'\nsession_credential_target: '{target}'\naccount_id: 'acc-1'\n"),
    ).unwrap();
    let data_dir = std::env::temp_dir().join(format!("knowlu-refresh-nosession-data-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&data_dir);

    let err = refresh_entitlement(&vault, &data_dir);
    assert!(err.is_err(), "no session credential means no refresh: {err:?}");
    assert!(load_cache(&data_dir).is_none(), "a failed refresh must write no cache");
    assert!(!data_dir.join("entitlement.json").exists(), "nothing was written at all");
    let _ = std::fs::remove_dir_all(&vault);
    let _ = std::fs::remove_dir_all(&data_dir);
}

/// Fix round 1, item 5b: a live (loopback) `/entitlement` reply is cached with exactly the four
/// values it carried — the round trip `refresh_entitlement` exists for. The session already has
/// plenty of life left, so `valid_access_token_at` never needs to refresh it and the loopback server
/// only ever has to answer the one `/entitlement` request.
#[cfg(windows)]
#[test]
fn refresh_entitlement_saves_the_cache_from_a_live_reply() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::{load_cache, refresh_entitlement, save_session, Session};
    let target = format!("knowlu/test-refresh-ok-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    let s = Session {
        access_token: "tok".into(),
        refresh_token: "rt".into(),
        expires_at: jiff::Timestamp::now().as_second() + 3600,
        email: "a@example.invalid".into(),
    };
    save_session(&target, "acc-1", &s).expect("write a live session for the refresh to find");

    let vault = std::env::temp_dir().join(format!("knowlu-refresh-ok-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&vault);
    std::fs::create_dir_all(vault.join("config")).unwrap();
    let data_dir = std::env::temp_dir().join(format!("knowlu-refresh-ok-data-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&data_dir);

    // Started LAST, immediately before the one call that reaches it: everything above is Credential
    // Manager and filesystem setup that can be slow under a loaded parallel test run, and the
    // loopback server's `accept()` carries a fixed 10s deadline that must not have to survive that
    // setup too — starting it first (as the loop's own doc suggests at a glance) intermittently let
    // the deadline expire before the client ever connected.
    let (base, handle) = loopback(vec![(
        200,
        r#"{"status":"active","current_period_end":"2026-10-01T00:00:00+00:00","plan":"monthly","checked_at":"2026-09-10T12:00:00.000Z"}"#.to_string(),
    )]);
    std::fs::write(
        vault.join("config").join("cloud.yaml"),
        format!("api_base: '{base}/functions/v1'\nanon_key: 'anon'\nsession_credential_target: '{target}'\naccount_id: 'acc-1'\n"),
    ).unwrap();

    let got = refresh_entitlement(&vault, &data_dir);
    let seen = handle.join().expect("server thread");
    let c = got.expect("refresh");
    assert_eq!(c.status, "active");
    assert!(seen[0].starts_with("GET /functions/v1/entitlement "), "{}", seen[0]);
    let cached = load_cache(&data_dir).expect("the cache was written");
    assert_eq!(cached.status, "active");
    assert_eq!(cached.current_period_end.as_deref(), Some("2026-10-01T00:00:00+00:00"));
    assert_eq!(cached.plan.as_deref(), Some("monthly"));
    assert_eq!(cached.checked_at, "2026-09-10T12:00:00.000Z");
    let _ = std::fs::remove_dir_all(&vault);
    let _ = std::fs::remove_dir_all(&data_dir);
}

/// Base64url without padding, as a JWT segment is — a dozen lines here rather than a `base64`
/// dependency the app does not otherwise need.
fn decode_b64url(s: &str) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let (mut bits, mut n, mut out) = (0u32, 0u32, Vec::new());
    for c in s.bytes() {
        let v = T.iter().position(|&t| t == c).expect("a base64url character") as u32;
        bits = (bits << 6) | v;
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push(((bits >> n) & 0xff) as u8);
            bits &= (1u32 << n) - 1;
        }
    }
    String::from_utf8(out).expect("a UTF-8 payload")
}

/// The one catastrophic version of filling the P1 constants is pasting the service-role key into a
/// constant that ships in every installer. Pinned statically, like the updater's flag ⇔ plugin test.
#[test]
fn the_compiled_in_project_is_a_prod_functions_base_and_an_anon_key() {
    assert!(check_api_base(knowlu::account::DEFAULT_API_BASE).is_ok());
    assert!(auth_base(knowlu::account::DEFAULT_API_BASE).is_ok());
    let payload = decode_b64url(knowlu::account::DEFAULT_ANON_KEY.split('.').nth(1).expect("a JWT has three segments"));
    assert!(payload.contains(r#""role":"anon""#), "the compiled-in key is not an anon key");
    let r = payload.split(r#""ref":""#).nth(1).expect("a ref claim").split('"').next().expect("a ref value");
    assert!(knowlu::account::DEFAULT_API_BASE.contains(r), "key and URL name different projects");
}

/// The process-global `KNOWLU_API_BASE`, set for the length of one test and restored on any exit
/// path — a panicking assertion included, which a straight-line set-call-restore is not. Every test
/// that sets it also holds [`CREDMAN_LOCK`]: this file's only other readers of `api_base()` are
/// credential tests, so the one lock covers both kinds of process-global state, exactly as
/// `tests/scheduler.rs`'s `EnvSeam` does for `KNOWLU_ENGINE_EXE`.
#[cfg(windows)]
struct ApiBase(Option<std::ffi::OsString>);

#[cfg(windows)]
impl ApiBase {
    fn set(value: &str) -> Self {
        let prev = std::env::var_os("KNOWLU_API_BASE");
        unsafe { std::env::set_var("KNOWLU_API_BASE", value) };
        ApiBase(prev)
    }
}

#[cfg(windows)]
impl Drop for ApiBase {
    fn drop(&mut self) {
        match self.0.take() {
            Some(v) => unsafe { std::env::set_var("KNOWLU_API_BASE", v) },
            None => unsafe { std::env::remove_var("KNOWLU_API_BASE") },
        }
    }
}

/// A loopback base **nothing is listening on**: a port is bound only long enough to learn that it is
/// free, then released. A request to it is refused in microseconds instead of waiting out a timeout,
/// and — the point — it can never leave this machine. Used where a call must not reach the network
/// and its failure is what the test is about.
#[cfg(windows)]
fn closed_loopback_base() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = l.local_addr().expect("addr").port();
    drop(l);
    format!("http://127.0.0.1:{port}/functions/v1")
}

/// Spec §11a: an install that already exists is **adopted in place**. Its folder, its settings, its
/// Credential Manager entries and its profile id all stay exactly where they are, and the account is
/// added to what is already there — no folder question, no second vault, no rewrite of a single file
/// the student already had.
///
/// `KNOWLU_API_BASE` points at a closed port for the whole test: `attach_in` ends by back-filling the
/// LMS feed to the account, and that attempt must fail here rather than reach anything real. Its
/// failing is also a claim worth making — the adoption succeeds anyway.
#[cfg(windows)]
#[test]
fn a_vault_from_before_c1_needs_an_account_and_gains_one_without_moving() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _api = ApiBase::set(&closed_loopback_base());
    use knowlu::account::{attach_in, needs_account, save_session, Session, PENDING_TARGET};
    let root = std::env::temp_dir().join(format!("knowlu-adopt-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&root);
    let vault = root.join("Fall 2026");
    // A pre-C1 vault: config/planning.yaml, tasks/, runners.yaml — and NO cloud.yaml.
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::create_dir_all(vault.join("tasks")).unwrap();
    std::fs::write(vault.join("config").join("planning.yaml"), "daily_effort_budget: 4.0\n").unwrap();
    std::fs::write(vault.join("config").join("ingest.yaml"), "ics_url: 'https://lms.example.invalid/a.ics'\n").unwrap();
    let before = std::fs::read_to_string(vault.join("config").join("ingest.yaml")).unwrap();
    assert!(needs_account(&vault), "a vault with no cloud.yaml needs one");

    // The wizard-less upgrade signs in first, into the pending target, exactly as the wizard does.
    let target = format!("knowlu/test-adopt-{}-{}/pending", std::process::id(), line!());
    let profile_id = knowlu::profiles::id_for(&vault);
    let _cleanup = Cleanup(vec![target.clone(), knowlu::account::session_target(&profile_id)]);
    save_session(&target, "acc-9", &Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 9, email: "a@example.invalid".into() }).unwrap();
    attach_in(&vault, &profile_id, &target).expect("attach");

    assert!(!needs_account(&vault), "…and stops needing one");
    let cfg = knowlu::account::cloud_config(&vault).expect("cloud_config");
    assert_eq!(cfg.account_id, "acc-9");
    assert_eq!(cfg.session_credential_target, format!("knowlu/{profile_id}/session"));
    // Nothing else in the vault moved: the folder, the feed and the profile id are exactly as they
    // were. That is the whole ruling.
    assert_eq!(std::fs::read_to_string(vault.join("config").join("ingest.yaml")).unwrap(), before);
    assert_eq!(knowlu::profiles::id_for(&vault), profile_id);
    // The session moved onto the profile and left the pending target.
    let (id, _) = knowlu::account::load_session(&cfg.session_credential_target).expect("session");
    assert_eq!(id, "acc-9");
    assert!(knowlu::account::load_session(&target).is_err());
    // Adopting twice is refused **for the reason the test is named for** — the vault already has an
    // account — and not incidentally because the pending session has since been moved away. So the
    // second attempt is given a live pending session, and the error text is asserted.
    save_session(&target, "acc-9", &Session { access_token: "at".into(), refresh_token: "rt".into(), expires_at: 9, email: "a@example.invalid".into() }).unwrap();
    let again = attach_in(&vault, &profile_id, &target).unwrap_err();
    assert!(again.contains("already has an account"), "{again}");
    let _ = PENDING_TARGET;
    let _ = std::fs::remove_dir_all(&root);
}

/// The back-fill (Interfaces with C2, item 3). A vault onboarded before the account existed carries
/// its feeds in `config/ingest.yaml` and in no `sources` row — and C2's `/ingest/ics` reads that row.
/// Adoption sends **both** of them, once each, with this profile's own bearer: the school feed as
/// `lms_ics` and the first personal calendar as `calendar_ics`.
#[cfg(windows)]
#[test]
fn an_adopted_vaults_two_feeds_are_back_filled_to_the_account() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::{attach_in, save_session, Session};
    let root = std::env::temp_dir().join(format!("knowlu-backfill-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&root);
    let vault = root.join("Fall 2026");
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::create_dir_all(vault.join("tasks")).unwrap();
    std::fs::write(vault.join("config").join("planning.yaml"), "daily_effort_budget: 4.0\n").unwrap();
    // The shape `scaffold::ingest_yaml` writes: a school feed, and `calendars:` as a block list of
    // `{name, ics_url}` mappings.
    std::fs::write(
        vault.join("config").join("ingest.yaml"),
        "ics_url: 'https://lms.example.invalid/a.ics'\ntimezone: 'America/Chicago'\ncalendars:\n  - name: personal\n    ics_url: 'https://cal.example.invalid/b.ics'\n",
    ).unwrap();

    let profile_id = knowlu::profiles::id_for(&vault);
    let target = format!("knowlu/test-backfill-{}-{}/pending", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone(), knowlu::account::session_target(&profile_id)]);
    // Plenty of life left, so `valid_access_token_at` never has to refresh and the loopback only has
    // to answer the two PUTs the back-fill itself makes.
    let s = Session {
        access_token: "tok".into(),
        refresh_token: "rt".into(),
        expires_at: jiff::Timestamp::now().as_second() + 3600,
        email: "a@example.invalid".into(),
    };
    save_session(&target, "acc-7", &s).expect("write the pending session");

    // Started last, immediately before the call that reaches it — the `accept()` deadline must not
    // have to outlast the Credential Manager setup above (the same ordering
    // `refresh_entitlement_saves_the_cache_from_a_live_reply` documents).
    let (base, handle) = loopback(vec![(200, "{\"ok\":true}".to_string()), (200, "{\"ok\":true}".to_string())]);
    let _api = ApiBase::set(&format!("{base}/functions/v1"));
    attach_in(&vault, &profile_id, &target).expect("attach");
    let seen = handle.join().expect("server thread");

    assert_eq!(seen.len(), 2, "one call per feed, and no more");
    assert!(seen[0].starts_with("PUT /functions/v1/account/sources "), "{}", seen[0]);
    assert!(seen[0].contains("\"kind\":\"lms_ics\"") && seen[0].contains("a.ics"), "{}", seen[0]);
    assert!(seen[1].contains("\"kind\":\"calendar_ics\"") && seen[1].contains("b.ics"), "{}", seen[1]);
    // The bearer is sent; its value is nobody's business, here least of all.
    assert!(seen[0].to_lowercase().contains("authorization: bearer "), "the request carries a bearer");
    assert!(knowlu::account::cloud_config(&vault).is_ok(), "the adoption itself still landed");
    let _ = std::fs::remove_dir_all(&root);
}

/// Spec §4.1's *Delete my data*, driven over scratch folders under the temp dir — never a real
/// profile, and with the server call and the window's exit left out, because neither is what this is
/// about. **The list is exactly this long**: the vault, THIS profile's subtree of the shared backups
/// root, this profile's app data, this profile's row in the registry, its session and its two
/// coursework logins. A second profile on the same machine — its vault, its snapshots, its app data,
/// its registry row, its logins — comes through untouched, and so does the backups root itself.
#[cfg(windows)]
#[test]
fn deleting_my_data_removes_this_profiles_things_and_nothing_else() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = std::env::temp_dir().join(format!("knowlu-delete-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&root);
    let mine = root.join("Fall 2026");
    let theirs = root.join("Someone else");
    std::fs::create_dir_all(mine.join("tasks")).unwrap();
    std::fs::create_dir_all(theirs.join("tasks")).unwrap();
    std::fs::write(mine.join("tasks").join("a.md"), "mine").unwrap();
    std::fs::write(theirs.join("tasks").join("b.md"), "theirs").unwrap();
    let (my_id, their_id) = (knowlu::profiles::id_for(&mine), knowlu::profiles::id_for(&theirs));

    // One backups ROOT, two profiles inside it — the whole reason this test exists.
    let backups = root.join("Backups");
    std::fs::create_dir_all(backups.join(&my_id).join("vault").join("tasks")).unwrap();
    std::fs::create_dir_all(backups.join(&their_id).join("snapshots")).unwrap();
    std::fs::write(backups.join(&my_id).join("vault").join("tasks").join("a.md"), "mine").unwrap();
    std::fs::write(backups.join(&their_id).join("snapshots").join("2026-09-10.zip"), "theirs").unwrap();

    let app_root = root.join("appdata");
    knowlu::profiles::register(&app_root, "Fall 2026", &mine).expect("register mine");
    knowlu::profiles::register(&app_root, "Someone else", &theirs).expect("register theirs");
    let data_dir = knowlu::profiles::profile_dir(&app_root, &my_id);
    std::fs::write(data_dir.join("settings.json"), "{}").unwrap();
    let their_data = knowlu::profiles::profile_dir(&app_root, &their_id);

    let session = format!("knowlu/test-delete-{}-{}/session", std::process::id(), line!());
    let my_login = knowlu::credentials::target_for(&my_id, "zybooks");
    let their_login = knowlu::credentials::target_for(&their_id, "zybooks");
    let _cleanup = Cleanup(vec![session.clone(), my_login.clone(), their_login.clone()]);
    knowlu::credentials::write(&session, "acc-1", "{}").expect("a session to delete");
    knowlu::credentials::write(&my_login, "me@example.invalid", "pw").expect("my coursework login");
    knowlu::credentials::write(&their_login, "them@example.invalid", "pw").expect("their coursework login");

    knowlu::account::delete_local_data(&mine, &data_dir, Some(&backups), &my_id, Some(&app_root), &session);

    assert!(!mine.exists(), "the vault is gone");
    assert!(!backups.join(&my_id).exists(), "my snapshots are gone");
    assert!(!data_dir.exists(), "my app data is gone");
    assert!(!knowlu::credentials::exists(&session), "my session is gone");
    assert!(!knowlu::credentials::exists(&my_login), "my coursework login is gone");
    let left = knowlu::profiles::load(&app_root).expect("the registry still reads");
    assert_eq!(left.len(), 1, "only my row left the registry: {left:?}");
    assert_eq!(left[0].id, their_id);

    assert!(backups.is_dir(), "the backups ROOT is shared and must survive");
    assert!(backups.join(&their_id).join("snapshots").join("2026-09-10.zip").is_file(), "another profile's snapshots were destroyed");
    assert!(theirs.join("tasks").join("b.md").is_file(), "another profile's vault was destroyed");
    assert!(their_data.is_dir(), "another profile's app data was destroyed");
    assert!(knowlu::credentials::exists(&their_login), "another profile's coursework login was destroyed");
    assert!(root.is_dir(), "nothing above the folders named was touched");
    let _ = std::fs::remove_dir_all(&root);
}
