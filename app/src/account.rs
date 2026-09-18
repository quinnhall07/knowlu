//! The account: the session, the sign-in, the entitlement cache, and the commands the wizard and the
//! settings panel call (Knowlu C1, Tasks 10, 11 and 17).
//!
//! **Two rules from `credentials.rs` carry over unchanged.** A secret is never logged, never in a run
//! record, a backup, a fixture, a test name or an error message; and the page never sees one — a
//! command takes an email address or a six-digit code in and gives an envelope back, and the token
//! that comes out of it goes straight into Credential Manager without passing through the webview.
use serde_json::{json, Value};

/// The Supabase project this build talks to. **Both values are public**: Supabase publishes the
/// project URL and the anon key in every client it generates, and neither grants anything on its own
/// — row-level security and the edge functions decide what a caller may do. The service-role key,
/// which does bypass all of that, is a function secret Quinn sets and appears nowhere in this repo.
///
/// Filled from precondition P1 (the `knowlu-prod` project, 2026-09-10). `KNOWLU_API_BASE` /
/// `KNOWLU_ANON_KEY` override them at run time, which is how a scratch profile is pointed at
/// `knowlu-staging` (§11 R6) without a second build — the same shape `KNOWLU_ENGINE_EXE` already uses.
pub const DEFAULT_API_BASE: &str = "https://jxthohvwrijwtuwlglan.supabase.co/functions/v1";
pub const DEFAULT_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Imp4dGhvaHZ3cmlqd3R1d2xnbGFuIiwicm9sZSI6ImFub24iLCJpYXQiOjE3ODkwMTczMDEsImV4cCI6MjEwNDU5MzMwMX0.hBNxOo94M9rF1_IEm93d7s64fmdg6RV8qy5kcJcuYNU";

/// The policy versions a sign-up records. Dates, not numbers, because the pages carry a date too and
/// a reader comparing the two should not have to hold a mapping in their head. Bump BOTH the constant
/// and the page's date in the same commit, or the consent log points at text nobody can find.
pub const TOS_VERSION: &str = "2026-09-10";
pub const PRIVACY_VERSION: &str = "2026-09-16";

/// Where a session lives before there is a vault to key it to. The wizard signs in on panel 2 and
/// creates the vault on panel 9, so for those seven panels the profile id does not exist yet — and it
/// is derived from the vault path, which the user can still change. `move_session` walks it over at
/// Finish, exactly as `retarget_credentials` walks the coursework logins (R-P4a-23), and for exactly
/// the same reason: a credential filed under a path nothing will look at is worse than no credential.
pub const PENDING_TARGET: &str = "knowlu/pending/session";

/// GoTrue's `/verify` request body names a `type`. Settled against staging on 2026-09-10 (Task 10
/// step 3a, closing ruling R-C1-25): a `POST /auth/v1/verify` with `"magiclink"` and a made-up code
/// answers `403 otp_expired` — it got as far as looking the token up, so the type is accepted.
/// (`"email"` is accepted by this GoTrue too; `"magiclink"` stays because it is what the plan named.)
const VERIFY_TYPE: &str = "magiclink";

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub fn api_base() -> String {
    std::env::var("KNOWLU_API_BASE").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_API_BASE.to_string())
}

pub fn anon_key() -> String {
    std::env::var("KNOWLU_ANON_KEY").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_ANON_KEY.to_string())
}

/// `https://` always; `http://127.0.0.1:` only, and only so the tests in `app/tests/account.rs` can
/// stand a real server up on a real socket without a `#[cfg(test)]` branch inside the production
/// path. Nothing else — an `http://` host on a campus wifi is a session token in the clear.
pub fn check_api_base(api_base: &str) -> Result<(), String> {
    let scheme_ok = api_base.starts_with("https://") || api_base.starts_with("http://127.0.0.1:");
    // `http://127.0.0.1:x@evil.com/...` starts with the literal loopback prefix above, but an `@`
    // in the authority means everything before it is userinfo and the real host is whatever
    // follows — `evil.com` here. Refusing any `@` in the authority closes that off rather than
    // trusting a prefix match a crafted URL can still wear.
    let authority = api_base.split("://").nth(1).and_then(|rest| rest.split('/').next()).unwrap_or("");
    if scheme_ok && !authority.contains('@') {
        Ok(())
    } else {
        Err(format!("{api_base}: an api_base must be https://"))
    }
}

/// `…/functions/v1` → `…/auth/v1`. Derived rather than stored because `config/cloud.yaml`'s four keys
/// are a contract with C2 and adding a fifth would be a conversation. Strict on purpose: anything
/// that is not a functions base is refused here rather than turned into a request to a host nobody
/// chose.
pub fn auth_base(api_base: &str) -> Result<String, String> {
    let trimmed = api_base.trim_end_matches('/');
    match trimmed.strip_suffix("/functions/v1") {
        Some(root) => {
            check_api_base(api_base)?;
            Ok(format!("{root}/auth/v1"))
        }
        None => Err(format!("{api_base}: an api_base ends in /functions/v1")),
    }
}

/// What Credential Manager holds at `knowlu/<profile_id>/session`. `UserName` is the account id — not
/// a secret, and the one field C2 reads without decoding anything — and the blob is this struct as
/// JSON. `expires_at` is Unix seconds, so a clock comparison needs no date library at the call site.
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub email: String,
}

/// Hand-written, the way `wincred::Secret` is: a `Session` is two live credentials, and a derived
/// `Debug` is one `{:?}` — a `dbg!`, an `unwrap()` panic, a stray log line — away from putting both
/// of them somewhere that is not Credential Manager.
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("access_token", &"<redacted>")
            .field("refresh_token", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .field("email", &self.email)
            .finish()
    }
}

pub fn session_target(profile_id: &str) -> String { crate::credentials::target_for(profile_id, "session") }

/// Credential Manager's generic blob is UTF-16 and Windows caps `CredentialBlobSize` at 2,560 bytes
/// (`CRED_MAX_CREDENTIAL_BLOB_SIZE`) — 1,280 UTF-16 code units. A `Session` this size never comes
/// from GoTrue; refusing it here, before the Win32 call, turns a silent truncation into a message
/// that names what happened.
const MAX_SESSION_CHARS: usize = 1_280;

pub fn save_session(target: &str, account_id: &str, s: &Session) -> Result<(), String> {
    let blob = serde_json::to_string(s).map_err(|e| e.to_string())?;
    if blob.chars().count() > MAX_SESSION_CHARS {
        return Err("the sign-in reply is too large to keep on this device — please report this".to_string());
    }
    crate::credentials::write(target, account_id, &blob)
}

/// A fixed sentence rather than serde's own message: whatever is stored under `target` is not this
/// process's concern to describe — it is a signed-in state that failed to read back, and "sign in
/// again" is the one thing a user can act on.
const UNREADABLE_SESSION: &str = "the stored session is unreadable; sign in again";

pub fn load_session(target: &str) -> Result<(String, Session), String> {
    let cred = knowlu_engine::wincred::read_credential(target).map_err(|e| e.to_string())?;
    let s: Session = serde_json::from_str(cred.password.expose()).map_err(|_| UNREADABLE_SESSION.to_string())?;
    Ok((cred.username, s))
}

/// Read, write the new one, delete the old — in that order, never delete-then-write: a failure in
/// between would leave the user signed out with no way back but another emailed code or another trip
/// through the browser. `retarget_credentials` makes the same argument about the coursework logins.
///
/// Idempotent both ways: a `from` that is already gone with a `to` that already holds a session is
/// **not** an error — the move already happened, most likely on an earlier call this one is
/// retrying — and `to`'s prior contents are always overwritten by `from`'s, so the moved session
/// wins. Only a `from` and `to` that are **both** empty is a real failure, and the message names the
/// source so the caller knows what it was looking for.
pub fn move_session(from: &str, to: &str) -> Result<(), String> {
    if from == to {
        return Ok(());
    }
    if !crate::credentials::exists(from) {
        return if crate::credentials::exists(to) {
            Ok(())
        } else {
            Err(format!("no session at {from} to move"))
        };
    }
    let (account_id, s) = load_session(from)?;
    save_session(to, &account_id, &s)?;
    crate::credentials::delete(from)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        // FALSE deliberately: GoTrue says why it refused in the body of a 400, and turning that into
        // an `Err(status)` would replace "Invalid login credentials" with "400" on the panel.
        .http_status_as_error(false)
        .build()
        .into()
}

