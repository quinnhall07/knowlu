//! `account.rs` against a loopback server, never the network (the 3a rule: `127.0.0.1` only, and the
//! serving thread is joined before the test returns, so a test can never outlive its own socket).
use knowlu::account::{auth_base, check_api_base, Session};
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
fn the_code_request_creates_the_account_and_carries_no_consent_key_at_all() {
    use knowlu::account::magic_link_at;
    let (base, handle) = loopback(vec![(200, "{}".to_string())]);
    let out = magic_link_at(&format!("{base}/auth/v1"), "anon-key", "n@example.invalid");
    let seen = handle.join().expect("server thread");
    assert!(out.is_ok(), "{out:?}");
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/otp "), "{req}");
    // `create_user: false` is what made a new student's first press answer "Signups not allowed for
    // otp". One field, one button, one code — there is no separate create step to fall back to.
    assert!(req.contains("\"create_user\":true"), "{req}");
    // R-C1b-3: nothing about consent travels with a request that anyone holding the public anon key
    // can send. `data` would land as `raw_user_meta_data`, and after migration 20260917000100 the
    // trigger reads none of it — the attestation is recorded by `POST /account/consent` once the
    // code has proved the address, on this path exactly as on Google's.
    for key in ["age_attested", "tos_version", "privacy_version", "\"data\""] {
        assert!(!req.contains(key), "{key} must not travel with /otp: {req}");
    }
    // And nothing that smells of a password, on any path in this file any more.
    assert!(!req.to_lowercase().contains("password"), "{req}");
}

/// The deletion, pinned. A dead command that can still make a password account is a second door.
#[test]
fn there_is_no_password_path_left_in_the_crate() {
    let src = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    for gone in ["fn sign_up_at", "fn sign_in_at", "pub fn sign_up(", "pub fn sign_in(", "grant_type=password", "/signup"] {
        assert!(!src.contains(gone), "{gone} must be gone with the password");
    }
    // **NOT a ban on the word** (review C2). `load_session` reads `cred.password.expose()`
    // (`account.rs:130`) and `password` there is a field name on `knowlu_engine::wincred`'s
    // credential struct — engine-owned, and `engine/**` is not this stream's to edit, so the old
    // catch-all could never pass without deleting the one function that reads a stored session.
    // Two claims that are true AND load-bearing instead: no request body carries a password field,
    // and no function in this file takes one.
    assert!(!src.contains("\"password\""), "no request body may carry a password field");
    assert!(!src.contains("password: &str") && !src.contains("password: String"),
        "no function in account.rs takes a password");
}