pub const UNREACHABLE: &str = "the account service could not be reached";

/// One GoTrue POST. Returns the parsed body and the status; the caller decides what a status means.
/// **The transport failure has a stable first clause**, [`UNREACHABLE`], because the page has to
/// tell "that code is wrong" from "there is no network" — the first is something a user can fix
/// on the panel, the second is what stands the upgrade overlay down instead of trapping someone
/// behind it. Everything else is the provider's own sentence.
///
/// **The body is sent compact, not through `send_json`.** `send_json` needs ureq's `json` feature,
/// which `app/Cargo.toml` does not enable (it carries the engine's `cookies` feature and no other) —
/// but the choice is not free of behaviour either way: `RequestBuilder::send_json` pretty-prints
/// with embedded newlines, and this crate's own loopback tests assert exact substrings like
/// `"type":"magiclink"` and `"create_user":true` against the raw request text, which a pretty-printed
/// body would break. Serializing with `serde_json::to_string` — already a dependency — and sending
/// the compact result as a plain string is **required**, not merely equivalent.
fn post_json(url: &str, anon: &str, body: &Value) -> Result<(u16, Value), String> {
    let text = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let mut res = agent()
        .post(url)
        .header("apikey", anon)
        .header("content-type", "application/json")
        .send(text)
        .map_err(|e| format!("{UNREACHABLE} ({e})"))?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 20).read_to_string().map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    Ok((status, value))
}

/// GoTrue's own sentence, whichever field it used this time; never a bare status code on a panel.
fn provider_error(status: u16, v: &Value) -> String {
    for key in ["error_description", "msg", "message", "error"] {
        if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
            if !s.is_empty() { return s.to_string(); }
        }
    }
    format!("the account service refused the request ({status})")
}

fn session_from(v: &Value, now_unix: i64) -> Result<(String, Session), String> {
    let account_id = v.get("user").and_then(|u| u.get("id")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let access_token = v.get("access_token").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let refresh_token = v.get("refresh_token").and_then(|x| x.as_str()).unwrap_or_default().to_string();
    if account_id.is_empty() || access_token.is_empty() {
        return Err("the account service returned no session".to_string());
    }
    let expires_in = v.get("expires_in").and_then(|x| x.as_i64()).unwrap_or(3600);
    let email = v.get("user").and_then(|u| u.get("email")).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    Ok((account_id, Session { access_token, refresh_token, expires_at: now_unix + expires_in, email }))
}

/// Base64url without padding (RFC 4648 §5), written here rather than taken as a crate: it is a
/// table lookup, and the workspace's crate budget is a product line. `-` and `_` instead of `+` and
/// `/` is the whole point — the challenge travels in a query string.
pub fn b64url(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        // 4 characters for 3 bytes, 3 for 2, 2 for 1 — and no `=`, which is what "unpadded" means.
        for i in 0..(chunk.len() + 1) {
            out.push(A[((n >> (18 - 6 * i)) & 0x3f) as usize] as char);
        }
    }
    out
}

/// A verifier and its challenge (RFC 7636 §4.1, §4.2). 32 bytes of OS entropy is 43 unpadded
/// base64url characters, the low end of the standard's 43..=128 range and the length every client
/// library uses.
///
/// **The entropy is `knowlu_engine::ids::new_id`'s**, five bytes at a time. `getrandom` is the
/// engine's dependency and not the app's, and seven calls to a function that already asks the OS is
/// a smaller change than a new crate or a new `windows` feature for one buffer. The hex it returns
/// is decoded back to bytes rather than used as text, so the verifier really is 256 bits and not
/// 256 bits' worth of hex digits.
pub fn pkce_pair() -> (String, String) {
    use sha2::{Digest, Sha256};
    let mut bytes = Vec::with_capacity(35);
    while bytes.len() < 32 {
        let id = knowlu_engine::ids::new_id("pkce");
        let hex = id.rsplit('_').next().unwrap_or_default().to_string();
        for pair in hex.as_bytes().chunks(2) {
            if let Ok(b) = u8::from_str_radix(&String::from_utf8_lossy(pair), 16) {
                bytes.push(b);
            }
        }
    }
    bytes.truncate(32);
    let verifier = b64url(&bytes);
    let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// The loopback redirect, percent-encoded as a query value. Only the six characters that appear in
/// `http://127.0.0.1:<port>/callback` need it, so this is not a general encoder and does not pretend
/// to be one.
fn redirect_to(port: u16) -> String {
    format!("http%3A%2F%2F127.0.0.1%3A{port}%2Fcallback")
}

/// `<auth>/authorize?…` — the URL the system browser is sent to.
///
/// **No `state`.** GoTrue owns it: a client-supplied `state` is deleted from the query before the
/// provider is called (`reservedOAuthParams`, `internal/api/external.go`), and the flow state it
/// creates instead is what carries the challenge across the round trip. **No `flow_type`** either:
/// the presence of `code_challenge` is what selects PKCE (`getFlowFromChallenge`).
pub fn authorize_url(auth_base: &str, port: u16, challenge: &str) -> String {
    format!(
        "{}/authorize?provider=google&redirect_to={}&code_challenge={challenge}&code_challenge_method=s256",
        auth_base.trim_end_matches('/'),
        redirect_to(port),
    )
}

/// Percent-decoding, for the one query value this file reads back.
fn pct_decode(s: &str) -> String {
    let b = s.replace('+', " ").into_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&String::from_utf8_lossy(&b[i + 1..i + 3]), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// The authorisation code out of `GET /callback?code=… HTTP/1.1`, or the provider's own sentence.
///
/// Pure, and taking the request line rather than a socket, so the parsing is tested directly — the
/// shape `google_error_for_status` uses in this same file, and for the same reason. A request that
/// is not the callback (a browser's `/favicon.ico`, a probe) is an `Err`, never an empty `Ok`.
pub fn code_from_request_line(line: &str) -> Result<String, String> {
    let target = line.split_whitespace().nth(1).unwrap_or_default();
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or_default();
    let mut code = None;
    let mut error = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("code", v)) => code = Some(pct_decode(v)),
            Some(("error_description", v)) => error = Some(pct_decode(v)),
            Some(("error", v)) if error.is_none() => error = Some(pct_decode(v)),
            _ => {}
        }
    }
    match (code, error) {
        (Some(c), _) if !c.is_empty() => Ok(c),
        (_, Some(e)) if !e.is_empty() => Err(e),
        _ => Err("the browser came back without a sign-in code".to_string()),
    }
}

/// What the browser is left looking at. One sentence, no styling, no script, no link back — the
/// student's next move is the Knowlu window that is already open behind it.
pub const CALLBACK_PAGE: &str =
    "<!doctype html><meta charset=\"utf-8\"><title>Knowlu</title>\
     <p style=\"font:16px system-ui;margin:3rem\">You are signed in to Knowlu. You can close this window.</p>";

/// The other page: anything on this machine that is not the sign-in.
pub const NOT_FOUND_PAGE: &str =
    "<!doctype html><meta charset=\"utf-8\"><title>Knowlu</title>\
     <p style=\"font:16px system-ui;margin:3rem\">Nothing here. You can close this window.</p>";

/// One reply, on a socket this function is finished with.
fn write_page(stream: &mut std::net::TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    use std::io::Write;
    let resp = format!(
        "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.flush()
}

/// Serve exactly **one sign-in**: wait for the callback, answer it, and give the socket back to the
/// OS.
///
/// `listener` is taken **by value** on purpose: it is dropped when this returns, so there is no way
/// to leave a port open on a student's machine after a sign-in — successful, refused or abandoned.
/// The deadline is enforced by polling `accept` on a non-blocking listener rather than by a second
/// thread, so nothing outlives the call.
///
/// **It loops to the deadline rather than returning on the first connection** (review I3). An open
/// loopback port is reachable by everything else on the machine — a browser preconnect, a favicon
/// fetch, security software, a port scanner — and taking the first socket as the answer meant a
/// stray request ended the sign-in while the real redirect was still in flight, which then met a
/// closed port. Anything whose target is not `/callback` gets a 404 and the wait continues; the
/// first real callback returns, and the guarantee is unchanged, because the listener dies with this
/// call.
///
/// **The compensating control against a *forged or replayed* callback is the verifier, not the
/// socket.** GoTrue owns `state` — a client-supplied one is stripped (`reservedOAuthParams`) — so
/// this cannot carry a nonce of its own, and it does not need one: the verifier is minted per
/// attempt and never leaves the process, so a code minted under any other challenge fails the
/// exchange. That is the actual guarantee (review finding 3) — it is not a claim that a same-user
/// process is locked out. The challenge itself is not secret: it travels as an argument on the
/// browser's command line (`open_in_browser`), readable by anything running as the same Windows
/// account, which could equally well read `knowlu/pending/session` out of Credential Manager once a
/// sign-in lands. Same-user code execution is outside this function's threat model; a local process
/// that only guesses the port, with no challenge of its own, can end a sign-in with a refusal
/// sentence and nothing more.
///
/// The reply is sent on **every** path. A refusal is still a browser window a person is looking at,
/// and a connection reset is not an explanation.
pub fn serve_one_callback(listener: std::net::TcpListener, wait: std::time::Duration) -> Result<String, String> {
    use std::io::Read;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + wait;
    loop {
        // Checked at the top of every iteration, not only in the `WouldBlock` arm below (review
        // finding 1): a peer that connects and disconnects in a loop without ever sending
        // `/callback` takes the `Ok` arm every time, `continue`s past the check that used to live
        // only in `WouldBlock`, and never hit the deadline — holding the port, and the Tauri
        // async-runtime worker with it, open indefinitely.
        if std::time::Instant::now() >= deadline {
            return Err("the Google sign-in was not finished — try again".to_string());
        }
        let mut stream = match listener.accept() {
            Ok((s, _)) => s,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
        // The request LINE is all this needs, and a browser sends it in the first packet. Reading to
        // the first newline rather than to EOF is also what keeps a keep-alive connection from hanging.
        // A fixed 4096-byte buffer (review finding 4): a request line split across TCP segments would
        // be mis-parsed, but a loopback browser sends the header block in one segment, so the risk is
        // theoretical — and the fixed size is also what makes an oversized request line harmless (no
        // growth, no allocation). Any future loop reading to a terminator must keep this same cap.
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        let head = String::from_utf8_lossy(&buf[..n]).to_string();
        let line = head.lines().next().unwrap_or_default().to_string();
        let target = line.split_whitespace().nth(1).unwrap_or_default();
        if target.split('?').next().unwrap_or_default() != "/callback" {
            let _ = write_page(&mut stream, "404 Not Found", NOT_FOUND_PAGE);
            continue;
        }
        let _ = write_page(&mut stream, "200 OK", CALLBACK_PAGE);
        return code_from_request_line(&line);
    }
}

/// One button: the code that both creates the account and signs in. `create_user: true` is the whole
/// difference from C1's version — and **nothing about consent travels with it** (R-C1b-3). `data`
/// would land as `raw_user_meta_data`, and migration `20260917000100` stopped the trigger reading
/// it, precisely because this request is one anyone holding the public anon key can send for any
/// address they like. The attestation is recorded after the code is typed, by `POST /account/consent`.
pub fn magic_link_at(auth_base: &str, anon: &str, email: &str) -> Result<(), String> {
    let (status, v) = post_json(&format!("{auth_base}/otp"), anon, &json!({ "email": email, "create_user": true }))?;
    if (200..300).contains(&status) { Ok(()) } else { Err(provider_error(status, &v)) }
}

/// …and this is how a **desktop** app finishes one. The link in the mail redirects to
/// `https://knowlu.com/signed-in.html` with the session in the URL fragment, **in the user's
/// browser** — a place this process will never see. So the mail carries a six-digit code beside the
/// link (`config.toml`'s `[auth.email.template.magic_link]`, GoTrue's own `{{ .Token }}`), the panel
/// asks for it, and `/verify` trades it for the same session the link would have given.
///
/// No deep-link scheme, no URI registration, no new dependency — and the button on the panel does
/// what its label says, which is the whole point of keeping it (spec §5.1 names magic link as one of
/// the two identity paths).
/// The `type` is the one thing here that varies across GoTrue versions — [`VERIFY_TYPE`] records
/// how it was settled against staging, and is the only line that changes if a GoTrue upgrade moves it.
pub fn verify_email_code_at(auth_base: &str, anon: &str, email: &str, code: &str, now_unix: i64) -> Result<(String, Session), String> {
    let (status, v) = post_json(&format!("{auth_base}/verify"), anon, &json!({ "type": VERIFY_TYPE, "email": email, "token": code.trim() }))?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    session_from(&v, now_unix)
}

pub fn refresh_at(auth_base: &str, anon: &str, refresh_token: &str, now_unix: i64) -> Result<(String, Session), String> {
    let (status, v) = post_json(&format!("{auth_base}/token?grant_type=refresh_token"), anon, &json!({ "refresh_token": refresh_token }))?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    session_from(&v, now_unix)
}

/// **Refresh is C1's job** (the C2 contract, point 2). A token with under two minutes left is
/// refreshed and the entry rewritten, so C2 — which only ever reads — finds a live token or a stale
/// one it can wait out, and never has to hold a refresh race with this process.
pub fn valid_access_token_at(auth_base: &str, anon: &str, target: &str, now_unix: i64) -> Result<String, String> {
    let (account_id, s) = load_session(target)?;
    if s.expires_at - now_unix > 120 { return Ok(s.access_token); }
    let (id, fresh) = refresh_at(auth_base, anon, &s.refresh_token, now_unix)?;
    let id = if id.is_empty() { account_id } else { id };
    save_session(target, &id, &fresh)?;
    Ok(fresh.access_token)
}

fn env_pair() -> Result<(String, String, String), String> {
    let base = api_base();
    check_api_base(&base)?;
    Ok((auth_base(&base)?, anon_key(), base))
}

fn now_unix() -> i64 { jiff::Timestamp::now().as_second() }

fn ok_account(account_id: &str, email: &str) -> Value {
    json!({ "ok": true, "error": Value::Null, "account_id": account_id, "email": email })
}

/// The attestation is still refused **here**, before a single mail is sent — it is the consent the
/// wizard's two checkboxes stand for. What changed is where it is recorded: in the account, by the
/// route, once the code has proved the address.
#[tauri::command(async)]
pub fn send_magic_link(email: String, age_attested: bool) -> Value {
    if !age_attested {
        return json!({ "ok": false, "error": "Knowlu is for people 18 or older." });
    }
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e }) };
    match magic_link_at(&auth, &anon, email.trim()) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// The other half of the magic link: the code from the mail, traded for a session on this machine.
#[tauri::command(async)]
pub fn verify_email_code(email: String, code: String) -> Value {
    let (auth, anon, base) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e, "account_id": Value::Null }) };
    match verify_email_code_at(&auth, &anon, email.trim(), &code, now_unix()) {
        Ok((id, s)) => match save_session(PENDING_TARGET, &id, &s) {
            Ok(()) => {
                // The same call `google_sign_in` makes, for the same reason and with the same
                // handling: the trigger records no consent on either path any more, this is the one
                // writer, and a failure is logged rather than swallowed — `open_checkout` retries it
                // and `billing-checkout` refuses an account that still has no attestation.
                if let Err(e) = record_consent_at(&base, &s.access_token, TOS_VERSION, PRIVACY_VERSION) {
                    eprintln!("Knowlu: the sign-up consent could not be recorded ({e})");
                }
                ok_account(&id, &s.email)
            }
            Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
        },
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}