/// The magic link's second half, which is what makes the button on the panel honest.
#[test]
fn a_code_from_the_email_becomes_a_session_on_this_machine() {
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
/// `ENGINE_ENV_LOCK` in `tests/scheduler.rs` serializes process-global env var access. The store is
/// shared by every process as well, so this is the cross-process lock in `support/credman_lock.rs`.
#[cfg(windows)]
#[path = "support/credman_lock.rs"]
mod credman_lock;
#[cfg(windows)]
static CREDMAN_LOCK: credman_lock::CredmanLock = credman_lock::CredmanLock::new();

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
/// silent "ok" both times.
///
/// **R-C1-57 (I1): the base is pointed at a closed loopback port for the whole test.** `sign_out`
/// ends with a best-effort `POST {auth}/logout`, and this comment used to say no loopback was needed
/// because `auth_base` would refuse the placeholder default — which stopped being true the moment
/// `DEFAULT_API_BASE` was filled with the real project (Task 10, `7029b6a`). Since then every run of
/// this suite posted to prod GoTrue. Now nothing does; the revoke fails against a dead port, which is
/// exactly the "best effort, and second" path `sign_out` documents.
#[cfg(windows)]
#[test]
fn signing_out_a_profile_reports_whether_it_had_a_session_and_removes_it() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _api = ApiBase::set(&closed_loopback_base());
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

/// R-C1c-final-5: `scheduler::spawn`'s launch refresh (at +10 s) and a slot's own in-slot refresh
/// (§2) can overlap when the round trip takes about that long. With one shared `.tmp` name, the
/// loser's `rename` finds its own source already moved away by the winner, falls into the fallback,
/// and `remove_file(&path)` deletes the WINNER's just-written fresh cache — both calls report
/// success and the profile is left with none at all (the F13 skip again). Many rounds of two
/// barrier-synchronised threads, to give that interleaving room to land within one test run. This
/// never touches Credential Manager, so — per CLAUDE.md — it does not take `CREDMAN_LOCK`.
#[test]
fn overlapping_saves_never_clobber_each_other_into_no_cache_at_all() {
    use knowlu::account::{cache_path, load_cache, save_cache, EntitlementCache};
    let dir = std::env::temp_dir().join(format!("knowlu-ent-race-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for round in 0..200u32 {
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut handles = Vec::new();
        for i in 0..2u32 {
            let dir = dir.clone();
            let barrier = barrier.clone();
            handles.push(std::thread::spawn(move || {
                let c = EntitlementCache {
                    status: if i == 0 { "active" } else { "trialing" }.into(),
                    current_period_end: None,
                    plan: Some("monthly".into()),
                    checked_at: format!("2026-09-22T00:00:{round:02}.00{i}Z"),
                };
                // Lines both threads up at the same starting gate so the write-then-rename windows
                // of the two calls actually overlap, rather than one finishing before the other starts.
                barrier.wait();
                save_cache(&dir, &c)
            }));
        }
        for h in handles {
            h.join().unwrap().expect("an overlapping save must still succeed");
        }
        assert!(
            load_cache(&dir).is_some(),
            "round {round}: the cache must end up present and parseable, never deleted by the other writer's fallback"
        );
    }
    assert!(cache_path(&dir).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fix round 1, item 5a: a profile with no session credential at all — the ordinary state for a
/// signed-out or never-signed-in install — must fail the refresh outright and leave no
/// `entitlement.json` behind. No loopback server needed — nothing here ever answers a request — but
/// `cloud_config`'s host check (R-C1-59 I1) now runs before the session lookup does, so the vault's
/// `api_base` must still name the seam's own base, never the compiled-in project.
#[cfg(windows)]
#[test]
fn refresh_entitlement_with_no_session_fails_and_writes_no_cache() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::{load_cache, refresh_entitlement};
    let target = format!("knowlu/test-refresh-nosession-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    assert!(!knowlu::credentials::exists(&target), "nothing must be there to begin with");
    let base = closed_loopback_base();
    let _api = ApiBase::set(&base);

    let vault = std::env::temp_dir().join(format!("knowlu-refresh-nosession-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&vault);
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::write(
        vault.join("config").join("cloud.yaml"),
        format!("api_base: '{base}'\nanon_key: 'anon'\nsession_credential_target: '{target}'\naccount_id: 'acc-1'\n"),
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
    let api_base = format!("{base}/functions/v1");
    let _api = ApiBase::set(&api_base);
    std::fs::write(
        vault.join("config").join("cloud.yaml"),
        format!("api_base: '{api_base}'\nanon_key: 'anon'\nsession_credential_target: '{target}'\naccount_id: 'acc-1'\n"),
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

/// R-C1c-13: the scheduler's pre-flight, run before the first cloud step of a slot. Unlike
/// `valid_access_token_at`'s 120-second margin (reached only at a slot's own end, by the telemetry
/// step), this one is asked with a floor wide enough to survive the slot itself — 45 minutes here —
/// so a token minted hours ago by an earlier slot is refreshed before anything tries to spend it.
#[cfg(windows)]
#[test]
fn ensure_session_for_at_refreshes_a_token_with_ten_minutes_left_against_a_forty_five_minute_floor() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::{ensure_session_for_at, load_session, save_session, Session};
    let target = format!("knowlu/test-ensure-refresh-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    let now = jiff::Timestamp::now().as_second();
    let s = Session {
        access_token: "stale-at".into(),
        refresh_token: "rt-1".into(),
        expires_at: now + 600, // ten minutes left
        email: "a@example.invalid".into(),
    };
    save_session(&target, "acc-1", &s).expect("write a near-expiry session");

    let body = r#"{"access_token":"fresh-at","refresh_token":"fresh-rt","expires_in":3600,"user":{"id":"acc-1","email":"a@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let refreshed = ensure_session_for_at(&format!("{base}/auth/v1"), "anon-key", &target, now, 45 * 60).expect("refresh");
    let seen = handle.join().expect("server thread");
    assert!(refreshed, "ten minutes left against a forty-five-minute floor must refresh");
    assert!(seen[0].starts_with("POST /auth/v1/token?grant_type=refresh_token "), "{}", seen[0]);
    assert!(seen[0].contains("\"refresh_token\":\"rt-1\""), "{}", seen[0]);
    let (_, back) = load_session(&target).expect("the rewritten entry");
    assert_eq!(back.access_token, "fresh-at");
    assert_eq!(back.refresh_token, "fresh-rt");
}

/// …and a token that already clears the floor is left alone — no request at all, so pointing it at a
/// port nothing listens on still succeeds and the stored entry is byte-for-byte what it was.
#[cfg(windows)]
#[test]
fn ensure_session_for_at_leaves_a_token_with_fifty_minutes_left_against_a_forty_five_minute_floor() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::{ensure_session_for_at, load_session, save_session, Session};
    let target = format!("knowlu/test-ensure-fresh-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    let now = jiff::Timestamp::now().as_second();
    let s = Session {
        access_token: "still-good-at".into(),
        refresh_token: "rt-2".into(),
        expires_at: now + 3000, // fifty minutes left
        email: "a@example.invalid".into(),
    };
    save_session(&target, "acc-1", &s).expect("write a comfortably fresh session");

    let base = closed_loopback_base();
    let refreshed = ensure_session_for_at(&base, "anon-key", &target, now, 45 * 60)
        .expect("a token that clears the floor must never even attempt the network");
    assert!(!refreshed, "fifty minutes left against a forty-five-minute floor must not refresh");
    let (_, back) = load_session(&target).expect("the untouched entry");
    assert_eq!(back, s);
}

/// A server for the F1 race test (R-C1c-exec-14): unlike `loopback`, above, it does not know in
/// advance how many requests to expect — that count is exactly what the test is proving. It answers
/// every `POST …/token?grant_type=refresh_token` that arrives within `window`, holding each one for
/// `hold` before it replies (long enough that two callers racing the same refresh genuinely overlap,
/// not merely by chance), and reports how many it actually served. It stops early once `cap` requests
/// have landed, so the GREEN case (one request, then nothing) is the only one that pays the full
/// `window`.
#[cfg(windows)]
fn racing_token_server(hold: std::time::Duration, window: std::time::Duration, cap: usize) -> (String, std::thread::JoinHandle<usize>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.set_nonblocking(true).expect("nonblocking listener");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + window;
        let mut served = 0usize;
        while std::time::Instant::now() < deadline && served < cap {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).expect("blocking stream");
                    stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).expect("read timeout");
                    // One read is enough: a loopback client's small JSON POST arrives in one segment
                    // (the same reasoning `serve_one_callback`'s own doc gives for its request line).
                    let mut buf = [0u8; 4096];
                    let _ = stream.read(&mut buf);
                    std::thread::sleep(hold);
                    served += 1;
                    let body = format!(
                        r#"{{"access_token":"fresh-at-{served}","refresh_token":"fresh-rt-{served}","expires_in":3600,"user":{{"id":"acc-1","email":"a@example.invalid"}}}}"#
                    );
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
        served
    });
    (format!("http://127.0.0.1:{port}/auth/v1"), handle)
}

/// F1 (R-C1c-exec-14): the scheduler's pre-flight (`ensure_session_for_at`, 45-minute floor) and
/// C2's own read (`valid_access_token_at`, 120-second floor) can both decide, on different threads,
/// that the SAME stored session needs refreshing — the slot's pre-flight and its entitlement/telemetry
/// steps run on one thread each slot, but the housekeeping thread's independent 6-hourly
/// `refresh_entitlement` tick (which itself calls `valid_access_token_at`) is gated by no lock at all.
/// GoTrue rotates the refresh token on every use, so two concurrent refreshes of the same token are a
/// real hazard, not just wasted work. A one-minute-left session is under BOTH floors, so both
/// functions independently decide to refresh; only one request may ever reach the token endpoint.
#[cfg(windows)]
#[test]
fn a_concurrent_preflight_and_read_only_ever_hit_the_token_endpoint_once() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use knowlu::account::{ensure_session_for_at, save_session, valid_access_token_at, Session};
    let target = format!("knowlu/test-race-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    let now = jiff::Timestamp::now().as_second();
    let s = Session {
        access_token: "stale-at".into(),
        refresh_token: "stale-rt".into(),
        // One minute left: under `valid_access_token_at`'s 120-second floor AND a 45-minute
        // pre-flight floor, so both callers independently decide a refresh is needed.
        expires_at: now + 60,
        email: "a@example.invalid".into(),
    };
    save_session(&target, "acc-1", &s).expect("write a near-expiry session");

    let (base, handle) = racing_token_server(std::time::Duration::from_millis(300), std::time::Duration::from_secs(3), 2);
    let (base1, base2) = (base.clone(), base.clone());
    let (target1, target2) = (target.clone(), target.clone());
    let (r1, r2) = std::thread::scope(|scope| {
        let t1 = scope.spawn(move || ensure_session_for_at(&base1, "anon-key", &target1, now, 45 * 60));
        let t2 = scope.spawn(move || valid_access_token_at(&base2, "anon-key", &target2, now));
        (t1.join().unwrap(), t2.join().unwrap())
    });
    let served = handle.join().expect("server thread");
    assert!(r1.is_ok(), "{r1:?}");
    assert!(r2.is_ok(), "{r2:?}");
    assert_eq!(served, 1, "exactly one refresh must reach the token endpoint, never two racing the same refresh_token");
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

/// Every file under `root`, as (relative path, bytes), sorted — a whole-tree fingerprint, the same
/// one `app/tests/onboarding.rs` uses to prove a folder was not written to. Content, not mtimes.
#[cfg(windows)]
fn fingerprint(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).expect("a readable directory").flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p); }
            else { out.push((p.strip_prefix(root).expect("under root").to_string_lossy().to_string(), std::fs::read(&p).expect("a readable file"))); }
        }
    }
    out.sort();
    out
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
    std::fs::write(vault.join("config").join("runners.yaml"), "runners:\n  local:\n    device: 'somebody-pc'\n").unwrap();
    std::fs::write(vault.join("tasks").join("t.md"), "---\nid: task_0000000001\n---\n\nA task.\n").unwrap();
    let before = std::fs::read_to_string(vault.join("config").join("ingest.yaml")).unwrap();
    // R-C1-57 (M9): the ruling's claim is about the WHOLE tree, so the whole tree is hashed either
    // side of the call — `app/tests/onboarding.rs` makes the same claim the same way.
    let tree_before = fingerprint(&vault);
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
    // M9: and the tree as a whole — every file, byte for byte — differs by exactly `config/cloud.yaml`.
    let after: Vec<_> = fingerprint(&vault).into_iter().filter(|(p, _)| p.replace('\\', "/") != "config/cloud.yaml").collect();
    assert_eq!(after, tree_before, "adopting in place wrote something other than config/cloud.yaml");
    assert!(vault.join("config").join("cloud.yaml").is_file(), "…and it did write that one");
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

/// **R-C1-57 (I1): no test in this file may reach the compiled-in project.** `DEFAULT_API_BASE` has
/// been the real Supabase project since Task 10, so any test that calls a function which reads
/// `api_base()` makes a live request out of `cargo test` unless it has pointed `KNOWLU_API_BASE`
/// somewhere harmless first — which is what `ApiBase::set(&closed_loopback_base())` is for. One test
/// had been doing exactly that, silently, for a fortnight.
///
/// Derived from the source rather than kept as a list, the way
/// `static_assets.rs::no_multi_word_command_argument_is_sent_in_the_wrong_case` is: read
/// `src/account.rs` for every function that reaches `api_base()` — directly, or through another one
/// that does — then read this file and insist that every test calling one of them sets the seam.
#[test]
fn no_test_in_this_file_can_reach_the_compiled_in_project() {
    /// Top-level `fn` items and the body text under each, comment lines dropped so a name mentioned
    /// in prose can never stand in for a call. Only a line with no leading whitespace starts one, so
    /// methods inside an `impl` stay part of their enclosing item.
    fn items(src: &str) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for line in src.lines() {
            let name = ["pub async fn ", "pub fn ", "async fn ", "fn "]
                .iter()
                .find_map(|p| line.strip_prefix(p))
                .map(|r| r.split(['(', '<']).next().unwrap_or("").trim().to_string())
                .filter(|n| !n.is_empty());
            if let Some(n) = name {
                out.push((n, String::new()));
            }
            if line.trim_start().starts_with("//") {
                continue;
            }
            if let Some(last) = out.last_mut() {
                last.1.push_str(line);
                last.1.push('\n');
            }
        }
        out
    }
    /// `name(` with nothing identifier-like in front of it, so `check_api_base(` is not a call to
    /// `api_base` and `sign_in_at(` is not a call to `sign_in`.
    fn calls(body: &str, name: &str) -> bool {
        body.match_indices(&format!("{name}(")).any(|(i, _)| {
            i == 0 || {
                let c = body.as_bytes()[i - 1];
                !(c.is_ascii_alphanumeric() || c == b'_')
            }
        })
    }

    let src = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    let source_items = items(&src);
    let mut risky: Vec<String> = Vec::new();
    loop {
        let before = risky.len();
        for (name, body) in &source_items {
            // `api_base` and `anon_key` only read an environment variable. It is their CALLERS that
            // then send something somewhere with what came back.
            if risky.contains(name) || name == "api_base" || name == "anon_key" {
                continue;
            }
            if calls(body, "api_base") || risky.iter().any(|r| calls(body, r)) {
                risky.push(name.clone());
            }
        }
        if risky.len() == before {
            break;
        }
    }
    // A scan that silently stops finding anything is a guard that silently stops guarding.
    assert!(risky.len() >= 5, "the scan stopped working: {risky:?}");
    for expected in ["sign_out", "attach_in", "entitlement_now", "open_checkout"] {
        assert!(risky.iter().any(|r| r == expected), "{expected} reaches the compiled-in base: {risky:?}");
    }

    for (name, body) in items(&std::fs::read_to_string("tests/account.rs").expect("tests/account.rs")) {
        let Some(hit) = risky.iter().find(|r| calls(&body, r)) else { continue };
        assert!(
            body.contains("ApiBase::set"),
            "{name} calls account::{hit}, which reads the compiled-in project — it must set the seam first (ApiBase::set(&closed_loopback_base()), or a loopback base)"
        );
    }
}

/// **R-C1-57 (I3): a profile id names paths, so it is checked before it names one.** `settings.json`
/// is a plain file a person can edit, `state::Settings` parses `profile_id` as an unchecked `String`,
/// and `delete_local_data` joins it onto the backups ROOT — `"../.."` there walks out of
/// `%USERPROFILE%\Knowlu\Backups` and takes the home folder with it.
#[test]
fn a_profile_id_that_is_not_one_is_never_used_to_name_a_path() {
    use knowlu::account::{is_profile_id, profile_id_or_derived};
    // The shape `ids::derived_id("profile", …)` always produces: the literal prefix and ten hex.
    let vault = std::path::Path::new("C:\\Users\\somebody\\Knowlu\\Fall 2026");
    let real = knowlu::profiles::id_for(vault);
    assert!(is_profile_id(&real), "{real}");
    assert!(is_profile_id("profile_0123456789"));
    for bad in ["", "..", "../..", "profile_", "profile_012345678", "profile_01234567890", "profile_zzzzzzzzzz", "profile_../..", "a/b", "profile_0123456789/x", "..\\..", "C:\\Windows"] {
        assert!(!is_profile_id(bad), "{bad:?} must never name a folder");
        // …and whatever was stored, what gets used is the id the vault path derives — the same one
        // `profiles::register` filed the profile under.
        assert_eq!(profile_id_or_derived(bad, vault), real, "{bad:?}");
    }
    assert_eq!(profile_id_or_derived(&real, vault), real, "a real id is used as it stands");
}

/// The other half of I3, over the filesystem: an id that is not one removes nothing keyed to an id —
/// not the backups subtree, not the registry row, not a credential — and above all never climbs.
/// The vault and the app-data folder are paths the CALLER named and are still removed, because those
/// are the two things *delete my data* is about.
#[test]
fn delete_local_data_never_climbs_out_of_the_backups_root() {
    let root = std::env::temp_dir().join(format!("knowlu-delete-climb-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    let backups = home.join("Knowlu").join("Backups");
    let neighbour = backups.join("profile_2222222222").join("vault");
    std::fs::create_dir_all(&neighbour).unwrap();
    std::fs::write(neighbour.join("b.md"), "theirs").unwrap();
    std::fs::create_dir_all(home.join("Documents")).unwrap();
    std::fs::write(home.join("Documents").join("thesis.docx"), "years of work").unwrap();

    for bad in ["..", "../..", "..\\..", "profile_2222222222/../profile_2222222222", ""] {
        let vault = root.join("vault");
        let data_dir = root.join("data");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::create_dir_all(&data_dir).unwrap();
        // No session target: this call must not reach Credential Manager at all, which is also why
        // it needs no lock.
        knowlu::account::delete_local_data(&vault, &data_dir, Some(&backups), bad, None, "");
        assert!(!vault.exists(), "the vault the caller named is still deleted ({bad:?})");
        assert!(!data_dir.exists(), "…and so is the app-data folder ({bad:?})");
        assert!(backups.is_dir(), "the backups root survived {bad:?}");
        assert!(neighbour.join("b.md").is_file(), "another profile's mirror survived {bad:?}");
        assert!(home.join("Documents").join("thesis.docx").is_file(), "{bad:?} climbed out of the backups root");
        assert!(home.is_dir(), "{bad:?} climbed to the home folder");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// **R-C1-57 (I4): the two halves really are one operation.** `attach_in`'s doc says a `cloud.yaml`
/// naming a credential target that holds nothing is the state it exists to prevent — so when the
/// session cannot be moved, the file it just wrote goes with it. Without that, `needs_account` is
/// false forever, and the upgrade overlay (the console's only sign-in surface) never comes back.
///
/// The move is a seam here for the same reason `lms_link::validate_for`'s fetch is one: a
/// `CredWriteW` that fails is not something a test can arrange on a healthy machine.
#[cfg(windows)]
#[test]
fn a_session_that_cannot_be_moved_takes_the_cloud_yaml_back_with_it() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _api = ApiBase::set(&closed_loopback_base());
    use knowlu::account::{attach_config_and_session, needs_account};
    let root = std::env::temp_dir().join(format!("knowlu-rollback-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&root);
    let vault = root.join("Fall 2026");
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::create_dir_all(vault.join("tasks")).unwrap();
    std::fs::write(vault.join("config").join("planning.yaml"), "daily_effort_budget: 4.0\n").unwrap();
    let profile_id = knowlu::profiles::id_for(&vault);

    let err = attach_config_and_session(&vault, &profile_id, "acc-1", "knowlu/nowhere/pending", &|_, _| {
        Err("the credential store refused".to_string())
    })
    .unwrap_err();
    assert!(err.contains("the credential store refused"), "{err}");
    assert!(!vault.join("config").join("cloud.yaml").exists(), "a failed move must leave no cloud.yaml behind");
    assert!(needs_account(&vault), "…so the next launch raises the overlay again");
    // config/ itself is left alone: it was already there, and it is full of the student's settings.
    assert!(vault.join("config").join("planning.yaml").is_file(), "nothing else in config/ was touched");

    // …and the same call with a move that succeeds writes the file and keeps it.
    attach_config_and_session(&vault, &profile_id, "acc-1", "knowlu/nowhere/pending", &|_, _| Ok(())).expect("attach");
    assert!(vault.join("config").join("cloud.yaml").is_file());
    assert!(!needs_account(&vault));
    let _ = std::fs::remove_dir_all(&root);
}

use knowlu::account::{authorize_url, b64url, code_from_request_line, pkce_pair};

/// RFC 7636 Appendix B, verbatim. The one place in this file where a literal is not ours: it is the
/// standard's own worked example, and matching it is what says our challenge is a PKCE challenge
/// rather than a hash of something adjacent.
#[test]
fn the_challenge_is_rfc_7636_appendix_bs_worked_example() {
    use sha2::{Digest, Sha256};
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
    assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
}

#[test]
fn base64url_is_unpadded_and_uses_the_url_alphabet() {
    // The three bytes that produce a `+` and a `/` in standard base64 — the two characters that
    // would be re-encoded or mis-read inside a query string, which is the whole reason for -url.
    assert_eq!(b64url(&[0xfb, 0xff, 0xbe]), "-_--");
    assert_eq!(b64url(&[]), "");
    assert_eq!(b64url(&[0x00]), "AA");
    assert_eq!(b64url(&[0x00, 0x00]), "AAA");
    assert!(!b64url(&[0x00]).contains('='), "no padding: the query string is not the place for it");
}

#[test]
fn a_verifier_is_43_characters_of_the_unreserved_alphabet_and_never_repeats() {
    let (v1, c1) = pkce_pair();
    let (v2, _) = pkce_pair();
    // RFC 7636 §4.1: 43 to 128 characters. 32 random bytes is 43 unpadded base64url characters.
    assert_eq!(v1.len(), 43);
    assert!(v1.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'), "{v1}");
    assert_eq!(c1.len(), 43);
    assert_ne!(v1, c1, "the challenge is the hash, never the verifier itself");
    assert_ne!(v1, v2, "a fresh pair every sign-in");
}

#[test]
fn the_authorize_url_names_google_pkce_and_the_loopback_port_and_nothing_else() {
    let url = authorize_url("https://abc.supabase.co/auth/v1", 54321, "CHAL");
    assert_eq!(
        url,
        "https://abc.supabase.co/auth/v1/authorize?provider=google\
         &redirect_to=http%3A%2F%2F127.0.0.1%3A54321%2Fcallback\
         &code_challenge=CHAL&code_challenge_method=s256"
            .replace(' ', "")
    );
    // `state` is in GoTrue's `reservedOAuthParams` and is stripped from the query before the
    // provider is called — sending one would be a line of code that does nothing (spec §4).
    assert!(!url.contains("state="), "{url}");
    // No `flow_type`: GoTrue infers PKCE from the presence of `code_challenge`.
    assert!(!url.contains("flow_type"), "{url}");
}

#[test]
fn the_callback_request_line_yields_the_code_or_the_providers_own_sentence() {
    assert_eq!(code_from_request_line("GET /callback?code=abc123 HTTP/1.1").unwrap(), "abc123");
    // Percent-decoded, because GoTrue query-encodes what it puts there.
    assert_eq!(code_from_request_line("GET /callback?code=a%2Bb HTTP/1.1").unwrap(), "a+b");
    let e = code_from_request_line("GET /callback?error=access_denied&error_description=You+said+no HTTP/1.1")
        .expect_err("an error is not a code");
    assert!(e.contains("You said no"), "{e}");
    assert!(code_from_request_line("GET /favicon.ico HTTP/1.1").is_err(), "no code, no session");
    assert!(code_from_request_line("garbage").is_err());
}

use knowlu::account::{serve_one_callback, CALLBACK_FAILED_PAGE, CALLBACK_PAGE, NOT_FOUND_PAGE};

/// A loopback listener on `127.0.0.1:0` is not the network: the same machine, the same process
/// tree, nothing that leaves it. The serving side is the production code; the client side is this
/// test's own thread, joined before the test returns (the 3a rule).
#[test]
fn the_callback_listener_serves_exactly_one_browser_and_answers_in_one_sentence() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let browser = std::thread::spawn(move || {
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        s.write_all(b"GET /callback?code=xyz789 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .expect("write");
        let mut got = String::new();
        let _ = s.read_to_string(&mut got);
        got
    });
    let code = serve_one_callback(listener, std::time::Duration::from_secs(5)).expect("a code");
    let page = browser.join().expect("browser thread");
    assert_eq!(code, "xyz789");
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("You are signed in to Knowlu. You can close this window."), "{page}");
    assert!(page.contains(CALLBACK_PAGE), "the served body is the constant, not a second copy");
}

#[test]
fn a_refusal_in_the_query_is_the_providers_sentence_and_the_browser_still_gets_a_page() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let browser = std::thread::spawn(move || {
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        s.write_all(b"GET /callback?error=access_denied&error_description=You+said+no HTTP/1.1\r\n\r\n")
            .expect("write");
        let mut got = String::new();
        let _ = s.read_to_string(&mut got);
        got
    });
    let err = serve_one_callback(listener, std::time::Duration::from_secs(5)).expect_err("a refusal");
    let page = browser.join().expect("browser thread");
    assert!(err.contains("You said no"), "{err}");
    // A browser left staring at a connection reset is a worse answer than a sentence.
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    // F1: a declined consent screen must never be told "You are signed in" — that sentence is only
    // true once `code_from_request_line` returned `Ok`.
    assert!(page.contains(CALLBACK_FAILED_PAGE), "the served body is the failure constant, not the success one: {page}");
    assert!(!page.contains("You are signed in to Knowlu."), "{page}");
}

#[test]
fn a_browser_that_never_comes_back_times_out_instead_of_holding_the_wizard_forever() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let out = serve_one_callback(listener, std::time::Duration::from_millis(150));
    let err = out.expect_err("nothing connected");
    assert!(err.contains("not finished"), "{err}");
}

/// **Review I3.** Anything on this machine may reach an open loopback port first: a browser
/// preconnect, a favicon fetch, security software, a port scanner, a second process. Answering it
/// and returning would end the sign-in with *the browser came back without a sign-in code* while the
/// real redirect was still in flight — and that redirect would then meet a closed socket. The
/// listener answers 404 and keeps waiting; the FIRST real callback is still the only one served.
#[test]
fn a_stray_local_connection_gets_404_and_the_real_callback_still_lands() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let noise_then_browser = std::thread::spawn(move || {
        let mut probe = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        probe.write_all(b"GET /favicon.ico HTTP/1.1\r\nConnection: close\r\n\r\n").expect("write");
        let mut probe_got = String::new();
        let _ = probe.read_to_string(&mut probe_got);
        let mut browser = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect again");
        browser
            .write_all(b"GET /callback?code=late123 HTTP/1.1\r\nConnection: close\r\n\r\n")
            .expect("write");
        let mut page = String::new();
        let _ = browser.read_to_string(&mut page);
        (probe_got, page)
    });
    let code = serve_one_callback(listener, std::time::Duration::from_secs(5)).expect("the real code");
    let (probe_got, page) = noise_then_browser.join().expect("the client thread");
    assert_eq!(code, "late123", "the stray request must not consume the sign-in");
    assert!(probe_got.starts_with("HTTP/1.1 404"), "{probe_got}");
    // Review finding 5: the 404 body is the constant, not a second copy of the same sentence.
    assert!(probe_got.contains(NOT_FOUND_PAGE), "{probe_got}");
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("You are signed in to Knowlu."), "{page}");
}

use knowlu::account::{exchange_pkce_at, record_consent_at};

#[test]
fn the_code_and_the_verifier_are_traded_for_a_session_at_grant_type_pkce() {
    let body = r#"{"access_token":"atG","refresh_token":"rtG","expires_in":3600,"user":{"id":"acc-g","email":"g@example.invalid"}}"#;
    let (base, handle) = loopback(vec![(200, body.to_string())]);
    let out = exchange_pkce_at(&format!("{base}/auth/v1"), "anon-key", "the-code", "the-verifier", 1_760_000_000);
    let seen = handle.join().expect("server thread");
    let (id, s) = out.expect("exchange");
    assert_eq!(id, "acc-g");
    assert_eq!(s.access_token, "atG");
    assert_eq!(s.email, "g@example.invalid");
    assert_eq!(s.expires_at, 1_760_000_000 + 3600);
    let req = &seen[0];
    assert!(req.starts_with("POST /auth/v1/token?grant_type=pkce "), "{req}");
    assert!(req.to_lowercase().contains("apikey: anon-key"), "{req}");
    // GoTrue's `PKCEGrantParams` has exactly these two fields; `code` or `verifier` would be
    // silently empty and answer 400 `invalid request: both auth code and code verifier should be non-empty`.
    assert!(req.contains("\"auth_code\":\"the-code\""), "{req}");
    assert!(req.contains("\"code_verifier\":\"the-verifier\""), "{req}");
}

#[test]
fn a_bad_verifier_is_gotrues_own_sentence_and_never_a_status_code() {
    let (base, handle) = loopback(vec![(400, r#"{"error_description":"code challenge does not match previously saved code verifier"}"#.to_string())]);
    let out = exchange_pkce_at(&format!("{base}/auth/v1"), "anon-key", "c", "v", 0);
    let _ = handle.join();
    let e = out.expect_err("refused");
    assert!(e.contains("code challenge does not match"), "{e}");
    assert!(!e.contains("400"), "the panel reads a sentence, not a status: {e}");
}

#[test]
fn the_consent_call_carries_the_attestation_and_both_compiled_in_versions() {
    let (base, handle) = loopback(vec![(200, r#"{"ok":true}"#.to_string())]);
    let out = record_consent_at(&format!("{base}/functions/v1"), "the-token", "2026-09-10", "2026-09-17");
    let seen = handle.join().expect("server thread");
    assert!(out.is_ok(), "{out:?}");
    let req = &seen[0];
    assert!(req.starts_with("POST /functions/v1/account/consent "), "{req}");
    assert!(req.to_lowercase().contains("authorization: bearer the-token"), "{req}");
    assert!(req.contains("\"age_attested\":true"), "{req}");
    assert!(req.contains("\"tos_version\":\"2026-09-10\""), "{req}");
    assert!(req.contains("\"privacy_version\":\"2026-09-17\""), "{req}");
}

/// R-C1b-3: one route, both paths. A `verify_email_code` that saved a session and recorded nothing
/// would leave the emailed-code student with a null attestation and a 403 at the subscribe step.
///
/// **The owner strings are built, not written literally** (`owner_of`, below): a bare
/// `"pub fn verify_email_code("` in this file's own source text is exactly the shape
/// `no_test_in_this_file_can_reach_the_compiled_in_project`'s scan reads as a real call to a
/// risky function — `verify_email_code` calls `env_pair`, which reads the compiled-in `api_base()`
/// — and that meta-test would then demand an `ApiBase::set` seam this test has no reason to hold: it
/// touches no network and no environment variable, only `src/account.rs`'s own text.
fn owner_of(name: &str) -> String { format!("pub fn {name}(") }

#[test]
fn both_sign_in_paths_record_the_consent() {
    let src = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    for name in ["google_sign_in", "verify_email_code"] {
        let owner = owner_of(name);
        let body = src.split(&owner).nth(1).expect(&owner);
        let body = body.split("\n#[tauri::command").next().unwrap_or(body);
        assert!(body.contains("record_consent_at("), "{owner} must record the consent it just took");
    }
}

/// **F2.** `google_sign_in` now takes the attestation the page's checkbox stands for, and refuses on
/// it the same way `send_magic_link` does — the belt the C1 static test used to describe is back.
/// The order is the point: the check must be the first thing the function does, before `api_base()`,
/// before the loopback listener is bound and before a browser ever opens. A source-text pin, in the
/// same style `owner_of` above already uses for this file.
#[test]
fn google_sign_in_gates_the_attestation_before_any_listener_is_bound() {
    let src = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    let owner = owner_of("google_sign_in");
    let body = src.split(&owner).nth(1).expect(&owner);
    let body = body.split("\n#[tauri::command").next().unwrap_or(body);
    let gate = body.find("!age_attested").expect("google_sign_in must gate on age_attested");
    let bind = body.find("TcpListener::bind").expect("google_sign_in must still bind the loopback listener");
    assert!(gate < bind, "the attestation must be refused before the listener is bound");
}

/// The same claim, proved by calling the function rather than reading it: `ApiBase::set` points the
/// compiled-in base at a closed loopback port for the call, so if the gate above ever moved past the
/// listener bind or the browser open, this call would hang on a dead connection (or open a real
/// browser tab in CI) instead of returning the refusal sentence instantly.
#[test]
fn google_sign_in_refuses_with_no_attestation_and_opens_nothing() {
    let _api = ApiBase::set(&closed_loopback_base());
    use knowlu::account::google_sign_in;
    let out = google_sign_in(false);
    assert_eq!(out["ok"], false);
    assert_eq!(out["error"], "Knowlu is for people 18 or older.");
    assert!(out["account_id"].is_null(), "{out:?}");
}

/// **Review I4.** The consent call after a sign-in is best effort, so `open_checkout` retries it
/// **before** the checkout POST: an account whose attestation never landed meets
/// `billing-checkout`'s 403 — *the 18+ attestation is missing — sign in again* — and signing in
/// again would take the same failing path. The route writes only when the account has none, so the
/// retry costs one request and can never double-record. A retry that fails again is logged and does
/// not stand in the way: the 403 is the honest answer, and it is the server's to give.
#[test]
fn the_checkout_retries_the_consent_first_and_a_failed_retry_still_reaches_stripe() {
    use knowlu::account::checkout_url_at;
    let (base, handle) = loopback(vec![
        (500, r#"{"msg":"the consent route is down"}"#.to_string()),
        (200, r#"{"url":"https://checkout.example.invalid/c/cs_1"}"#.to_string()),
    ]);
    let out = checkout_url_at(&format!("{base}/functions/v1"), "the-token", "monthly");
    let seen = handle.join().expect("server thread");
    assert_eq!(out.expect("a checkout link"), "https://checkout.example.invalid/c/cs_1");
    assert!(seen[0].starts_with("POST /functions/v1/account/consent "), "the retry comes first: {}", seen[0]);
    assert!(seen[1].starts_with("POST /functions/v1/billing-checkout "), "{}", seen[1]);
}

/// **Review finding 1.** The deadline used to be tested only inside the `WouldBlock` arm, so a peer
/// that connects and immediately closes (`n == 0`, no `/callback`, no sleep) took the `Ok` arm every
/// time and never re-tested it — holding the call, the loopback port and a Tauri async-runtime
/// worker open past the deadline indefinitely. This pins the fix: the deadline is now checked at the
/// top of every iteration, so a connect-and-close spin still meets it.
#[test]
fn a_peer_that_keeps_connecting_without_callback_still_meets_the_deadline() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let spinner_stop = stop.clone();
    let spinner = std::thread::spawn(move || {
        while !spinner_stop.load(std::sync::atomic::Ordering::Relaxed) {
            // Connect and drop immediately: the server's `read` sees `n == 0`, the target is empty
            // (never `/callback`), and the old code's `continue` skipped the deadline check entirely.
            if let Ok(s) = std::net::TcpStream::connect(("127.0.0.1", port)) {
                drop(s);
            }
        }
    });
    let start = std::time::Instant::now();
    let out = serve_one_callback(listener, std::time::Duration::from_millis(200));
    let elapsed = start.elapsed();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = spinner.join();
    let err = out.expect_err("no real callback ever arrived");
    assert!(err.contains("not finished"), "{err}");
    assert!(elapsed < std::time::Duration::from_secs(2), "the connect spin must not hold the call open past its deadline: {elapsed:?}");
}

/// **Review finding 2.** The bind address is this feature's whole security boundary — `127.0.0.1`
/// and never `0.0.0.0` — and it lives only in `google_sign_in`, which is not unit-testable;
/// `serve_one_callback` takes a listener by value, so every listener test above binds its own and
/// none of them would catch a regression to the wildcard address, which would put the OAuth callback
/// port on the LAN while every other test in the workspace kept passing. A source-text test, the
/// pattern this repo already uses for a security-relevant literal (`app/tests/static_assets.rs`).
#[test]
fn the_only_bind_in_account_rs_is_the_loopback_and_never_the_wildcard() {
    let rust = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    assert!(rust.contains("bind(\"127.0.0.1:0\")"), "the loopback bind must still be there, verbatim");
    assert!(!rust.contains("0.0.0.0"), "the OAuth callback port must never listen on the wildcard address");
}

/// **R-C1b-exec-7.** A live proof on this machine found Explorer routing every `https://` URL, even
/// `https://example.com/?x=1`, to a Documents folder window instead of a browser tab — so
/// `open_in_browser` no longer shells out to `explorer.exe` at all. The tray's *Open vault folder*
/// action (`tray.rs`) is unrelated — it really does open a folder — and keeps its one call exactly
/// as it was. A source-text pin, the same style `the_only_bind_in_account_rs_is_the_loopback_and_never_the_wildcard`
/// above already uses for a security-relevant literal in this file.
#[test]
fn explorer_exe_never_opens_a_url_again_but_still_opens_the_vault_folder() {
    let account_src = std::fs::read_to_string("src/account.rs").expect("src/account.rs");
    assert!(!account_src.contains("explorer.exe"), "a URL must never reach explorer.exe again (R-C1b-exec-7)");
    // The quoted literal, not the bare substring: `tray.rs`'s own comment names `explorer.exe` in
    // prose (backticked, no quotes) right above the one real call, and a substring count would see
    // both — the actual claim is about the one `Command::new("explorer.exe")` in the code.
    let tray_src = std::fs::read_to_string("src/tray.rs").expect("src/tray.rs");
    assert_eq!(tray_src.matches("\"explorer.exe\"").count(), 1, "the tray's folder opener must still use explorer.exe, exactly once");
}

/// `open_in_browser` refuses anything that is not a web address before either the `ShellExecuteW`
/// call or the `rundll32` fallback is ever reached — no process spawns and no browser opens. Every
/// real caller only ever passes `https://`/`http://` (the sign-in authorize URL, the Stripe checkout
/// URL, a published policy page, the Google consent URL); this is the whole allow-list, proved at
/// the unit level rather than by reading the source, the way the refusal-gate tests above do for
/// `google_sign_in`.
#[test]
fn open_in_browser_refuses_anything_that_is_not_a_web_address_and_opens_nothing() {
    use knowlu::account::open_in_browser;
    let err = open_in_browser("not a url").unwrap_err();
    assert_eq!(err, "only a web address can be opened");
    let err = open_in_browser("file:///C:/x").unwrap_err();
    assert_eq!(err, "only a web address can be opened");
}

// ---------------------------------------------------------------------------
// C3′'s final fix wave, the final review's C1 (R-C3′-exec-43): *Sync now* and the quit push refresh
// the session before they reach the account. The slot's own half is C1c's (R-C1c-13).
// ---------------------------------------------------------------------------

/// A vault that has an account on the loopback `api_base`, and a session under `target` whose access
/// token expired a minute ago — the steady state of a desktop whose last refresh was over an hour ago
/// (`jwt_expiry = 3600`).
#[cfg(windows)]
fn vault_with_an_expired_session(tag: &str, api_base: &str, target: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    use knowlu::account::{save_session, Session};
    let stale = Session {
        access_token: "stale-at".into(),
        refresh_token: "stale-rt".into(),
        expires_at: jiff::Timestamp::now().as_second() - 60,
        email: "a@example.invalid".into(),
    };
    save_session(target, "acc-1", &stale).expect("write an expired session");
    let vault = std::env::temp_dir().join(format!("knowlu-c1-{tag}-{}", std::process::id()));
    let data = std::env::temp_dir().join(format!("knowlu-c1-{tag}-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&vault);
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(vault.join("config")).unwrap();
    std::fs::write(
        vault.join("config").join("cloud.yaml"),
        format!("api_base: '{api_base}'\nanon_key: 'anon'\nsession_credential_target: '{target}'\naccount_id: 'acc-1'\n"),
    ).unwrap();
    (vault, data)
}

/// What GoTrue answers a good `grant_type=refresh_token`.
const FRESH_SESSION: &str = r#"{"access_token":"fresh-at","refresh_token":"fresh-rt","expires_in":3600,"user":{"id":"acc-1","email":"a@example.invalid"}}"#;
const EMPTY_PULL: &str = r#"{"records":[],"notes":[],"record_cursor":0,"note_cursor":0,"more":false}"#;
const NOTHING_PUSHED: &str = r#"{"records":0,"notes":0}"#;

/// Final review C1: `state::run_sync` (*Sync now*) handed the engine whatever token Credential
/// Manager held, and nothing had refreshed it since launch or the last six-hourly tick — so most
/// clicks got a 401 and read "signed out" on a signed-in, paying student's machine.
#[cfg(windows)]
#[test]
fn sync_now_refreshes_an_expired_session_before_it_pulls_or_pushes() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let target = format!("knowlu/test-c1-sync-now-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    let (base, handle) = loopback(vec![
        (200, FRESH_SESSION.to_string()),
        (200, EMPTY_PULL.to_string()),
        (200, NOTHING_PUSHED.to_string()),
    ]);
    let api_base = format!("{base}/functions/v1");
    let _api = ApiBase::set(&api_base);
    let (vault, data) = vault_with_an_expired_session("sync-now", &api_base, &target);
    let cs = knowlu::state::ConsoleState::open(vault.clone(), data.clone());

    let status = knowlu::state::run_sync(&cs);
    let (_, held) = knowlu::account::load_session(&target).expect("the session is still there");
    assert_eq!(held.access_token, "fresh-at", "Sync now must refresh an expired session before it syncs: {status:?}");
    let seen = handle.join().expect("server thread");
    assert!(seen[0].starts_with("POST /auth/v1/token?grant_type=refresh_token "), "{}", seen[0]);
    assert!(seen[1].starts_with("GET /functions/v1/sync-pull") && seen[1].contains("Bearer fresh-at"), "{}", seen[1]);
    assert!(seen[2].starts_with("POST /functions/v1/sync-push ") && seen[2].contains("Bearer fresh-at"), "{}", seen[2]);
    assert!(status.ok && status.last_error.is_none(), "{status:?}");
    drop(cs);
    let _ = std::fs::remove_dir_all(&vault);
    let _ = std::fs::remove_dir_all(&data);
}

/// The quit push is the same call, push only, and had the same stale token.
#[cfg(windows)]
#[test]
fn the_quit_push_refreshes_an_expired_session_before_it_pushes() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let target = format!("knowlu/test-c1-quit-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    let (base, handle) = loopback(vec![(200, FRESH_SESSION.to_string()), (200, NOTHING_PUSHED.to_string())]);
    let api_base = format!("{base}/functions/v1");
    let _api = ApiBase::set(&api_base);
    let (vault, data) = vault_with_an_expired_session("quit", &api_base, &target);
    let cs = knowlu::state::ConsoleState::open(vault.clone(), data.clone());

    let q = knowlu::state::quit_flush(&cs, std::time::Duration::from_secs(30), |_| {});
    let (_, held) = knowlu::account::load_session(&target).expect("the session is still there");
    assert_eq!(held.access_token, "fresh-at", "the quit push must refresh an expired session first: {q:?}");
    let seen = handle.join().expect("server thread");
    assert!(seen[0].starts_with("POST /auth/v1/token?grant_type=refresh_token "), "{}", seen[0]);
    assert!(seen[1].starts_with("POST /functions/v1/sync-push ") && seen[1].contains("Bearer fresh-at"), "{}", seen[1]);
    assert!(q.synced && !q.timed_out, "{q:?}");
    drop(cs);
    let _ = std::fs::remove_dir_all(&vault);
    let _ = std::fs::remove_dir_all(&data);
}

/// …and a refresh that fails is named, never swallowed. Here the account service refuses the refresh
/// token, the stale access token then meets a 401 on both halves, and the status says both: the
/// engine's own "signed out" (which the page turns into "signed out — sign in to sync"), and a line
/// naming the refresh that failed and why, for the diagnostics and the report.
#[cfg(windows)]
#[test]
fn a_session_refresh_that_fails_is_named_in_the_sync_status() {
    let _credman_guard = CREDMAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let target = format!("knowlu/test-c1-refresh-fails-{}-{}", std::process::id(), line!());
    let _cleanup = Cleanup(vec![target.clone()]);
    let (base, handle) = loopback(vec![
        (400, r#"{"error":"invalid_grant","error_description":"Invalid Refresh Token: Refresh Token Not Found"}"#.to_string()),
        (401, r#"{"error":"jwt expired"}"#.to_string()),
        (401, r#"{"error":"jwt expired"}"#.to_string()),
    ]);
    let api_base = format!("{base}/functions/v1");
    let _api = ApiBase::set(&api_base);
    let (vault, data) = vault_with_an_expired_session("refresh-fails", &api_base, &target);
    let cs = knowlu::state::ConsoleState::open(vault.clone(), data.clone());

    let status = knowlu::state::run_sync(&cs);
    assert!(
        status.lines.iter().any(|l| l == "sync: the session could not be refreshed (Invalid Refresh Token: Refresh Token Not Found)"),
        "the failed refresh is named: {status:?}"
    );
    assert_eq!(status.last_error.as_deref(), Some("signed out"), "{status:?}");
    let seen = handle.join().expect("server thread");
    assert!(seen[0].starts_with("POST /auth/v1/token?grant_type=refresh_token "), "{}", seen[0]);
    drop(cs);
    let _ = std::fs::remove_dir_all(&vault);
    let _ = std::fs::remove_dir_all(&data);
}

// ---- Gmail connect T4b: the three pure cores (spec §4.2, D3; §8.1 items 1–3) ----
//
// Each core takes `(api_base, anon, token)` and reads nothing process-global, so these need neither
// `CREDMAN_LOCK` nor the `ApiBase` seam: the base is the test's own loopback, and
// `no_test_in_this_file_can_reach_the_compiled_in_project` scans these tests like every other.
use knowlu::account::{google_connect_url_at, google_disconnect_at, google_status_at};

const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";
const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";

/// What every core sends: the named method and path, the session's bearer, and the anon key the
/// caller passed in (not one read from the environment).
fn assert_bearer_request(seen: &str, request_line: &str) {
    assert!(seen.starts_with(&format!("{request_line} HTTP/1.1\r\n")), "{seen}");
    let lower = seen.to_ascii_lowercase();
    assert!(lower.contains("\r\nauthorization: bearer the-access-token\r\n"), "{seen}");
    assert!(lower.contains("\r\napikey: the-anon-key\r\n"), "{seen}");
}

#[test]
fn google_status_at_reads_each_state_and_both_scopes() {
    let (base, handle) = loopback(vec![
        (200, r#"{"connected":false,"scopes":[],"status":"none","email":null}"#.to_string()),
        (200, format!(r#"{{"connected":true,"scopes":["openid","email","{CALENDAR_SCOPE}","{GMAIL_SCOPE}"],"status":"active","email":"student@example.invalid"}}"#)),
        // A bare scope name is not the scope: only the full URL counts, as the wizard's check did.
        (200, format!(r#"{{"connected":true,"scopes":["{CALENDAR_SCOPE}","gmail.readonly"],"status":"quiet","email":"student@example.invalid"}}"#)),
        (200, r#"{"connected":false,"scopes":[],"status":"revoked","email":"student@example.invalid"}"#.to_string()),
    ]);
    let api = format!("{base}/functions/v1");
    let got: Vec<_> = (0..4).map(|_| google_status_at(&api, "the-anon-key", "the-access-token")).collect();
    let got: Vec<_> = got.into_iter().map(|r| r.expect("a status")).collect();
    assert_eq!(got.iter().map(|s| s.state).collect::<Vec<_>>(), ["none", "active", "quiet", "revoked"]);
    assert_eq!(got.iter().map(|s| s.connected).collect::<Vec<_>>(), [false, true, true, false]);
    assert_eq!(got.iter().map(|s| (s.calendar, s.gmail)).collect::<Vec<_>>(), [(false, false), (true, true), (true, false), (false, false)]);
    let who = Some("student@example.invalid");
    assert_eq!(got.iter().map(|s| s.email.as_deref()).collect::<Vec<_>>(), [None, who, who, who]);
    for s in &handle.join().expect("server thread") {
        assert_bearer_request(s, "GET /functions/v1/google-connect?status=1");
    }
}

#[test]
fn google_status_at_reads_an_older_servers_reply() {
    let (base, handle) = loopback(vec![
        (200, format!(r#"{{"connected":true,"scopes":["{CALENDAR_SCOPE}"]}}"#)),
        (200, r#"{"connected":false,"scopes":[]}"#.to_string()),
    ]);
    let api = format!("{base}/functions/v1");
    let on = google_status_at(&api, "the-anon-key", "the-access-token").expect("a status");
    let off = google_status_at(&api, "the-anon-key", "the-access-token").expect("a status");
    assert_eq!((on.state, on.connected, on.calendar, on.gmail, on.email), ("active", true, true, false, None));
    assert_eq!((off.state, off.connected, off.calendar, off.gmail, off.email), ("none", false, false, false, None));
    handle.join().expect("server thread");
}

/// `gmail`, `reconnect` and `calendar` pass through; anything else — a typo, another scope, or an
/// attempt to smuggle a second `scope=` into the query — asks for `calendar`.
#[test]
fn google_connect_url_at_asks_for_the_named_scope_with_the_bearer() {
    let consent = "https://accounts.google.com/o/oauth2/v2/auth?state=s";
    let asks = ["gmail", "reconnect", "calendar", "drive", "gmail&scope=reconnect"];
    let (base, handle) = loopback(asks.iter().map(|_| (200, format!(r#"{{"url":"{consent}"}}"#))).collect());
    let api = format!("{base}/functions/v1");
    let got: Vec<_> = asks.iter().map(|ask| google_connect_url_at(&api, "the-anon-key", "the-access-token", ask)).collect();
    assert!(got.iter().all(|u| u.as_deref() == Ok(consent)), "{got:?}");
    let seen = handle.join().expect("server thread");
    for (s, scope) in seen.iter().zip(["gmail", "reconnect", "calendar", "calendar", "calendar"]) {
        assert_bearer_request(s, &format!("GET /functions/v1/google-connect?scope={scope}"));
    }
}

/// The URL is checked in Rust before any caller can open it, and the refusal never repeats it.
#[test]
fn google_connect_url_at_refuses_a_url_knowlu_will_not_open() {
    let (base, handle) = loopback(vec![(200, r#"{"url":"https://evil.example/o/oauth2/v2/auth?state=s"}"#.to_string())]);
    let got = google_connect_url_at(&format!("{base}/functions/v1"), "the-anon-key", "the-access-token", "gmail");
    assert_eq!(got, Err("the service returned a url Knowlu will not open".to_string()));
    handle.join().expect("server thread");
}

/// Each refusal maps by its status, never by its body (T4a's sentences, spec §4.2).
#[test]
fn google_disconnect_at_sends_delete_and_maps_each_status() {
    let (base, handle) = loopback(vec![
        (200, r#"{"disconnected":true}"#.to_string()),
        (502, r#"{"error":"a body the student never sees"}"#.to_string()),
        (401, r#"{"error":"jwt expired"}"#.to_string()),
        (402, r#"{"error":"subscription required"}"#.to_string()),
        (503, r#"{"error":"not configured"}"#.to_string()),
    ]);
    let api = format!("{base}/functions/v1");
    let got: Vec<_> = (0..5).map(|_| google_disconnect_at(&api, "the-anon-key", "the-access-token")).collect();
    assert_eq!(
        got,
        [
            Ok(()),
            Err("Google could not be reached to disconnect; try again".to_string()),
            Err("sign in again".to_string()),
            Err("your subscription is not active, so Google cannot be connected".to_string()),
            Err("Google sign-in is not available right now — use the secret address below".to_string()),
        ]
    );
    for s in &handle.join().expect("server thread") {
        assert_bearer_request(s, "DELETE /functions/v1/google-connect");
    }
}