/// The system browser, by `explorer.exe <url>` — the same mechanism the tray's *Open vault folder*
/// uses, and no new dependency for one line. Its exit code is not checked: `explorer.exe` returns
/// non-zero on success often enough that checking it would report failures that did not happen.
pub fn open_in_browser(url: &str) -> Result<(), String> {
    use knowlu_engine::childproc::NoConsole;
    std::process::Command::new("explorer.exe").no_console().arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// **The terms and the privacy policy, opened where they can actually be read.** `app/static/` holds
/// four files; a plain `<a href="terms.html">` in the wizard navigates the one webview to a missing
/// asset and the window is lost until restart — while the user is being asked to tick a box saying
/// they accept it, which is the legal core of §9's consent requirement. The markup keeps its `href`
/// (the static test reads it, and it is the honest link), the page's handler calls
/// `preventDefault()`, and this opens the published page in the system browser.
#[tauri::command(async)]
pub fn open_policy(which: String) -> Value {
    let page = match which.as_str() {
        "terms" => "terms.html",
        "privacy" => "privacy.html",
        _ => return json!({ "ok": false, "error": format!("no policy called {which:?}") }),
    };
    match open_in_browser(&format!("https://knowlu.com/{page}")) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// Sign a profile out: forget its session locally, then revoke it (spec §5.1, "sign-out revokes").
/// `profile_id: None` addresses the pending session — the wizard's, before Finish has adopted it
/// into a profile; `Some(id)` addresses that profile's own `knowlu/<id>/session`. The revoke is
/// **best effort and second**: a network that is down must not leave a token on this machine that
/// the user believes they signed out of. `had_session` is the local delete's own answer — `false`
/// for a target that never held one — so a repeated sign-out reports honestly as a no-op instead of
/// a silent "ok".
#[tauri::command(async)]
pub fn sign_out(profile_id: Option<String>) -> Result<Value, String> {
    let target = match &profile_id {
        Some(id) => session_target(id),
        None => PENDING_TARGET.to_string(),
    };
    let token = load_session(&target).ok().map(|(_, s)| s.access_token);
    let had_session = crate::credentials::exists(&target);
    if had_session {
        crate::credentials::delete(&target)?;
    }
    if let (Some(t), Ok(auth)) = (token, auth_base(&api_base())) {
        let _ = agent()
            .post(&format!("{auth}/logout"))
            .header("apikey", &anon_key())
            .header("authorization", &format!("Bearer {t}"))
            .send_empty();
    }
    Ok(json!({ "ok": true, "had_session": had_session }))
}

/// The four keys `config/cloud.yaml` carries — the contract with C2, in this order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudConfig {
    pub api_base: String,
    pub anon_key: String,
    pub session_credential_target: String,
    pub account_id: String,
}

/// `scheme://host` (lower-cased), for comparing two `api_base`-shaped strings without caring about
/// path or case. Not a general URL parser — `check_api_base` has already ruled out userinfo by the
/// time this runs against a trusted value, and a value that fails to parse here compares unequal to
/// everything, which is the safe direction.
fn scheme_and_host(u: &str) -> Option<String> {
    let (scheme, rest) = u.split_once("://")?;
    let host = rest.split('/').next().unwrap_or("");
    Some(format!("{}://{}", scheme.to_ascii_lowercase(), host.to_ascii_lowercase()))
}

/// Read through `pystr` and `serde_yaml_ng`, never by comparing bytes: a vault's files are whatever
/// Windows made them, and the engine translates line endings on every read for that reason.
///
/// **The vault's `api_base` may not redirect the session (R-C1-59 I1).** A vault the app already
/// owns is still a file on disk, and every caller here hands this `api_base` a live access or
/// refresh token as a bearer credential. Without this check, a planted `config/cloud.yaml` naming
/// another host turns the next housekeeping tick into an exfiltration of both tokens. The compiled-in
/// `api_base()` (which itself honours `KNOWLU_API_BASE` for a scratch profile) is the only host this
/// build is allowed to talk to, so a mismatch is refused here, once, before any caller sees the value.
pub fn cloud_config(vault: &std::path::Path) -> Result<CloudConfig, String> {
    let path = vault.join("config").join("cloud.yaml");
    let text = knowlu_engine::pystr::read_text(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let cfg = CloudConfig {
        api_base: s("api_base"),
        anon_key: s("anon_key"),
        session_credential_target: s("session_credential_target"),
        account_id: s("account_id"),
    };
    if cfg.api_base.is_empty() || cfg.account_id.is_empty() {
        return Err(format!("{}: api_base and account_id are required", path.display()));
    }
    check_api_base(&cfg.api_base).map_err(|e| format!("{}: {e}", path.display()))?;
    if scheme_and_host(&cfg.api_base) != scheme_and_host(&api_base()) {
        return Err(format!(
            "{}: api_base names a different service than this build talks to",
            path.display()
        ));
    }
    Ok(cfg)
}

/// `GET /entitlement`'s reply, cached. The four keys are the C2 contract; a reply carrying a fifth
/// key this app does not know is read past on the way in and simply not there on the way back out —
/// `save_cache` always serialises exactly these four fields, never the raw bytes it received.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntitlementCache {
    pub status: String,
    pub current_period_end: Option<String>,
    pub plan: Option<String>,
    pub checked_at: String,
}

/// Spec §5.1. Seventy-two hours, so a weekend of bad wifi never blanks today's page.
pub const GRACE: std::time::Duration = std::time::Duration::from_secs(72 * 60 * 60);

/// **A file beside `settings.json`, not a field in it.** `state::Settings` has no `#[serde(default)]`
/// on any field and `Settings::load` falls back to defaults on a parse failure, so adding a field
/// would silently reset a user's backup folder and autostart choice on their first launch after an
/// update. `onboarding::offer_marker` already makes this argument, and this follows its precedent.
pub fn cache_path(data_dir: &std::path::Path) -> std::path::PathBuf { data_dir.join("entitlement.json") }

pub fn load_cache(data_dir: &std::path::Path) -> Option<EntitlementCache> {
    let text = std::fs::read_to_string(cache_path(data_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Written atomically: the fresh bytes land in a sibling `.tmp` file first, and `fs::rename` — which
/// on Windows calls `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` — swaps it into place in one step,
/// so a reader (this process's own next `load_cache`, or a crash mid-write) never observes a
/// half-written file. The remove-then-rename fallback below is a second attempt for the rare case
/// where the direct rename itself fails (fix round 1, item 6).
pub fn save_cache(data_dir: &std::path::Path, c: &EntitlementCache) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let v = serde_json::to_value(c).map_err(|e| e.to_string())?;
    // `ledger::dumps_value`, like every other JSON this app writes: a file the app wrote and a file
    // the engine wrote never differ by whitespace.
    let bytes = knowlu_engine::ledger::dumps_value(&v);
    let path = cache_path(data_dir);
    let tmp = data_dir.join("entitlement.json.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(&tmp, &path) {
        // Fallback, and say so: remove the stale target and retry the rename rather than leaving
        // both the old cache and the fresh `.tmp` sitting on disk.
        let _ = std::fs::remove_file(&path);
        std::fs::rename(&tmp, &path).map_err(|e2| format!("entitlement cache rename failed ({e}); fallback also failed: {e2}"))?;
    }
    Ok(())
}

/// Four answers. `Unreadable` is `NoAccount`'s sibling, not its replacement (fix round 1, item 2):
/// **`NoAccount`** is a vault with no `config/cloud.yaml` at all — a pre-C1 install, or one that has
/// never onboarded into the cloud — and falls through to the local runtime/model gate exactly as
/// before. **`Unreadable`** is a file that IS there but does not parse, or is missing a required key:
/// a broken file must never be silently treated as "no account", which would run `judge` with no
/// account in play at all. The other two states are the sentence a user reads in the Runs view (`no
/// entitlement` vs `entitled`) — the same shape `IcsState` uses between "no feed" and "unreadable",
/// for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementState {
    Entitled,
    NotEntitled,
    NoAccount,
    Unreadable,
}

pub fn decide(cloud_configured: bool, cache: Option<&EntitlementCache>, now: jiff::Timestamp) -> EntitlementState {
    if !cloud_configured { return EntitlementState::NoAccount; }
    let Some(c) = cache else { return EntitlementState::NotEntitled };
    if c.status != "active" && c.status != "trialing" { return EntitlementState::NotEntitled; }
    let Ok(checked) = c.checked_at.parse::<jiff::Timestamp>() else { return EntitlementState::NotEntitled };
    let mut age = now.as_second() - checked.as_second();
    // Fix round 1, item 1: a clock a few minutes fast must not disentitle someone forever — each
    // refresh would otherwise write another "future" `checked_at` and the state would never recover
    // on its own. Up to an hour ahead is treated as "just checked"; beyond that it is still a clock
    // that moved, not a licence, and stays a sanity check rather than a licence.
    if age < 0 && -age <= 3600 { age = 0; }
    if age < 0 || age > GRACE.as_secs() as i64 { return EntitlementState::NotEntitled; }
    EntitlementState::Entitled
}

pub fn fetch_entitlement_at(api_base: &str, access_token: &str) -> Result<EntitlementCache, String> {
    check_api_base(api_base)?;
    let mut res = agent()
        .get(&format!("{}/entitlement", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {access_token}"))
        .call()
        .map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    if !(200..300).contains(&status) {
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        return Err(provider_error(status, &v));
    }
    serde_json::from_str(&text).map_err(|e| format!("the entitlement reply could not be read ({e})"))
}

/// One refresh: token, call, cache. Every failure leaves the previous cache exactly where it was —
/// which is what makes the grace a grace and not a countdown that a bad network can restart.
pub fn refresh_entitlement(vault: &std::path::Path, data_dir: &std::path::Path) -> Result<EntitlementCache, String> {
    let cfg = cloud_config(vault)?;
    let auth = auth_base(&cfg.api_base)?;
    let token = valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, now_unix())?;
    let c = fetch_entitlement_at(&cfg.api_base, &token)?;
    save_cache(data_dir, &c)?;
    Ok(c)
}

// ---- C1 Task 18: an install that already exists is adopted in place (spec §11a) ----

/// A vault written before C1 has no `config/cloud.yaml`. Spec §11a: it is **adopted in place** — the
/// folder, the settings, the Credential Manager entries and the profile id all stay exactly where
/// they are, and the account is added to what is already there.
///
/// A cloud.yaml that IS there and does not parse also answers `true` here, and that is deliberate:
/// the overlay comes up, [`attach_in`] refuses it by name (*this vault already has an account*), and
/// the student reads a sentence about a broken file. The alternative — treating unreadable as
/// absent — would silently overwrite the account of a vault that has one. `scheduler`'s
/// [`EntitlementState::Unreadable`] is where that distinction earns its keep instead.
pub fn needs_account(vault: &std::path::Path) -> bool { cloud_config(vault).is_err() }

/// Write the account into a vault that already exists, and move the session onto its profile. The
/// two halves are one operation on purpose: a `cloud.yaml` naming a credential target that holds
/// nothing is a vault that cannot reach the cloud with nothing anywhere saying why.
pub fn attach_in(vault: &std::path::Path, profile_id: &str, pending_target: &str) -> Result<(), String> {
    let (account_id, _) = load_session(pending_target)?;
    attach_config_and_session(vault, profile_id, &account_id, pending_target, &move_session)?;

    // **The back-fill** (Interfaces with C2, item 3). This vault was onboarded before the account
    // existed, so its LMS feed is in `config/ingest.yaml` and in no `sources` row — and C2's
    // `/ingest/ics` reads that row. Send it once, now that there is an account to send it to.
    //
    // **Best effort, and never fatal**: the adoption has already succeeded, the vault copy is what
    // `ingest` reads until C2 ships, and an unsubscribed or offline account must not leave a
    // half-adopted install behind. A failure is one logged line.
    for (kind, url) in feeds_in(vault) {
        if let Err(e) = crate::lms_link::store_source(&api_base(), &session_target(profile_id), kind, &url) {
            eprintln!("Knowlu: the {kind} link could not be saved to your account ({e})");
        }
    }
    Ok(())
}

/// The two halves that have to land together: `config/cloud.yaml`, then the session onto the
/// profile. **If the move fails the file goes back** (R-C1-57, I4) — otherwise `needs_account` is
/// false forever over a vault whose credential target holds nothing, and the upgrade overlay, which
/// is the console's only sign-in surface, never comes back to offer a retry. That is precisely the
/// state [`attach_in`]'s own doc says this design prevents; this is what makes it true.
///
/// **The move is a parameter** for the same reason `lms_link::validate_for`'s fetch is one: a
/// `CredWriteW` that fails is not something a test can arrange on a healthy machine, and the
/// rollback is the half worth proving.
pub fn attach_config_and_session(
    vault: &std::path::Path,
    profile_id: &str,
    account_id: &str,
    pending_target: &str,
    move_it: &dyn Fn(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    // **Every field, because `VaultPlan` has grown**: Tasks 14a and 14b added `zybooks_courses`,
    // `vhl_sections`, `zybooks_ignore`, `course_map` and `courses`, and Task 14c added
    // `campus_choice`. An adopted vault gains `config/cloud.yaml` and nothing else — its feeds, its
    // mappings, its courses and its campus are already on disk and are not rewritten — so every plan
    // field but the three cloud ones is empty by construction, and `write_cloud_yaml_if_absent` is
    // the only writer this calls.
    let plan = crate::scaffold::VaultPlan {
        profile_id: profile_id.to_string(),
        ics_url: None,
        personal_calendar: None,
        // An adopted vault's own `calendars:` list is not rewritten here — see the comment above —
        // so this flag plays no part in this call; it is `false` only because `write_cloud_yaml_if_absent`
        // never reads it either.
        google_calendar: false,
        zybooks_courses: Vec::new(),
        vhl_sections: Vec::new(),
        zybooks_ignore: Vec::new(),
        course_map: Vec::new(),
        courses: Vec::new(),
        timezone: String::new(),
        slots: Vec::new(),
        device: String::new(),
        campus: "none".to_string(),
        campus_choice: Default::default(),
        zybooks: false,
        vhl: false,
        api_base: api_base(),
        anon_key: anon_key(),
        account_id: account_id.to_string(),
    };
    // Only `config/cloud.yaml` is written. Nothing else in this vault is read, rewritten or moved.
    crate::scaffold::write_cloud_yaml_if_absent(vault, &plan)?;
    if let Err(e) = move_it(pending_target, &session_target(profile_id)) {
        // The file this call just wrote, and only that file — `config/` and everything else in it
        // was the student's before this ran.
        let _ = std::fs::remove_file(vault.join("config").join("cloud.yaml"));
        return Err(e);
    }
    Ok(())
}

/// Both feeds `config/ingest.yaml` may already hold: `ics_url` (the school) and the first entry of
/// `calendars:` (the personal one). Read the way `scheduler::ics_state` reads the same file —
/// `serde_yaml_ng` over `pystr`, never a byte compare.
fn feeds_in(vault: &std::path::Path) -> Vec<(&'static str, String)> {
    let Ok(text) = knowlu_engine::pystr::read_text(&vault.join("config").join("ingest.yaml")) else { return Vec::new() };
    let Ok(v) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else { return Vec::new() };
    let mut out = Vec::new();
    if let Some(u) = v.get("ics_url").and_then(|u| u.as_str()).filter(|u| !u.trim().is_empty()) {
        out.push(("lms_ics", u.to_string()));
    }
    if let Some(list) = v.get("calendars").and_then(|c| c.as_sequence()) {
        if let Some(u) = list.iter().find_map(|f| f.get("ics_url").and_then(|u| u.as_str())).filter(|u| !u.trim().is_empty()) {
            out.push(("calendar_ics", u.to_string()));
        }
    }
    out
}

/// The shape `profiles::id_for` — `knowlu_engine::ids::derived_id("profile", …)` — always produces:
/// the literal `profile_` and exactly ten hex characters, the first ten of a SHA-1. **A profile id
/// names folders and Credential Manager entries**, so anything else is not a "different id", it is a
/// path fragment somebody typed: `"../.."` under a backups root climbs out of it (R-C1-57, I3).
pub fn is_profile_id(id: &str) -> bool {
    id.strip_prefix("profile_")
        .map(|hex| hex.len() == 10 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or(false)
}

/// The id to use: the stored one when it is really one, and otherwise the id the vault path derives
/// — which is what `profiles::register` filed the profile under and what `Settings::load` defaults
/// to anyway. `settings.json` is a plain file a person can edit and `state::Settings` parses this
/// field as an unchecked `String`, so "stored" and "trustworthy" are not the same thing.
///
/// Pure, and taking the stored value rather than reading it, so `app/tests/account.rs` can drive the
/// refusal without a `ConsoleState` (the shape `scheduler::slot_argv` and `relaunch_args` use).
pub fn profile_id_or_derived(stored: &str, vault: &std::path::Path) -> String {
    if is_profile_id(stored) { stored.to_string() } else { crate::profiles::id_for(vault) }
}

fn profile_id_of(cs: &crate::state::ConsoleState) -> String {
    let stored = cs.settings.lock().map(|s| s.profile_id.clone()).unwrap_or_default();
    profile_id_or_derived(&stored, &cs.vault)
}

#[tauri::command(async)]
pub fn attach_account(cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let profile_id = profile_id_of(&cs);
    match attach_in(&cs.vault, &profile_id, PENDING_TARGET) {
        Ok(()) => {
            // R-C1-31's argument, on this path: the vault now has an account and the session sits
            // where the console looks for it, so this is the first moment the entitlement cache can
            // be written — and without it `judge` is *skipped: no entitlement* until the
            // housekeeping thread's six-hourly refresh comes round. Best effort: a student who has
            // just subscribed on a bad network is adopted either way, and the grace clock starts at
            // the next refresh instead of this one.
            let _ = refresh_entitlement(&cs.vault, &cs.data_dir);
            json!({ "ok": true, "error": Value::Null })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// What the settings panel and the upgrade overlay both read. `needs_account` is what puts the
/// overlay on screen at all.
///
/// `(async)` (R-C1-57, M10): it reads a credential (`CredReadW`) and a file, and every other
/// credential-touching command in this file is off the UI thread for that reason. The brief spelled
/// it as a plain `#[tauri::command]`; the page awaits a promise either way.
#[tauri::command(async)]
pub fn account_status(cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let cfg = cloud_config(&cs.vault).ok();
    let cache = load_cache(&cs.data_dir);
    let email = cfg
        .as_ref()
        .and_then(|c| load_session(&c.session_credential_target).ok())
        .map(|(_, s)| s.email)
        .unwrap_or_default();
    // **No network call here.** `account_status` answers from this machine only — a console that had
    // to reach the internet before it could decide whether to cover today's page would be the bug the
    // 72-hour grace exists to prevent (spec §5.1, D4). Whether the service is reachable is a fact the
    // page learns from an attempt that failed, and it is the page that stands the overlay down.
    json!({
        "ok": true, "error": Value::Null,
        "needs_account": cfg.is_none(),
        "account_id": cfg.as_ref().map(|c| c.account_id.clone()),
        "email": email,
        "status": cache.as_ref().map(|c| c.status.clone()),
        "plan": cache.as_ref().and_then(|c| c.plan.clone()),
        "current_period_end": cache.as_ref().and_then(|c| c.current_period_end.clone()),
        "checked_at": cache.as_ref().map(|c| c.checked_at.clone()),
    })
}

/// The wizard's poll (`entitlement_now`) reads the PENDING session, because the wizard has no vault
/// yet; the upgrade overlay uses the same one for the same reason — it signs in before it attaches,
/// so until `attach_account` runs there is no profile session either.
#[tauri::command(async)]
pub fn entitlement_now() -> Value {
    let base = api_base();
    let out = (|| -> Result<EntitlementCache, String> {
        let auth = auth_base(&base)?;
        let token = valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, now_unix())?;
        fetch_entitlement_at(&base, &token)
    })();
    match out {
        Ok(c) => json!({ "ok": true, "error": Value::Null, "status": c.status, "plan": c.plan, "current_period_end": c.current_period_end }),
        Err(e) => json!({ "ok": false, "error": e, "status": Value::Null }),
    }
}

// `open_in_browser` is already in this file — Task 10 added it for `open_policy`, and Checkout and
// the Portal use the same one rather than a second copy of the same three lines.

/// The PKCE half of `/token`. GoTrue's `PKCEGrantParams` names `auth_code` and `code_verifier`
/// (`internal/api/token.go`) — not `code`, not `verifier` — and either one empty is a 400 with a
/// sentence about both being non-empty.
pub fn exchange_pkce_at(
    auth_base: &str,
    anon: &str,
    code: &str,
    verifier: &str,
    now_unix: i64,
) -> Result<(String, Session), String> {
    let body = json!({ "auth_code": code, "code_verifier": verifier });
    let (status, v) = post_json(&format!("{auth_base}/token?grant_type=pkce"), anon, &body)?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    session_from(&v, now_unix)
}

/// **One authenticated POST**, returning the status and the parsed body. `post_for_url` and
/// `post_no_reply` are its two readings: a route that answers with a link to open, and a route whose
/// success is the status itself. Split out for review M5 — `record_consent_at` used to detect
/// success by string-matching `"no link came back"`, a literal private to `post_for_url` and free to
/// be reworded, which would have turned every recorded consent into a silent failure.
fn post_authed(api_base: &str, path: &str, token: &str, body: &Value) -> Result<(u16, Value), String> {
    check_api_base(api_base)?;
    let text = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let mut res = agent()
        .post(&format!("{}{path}", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .header("content-type", "application/json")
        .send(text)
        .map_err(|e| format!("{UNREACHABLE} ({e})"))?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// A route whose success has no link in it. (`post_for_url` keeps its own signature and its own
/// `"no link came back"`; both now go through [`post_authed`].)
fn post_no_reply(api_base: &str, path: &str, token: &str, body: &Value) -> Result<(), String> {
    let (status, v) = post_authed(api_base, path, token, body)?;
    if (200..300).contains(&status) { Ok(()) } else { Err(provider_error(status, &v)) }
}

/// The 18+ attestation and the two policy versions, recorded after the session exists.
///
/// **Why it is a second call and not metadata.** `/authorize` has no field for user metadata, and
/// migration `20260917000100` stopped the trigger reading metadata on either path — a `/otp` request
/// is anyone's to send, so an attestation taken from one would be an `age_18` row nobody made. This
/// is the only writer of a consent row; `billing-checkout` refuses an account where `age_attested_at`
/// is still null, so skipping this call is not a way around the gate.
///
/// Called after **both** sign-in paths — `google_sign_in` and `verify_email_code` — and idempotent on
/// the service side, so it needs no "is this a new account" question the app has no honest way to
/// answer, and `open_checkout` can retry it for free.
pub fn record_consent_at(api_base: &str, token: &str, tos: &str, privacy: &str) -> Result<(), String> {
    let body = json!({ "tos_version": tos, "privacy_version": privacy, "age_attested": true });
    post_no_reply(api_base, "/account/consent", token, &body)
}

/// One POST that answers with a link to open. **The body is serialized and sent as a plain string**,
/// not through `send_json`: ureq's `json` feature is not enabled in `app/Cargo.toml` (it carries the
/// engine's `cookies` feature and no other), so `send_json` does not exist on this build —
/// `post_json` above and `lms_link::put_source_at` both document the same decision.
fn post_for_url(api_base: &str, path: &str, token: &str, body: &Value) -> Result<String, String> {
    check_api_base(api_base)?;
    let text = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let mut res = agent()
        .post(&format!("{}{path}", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .header("content-type", "application/json")
        .send(text)
        .map_err(|e| format!("{UNREACHABLE} ({e})"))?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    v.get("url").and_then(|x| x.as_str()).map(str::to_string).ok_or_else(|| "no link came back".to_string())
}

/// The checkout link, with the consent retry in front of it (review I4). Split from the command so
/// the order — consent, then Stripe — is a test rather than a claim; the command is what opens the
/// browser.
pub fn checkout_url_at(api_base: &str, token: &str, plan: &str) -> Result<String, String> {
    // Free when the first call worked (the route writes only where the account has none), and the
    // difference between a student who can subscribe and one whose only advice is "sign in again"
    // when it did not. A retry that fails again is logged, never fatal: `billing-checkout`'s 403 is
    // the honest answer and the server is the one entitled to give it.
    if let Err(e) = record_consent_at(api_base, token, TOS_VERSION, PRIVACY_VERSION) {
        eprintln!("Knowlu: the consent retry before checkout did not land ({e})");
    }
    post_for_url(api_base, "/billing-checkout", token, &json!({ "plan": plan, "terms_version": TOS_VERSION }))
}

/// **Continue with Google** (spec D1, D2). One loopback listener, one browser window, one exchange.
///
/// Vault-less, like `google_connect_url` beside it: the session goes to [`PENDING_TARGET`], which is
/// where `create_vault_in` looks for it at Finish and where the upgrade overlay's `attach_account`
/// looks for it over an existing vault. Every panel after this one is unchanged, because it cannot
/// tell this path from the emailed code.
///
/// **Nothing here reaches the page.** The verifier, the code and both tokens stay in this function
/// and in Credential Manager; what crosses the IPC is an account id and an email address, the same
/// envelope `verify_email_code` returns.
#[tauri::command(async)]
pub fn google_sign_in() -> Value {
    let api = api_base();
    let out = (|| -> Result<(String, Session), String> {
        let auth = auth_base(&api)?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| format!("no local port for the sign-in ({e})"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let (verifier, challenge) = pkce_pair();
        open_in_browser(&authorize_url(&auth, port, &challenge))?;
        // Three minutes: long enough to pick an account and read a consent screen, short enough
        // that a wizard nobody came back to is not holding a socket at bedtime.
        let code = serve_one_callback(listener, std::time::Duration::from_secs(180))?;
        let (id, s) = exchange_pkce_at(&auth, &anon_key(), &code, &verifier, now_unix())?;
        save_session(PENDING_TARGET, &id, &s)?;
        // Best effort, and second — but never silent (review I4). The session is already on this
        // machine and an attestation that did not land is a subscribe step that says so, not a
        // sign-in to be redone; `open_checkout` retries it before the checkout POST. The log line is
        // `attach_in`'s shape, this file's existing way of saying "this part did not land".
        if let Err(e) = record_consent_at(&api, &s.access_token, TOS_VERSION, PRIVACY_VERSION) {
            eprintln!("Knowlu: the sign-up consent could not be recorded ({e})");
        }
        Ok((id, s))
    })();
    match out {
        Ok((id, s)) => ok_account(&id, &s.email),
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}

#[tauri::command(async)]
pub fn open_checkout(plan: String) -> Value {
    let base = api_base();
    let out = (|| -> Result<String, String> {
        let auth = auth_base(&base)?;
        let token = valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, now_unix())?;
        let url = checkout_url_at(&base, &token, &plan)?;
        open_in_browser(&url)?;
        Ok(url)
    })();
    match out { Ok(_) => json!({ "ok": true, "error": Value::Null }), Err(e) => json!({ "ok": false, "error": e }) }
}

/// *Manage subscription* — one click to Stripe's portal, one click to cancel there, no survey in
/// between. That is the whole of the cancel flow the legal note (§8) asks for.
#[tauri::command(async)]
pub fn open_portal(cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let out = (|| -> Result<String, String> {
        let cfg = cloud_config(&cs.vault)?;
        let auth = auth_base(&cfg.api_base)?;
        let token = valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, now_unix())?;
        let url = post_for_url(&cfg.api_base, "/billing-portal", &token, &json!({}))?;
        open_in_browser(&url)?;
        Ok(url)
    })();
    match out { Ok(_) => json!({ "ok": true, "error": Value::Null }), Err(e) => json!({ "ok": false, "error": e }) }
}

/// The local half of *Delete my data*, with no `AppHandle` and no network anywhere near it — so
/// `app/tests/account.rs` can drive it over scratch folders and prove the list is exactly this long.
/// **Everything it removes is named here and nowhere else**: the vault folder, THIS profile's
/// subtree of the backups root, this profile's app-data folder, this profile's row in
/// `profiles.json`, its session credential and its two coursework logins.
///
/// **`backup_root` is the ROOT** — `%USERPROFILE%\Knowlu\Backups` for every profile on the machine —
/// and `backup::tick` writes `<root>\<profile_id>\` inside it. Removing the root would destroy a
/// housemate's or a second profile's only other copy of their work, which is not what *delete MY
/// data* says.
///
/// **So the id is checked before it names anything** (R-C1-57, I3): an id that is not
/// [`is_profile_id`]-shaped — empty, `"../.."`, anything with a separator in it — removes nothing
/// keyed to an id. Not the backups subtree, not the registry row, not a credential. The vault and
/// the app-data folder are paths the CALLER named and are still removed, because those two are what
/// *delete my data* is about; `delete_my_data` gets its id from `profile_id_of`, which has already
/// fallen back to the vault's own derivation, so in the real flow this guard never fires.
///
/// Best effort throughout: a file held open by another process must not stop the rest of the wipe,
/// and there is nothing a user could do with the error anyway — the account is already deleted by
/// the time this runs.
pub fn delete_local_data(
    vault: &std::path::Path,
    data_dir: &std::path::Path,
    backup_root: Option<&std::path::Path>,
    profile_id: &str,
    app_root: Option<&std::path::Path>,
    session_credential_target: &str,
) {
    let _ = std::fs::remove_dir_all(vault);
    let _ = std::fs::remove_dir_all(data_dir);
    if !session_credential_target.is_empty() {
        let _ = crate::credentials::delete(session_credential_target);
    }
    if !is_profile_id(profile_id) {
        eprintln!("Knowlu: {profile_id:?} is not a profile id, so nothing keyed to one was removed");
        return;
    }
    if let Some(b) = backup_root {
        let _ = std::fs::remove_dir_all(b.join(profile_id));
    }
    // …and forget the profile, or the picker goes on offering a vault that is not there. A registry
    // that cannot be READ is left alone rather than rewritten — `profiles::load`'s own rule, and the
    // reason it distinguishes absent from unreadable.
    if let Some(root) = app_root {
        if let Ok(all) = crate::profiles::load(root) {
            let left: Vec<_> = all.into_iter().filter(|pr| pr.id != profile_id).collect();
            let _ = crate::profiles::save(root, &left);
        }
    }
    // The coursework logins are keyed to the PROFILE, not the account (`credentials::target_for`),
    // and they are this machine's — deleting the account does not delete them, so this does.
    for source in ["zybooks", "vhl"] {
        let t = crate::credentials::target_for(profile_id, source);
        if crate::credentials::exists(&t) { let _ = crate::credentials::delete(&t); }
    }
}

/// Spec §4.1: *Delete my data* removes the vault, the snapshots, the app data **and** calls
/// `DELETE /account`. The server call goes FIRST: a local wipe that ran before it would leave an
/// account nobody can reach to delete, and the deletion right is the one that matters here.
///
/// **A refusal is also a stop**, not only a transport failure: the agent is built with
/// `http_status_as_error(false)`, so a 401 or a 500 arrives as `Ok`, and wiping the machine on one
/// would destroy the vault while the account it was supposed to delete is still there. Nothing local
/// is touched unless the server answered 2xx — `cloud/…/account/handler.ts` answers
/// `200 {deleted:true}` — **or 404** (R-C1-57, M1): an account that is already gone is the one
/// refusal that must not lock a user out of deleting their own machine's copy, which is otherwise
/// exactly what a half-finished first attempt would do.
#[tauri::command(async)]
pub fn delete_my_data(app: tauri::AppHandle, cs: tauri::State<'_, crate::state::ConsoleState>) -> Value {
    let cfg = match cloud_config(&cs.vault) { Ok(c) => c, Err(e) => return json!({ "ok": false, "error": e }) };
    let auth = match auth_base(&cfg.api_base) { Ok(a) => a, Err(e) => return json!({ "ok": false, "error": e }) };
    let token = match valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, now_unix()) { Ok(t) => t, Err(e) => return json!({ "ok": false, "error": e }) };
    let untouched = "nothing on this machine was touched";
    let res = agent()
        .delete(&format!("{}/account", cfg.api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .call();
    let mut res = match res {
        Ok(r) => r,
        Err(e) => return json!({ "ok": false, "error": format!("your account could not be deleted ({e}) — {untouched}") }),
    };
    let status = res.status().as_u16();
    if !(200..300).contains(&status) && status != 404 {
        let text = res.body_mut().with_config().limit(1 << 16).read_to_string().unwrap_or_default();
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        return json!({ "ok": false, "error": format!("your account could not be deleted ({}) — {untouched}", provider_error(status, &v)) });
    }
    let backup = cs.settings.lock().map(|s| s.backup_dir.clone()).unwrap_or_default();
    let profile_id = profile_id_of(&cs);
    delete_local_data(
        &cs.vault,
        &cs.data_dir,
        backup.as_deref(),
        &profile_id,
        crate::state::app_data_root().as_deref(),
        &cfg.session_credential_target,
    );
    // The envelope goes back first and the process ends a moment later, on another thread: a page
    // whose `.then` never runs cannot say "Deleted", and the settings row's own copy promises it will.
    let h = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        h.exit(0);
    });
    json!({ "ok": true, "error": Value::Null })
}

/// The Google consent URL for the account this wizard signed in as (§11a).
///
/// **Vault-less on purpose.** `#wiz-google` is on the wizard window, which has no `ConsoleState`
/// (C1: "no command that needs one can be called" there), so the session comes from
/// `PENDING_TARGET` — the same pre-vault target `sign_in` writes and `create_vault_in` later moves
/// onto the profile — and the base URLs come from `api_base()` / `anon_key()`, never from a
/// `config/cloud.yaml` that does not exist yet.
///
/// `scope` is `"calendar"` (the default, and the wizard's) or `"gmail"` (the later, incremental
/// ask). The service decides which Google scope each means and whether the consent widens an
/// existing grant; this command carries the session bearer and nothing else, and **never sees a
/// Google token** — the exchange happens server-side in `google-callback` (D12).
#[tauri::command(async)]
pub fn google_connect_url(scope: String) -> Value {
    let scope = if scope == "gmail" { "gmail" } else { "calendar" };
    let api = api_base();
    let auth = match auth_base(&api) {
        Ok(auth) => auth,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let token = match valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, jiff::Timestamp::now().as_second()) {
        Ok(token) => token,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    match get_json(&format!("{api}/google-connect?scope={scope}"), &token) {
        Ok(v) => match v.get("url").and_then(Value::as_str) {
            Some(url) => json!({ "ok": true, "url": url }),
            None => json!({ "ok": false, "error": "the service returned no url" }),
        },
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// Has the consent landed yet, and what did Google actually grant?
///
/// The calendar panel polls this after opening the consent page — the browser window closes itself
/// and there is nothing else to tell the wizard the round trip finished. `GET /google-connect?status=1`
/// answers `{connected, scopes}` from `google_accounts`, and the panel keys on the calendar scope
/// specifically, because a student can untick one on the consent screen.
#[tauri::command(async)]
pub fn google_connected() -> Value {
    let api = api_base();
    let auth = match auth_base(&api) {
        Ok(auth) => auth,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let token = match valid_access_token_at(&auth, &anon_key(), PENDING_TARGET, jiff::Timestamp::now().as_second()) {
        Ok(token) => token,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    match get_json(&format!("{api}/google-connect?status=1"), &token) {
        Ok(v) => {
            let scopes: Vec<String> = v
                .get("scopes")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            json!({
                "ok": true,
                "connected": v.get("connected").and_then(Value::as_bool).unwrap_or(false),
                "calendar": scopes.iter().any(|s| s == "https://www.googleapis.com/auth/calendar.readonly"),
                "gmail": scopes.iter().any(|s| s == "https://www.googleapis.com/auth/gmail.readonly"),
            })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// A-6: what the wizard's Google error line says for a failed status — pulled out as a pure
/// function (no network, no agent) so the mapping is tested directly rather than only through a
/// live `get_json` call. `get_json` below is reached by exactly two commands (`google_connect_url`,
/// `google_connected`), so this is specific to the Google flow on purpose: 401 means the wizard's
/// own pending session has gone stale (the fix is a sign-in, not a retry of THIS request), and 503
/// means the deployment has no Google client configured at all (`GOOGLE_NOT_CONFIGURED` on the
/// service side) — the fix is the `calendar_ics` fallback the panel already shows, not "try again".
/// Every other status keeps the generic form, which names the code but nothing more specific.
fn google_error_for_status(code: u16) -> String {
    match code {
        401 => "sign in again".to_string(),
        503 => "Google sign-in is not available right now — use the secret address below".to_string(),
        _ => format!("the service refused (HTTP {code})"),
    }
}

/// One bearer GET against the functions base. `check_api_base` is applied first, so an
/// `KNOWLU_API_BASE` pointing anywhere but https (or loopback, for the tests) is refused here
/// rather than turned into a request to a host nobody chose.
fn get_json(url: &str, token: &str) -> Result<Value, String> {
    check_api_base(url)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("apikey", anon_key())
        .call()
        .map_err(|e| format!("no network ({e})"))?;
    let code = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();
    if !(200..300).contains(&code) {
        // The status, not the body: this string reaches the wizard's error line.
        return Err(google_error_for_status(code));
    }
    serde_json::from_str(&body).map_err(|_| "the service returned no JSON".to_string())
}

/// The whole allow-list, as a pure predicate (ruling R-C2-E32): `https://accounts.google.com/` and
/// nothing else, no CR/LF, under 2048 chars. Split out of `open_external` so it is a plain function
/// a unit test can drive with no spawn and no browser — `starts_with` alone is the guard, and it
/// works precisely because it demands the literal `/` right after the host: a lookalike host like
/// `accounts.google.com.evil.example` fails at that character, never reaching the real prefix.
fn external_url_allowed(url: &str) -> bool {
    url.starts_with("https://accounts.google.com/") && !url.contains('\n') && !url.contains('\r') && url.len() < 2048
}

/// Open a URL in the system browser — the same `open_in_browser` mechanism `open_policy` and
/// `open_checkout` already use, exposed once so the wizard's Google step needs no third private
/// path. It joins the **wizard** window's list beside the two commands above.
///
/// **`https://accounts.google.com/` and nothing else.** This command takes a URL from the page, and
/// the page takes it from the service; one that opened anything would be one indirection away from
/// opening a `file:` URL or a phishing page if either the service or the page were ever wrong.
/// There is exactly one thing it is for, and the CR/LF guard is there because a header-shaped
/// injection into a URL that reaches `explorer.exe` is the other way this goes wrong.
#[tauri::command(async)]
pub fn open_external(url: String) -> Value {
    if !external_url_allowed(&url) {
        return json!({ "ok": false, "error": "only the Google consent page may be opened" });
    }
    match open_in_browser(&url) {
        Ok(()) => json!({ "ok": true }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

#[cfg(test)]
mod google_error_for_status_tests {
    use super::google_error_for_status;

    #[test]
    fn a_401_says_sign_in_again() {
        assert_eq!(google_error_for_status(401), "sign in again");
    }

    #[test]
    fn a_503_names_the_fallback_rather_than_asking_for_a_retry() {
        assert_eq!(
            google_error_for_status(503),
            "Google sign-in is not available right now — use the secret address below"
        );
    }

    #[test]
    fn every_other_status_keeps_the_generic_form() {
        assert_eq!(google_error_for_status(500), "the service refused (HTTP 500)");
        assert_eq!(google_error_for_status(429), "the service refused (HTTP 429)");
        assert_eq!(google_error_for_status(404), "the service refused (HTTP 404)");
    }
}

#[cfg(test)]
mod external_url_allowed_tests {
    use super::external_url_allowed;

    #[test]
    fn only_the_real_https_google_consent_prefix_is_allowed() {
        assert!(external_url_allowed("https://accounts.google.com/o/oauth2/v2/auth?x=1"));
    }

    #[test]
    fn plain_http_is_refused() {
        assert!(!external_url_allowed("http://accounts.google.com/x"));
    }

    #[test]
    fn a_lookalike_host_is_refused() {
        // `starts_with` alone is the guard: the literal prefix demands a `/` right where a
        // lookalike host puts a `.`, so `accounts.google.com.evil.example` never matches.
        assert!(!external_url_allowed("https://accounts.google.com.evil.example/"));
    }

    #[test]
    fn the_real_host_smuggled_after_an_evil_one_is_refused() {
        assert!(!external_url_allowed("https://evil.example/https://accounts.google.com/"));
    }

    #[test]
    fn an_embedded_crlf_is_refused() {
        assert!(!external_url_allowed("https://accounts.google.com/\r\nSet-Cookie: x"));
    }

    #[test]
    fn an_oversized_url_is_refused() {
        let padding = "a".repeat(2048);
        let url = format!("https://accounts.google.com/{padding}");
        assert!(url.len() >= 2048);
        assert!(!external_url_allowed(&url));
    }

    #[test]
    fn the_2048_boundary_is_exclusive() {
        let prefix = "https://accounts.google.com/";
        let at_2047 = format!("{prefix}{}", "a".repeat(2047 - prefix.len()));
        let at_2048 = format!("{prefix}{}", "a".repeat(2048 - prefix.len()));
        assert_eq!(at_2047.len(), 2047);
        assert_eq!(at_2048.len(), 2048);
        assert!(external_url_allowed(&at_2047), "2047 chars is still under 2048");
        assert!(!external_url_allowed(&at_2048), "2048 chars is not under 2048");
    }
}
