//! The account: the session, the sign-in, the entitlement cache, and the commands the wizard and the
//! settings panel call (Knowlu C1, Tasks 10, 11 and 17).
//!
//! **Two rules from `credentials.rs` carry over unchanged.** A secret is never logged, never in a run
//! record, a backup, a fixture, a test name or an error message; and the page never sees one — a
//! command takes a password in and gives an envelope back, and the token that comes out of it goes
//! straight into Credential Manager without passing through the webview.
use serde_json::{json, Value};

/// The Supabase project this build talks to. **Both values are public**: Supabase publishes the
/// project URL and the anon key in every client it generates, and neither grants anything on its own
/// — row-level security and the edge functions decide what a caller may do. The service-role key,
/// which does bypass all of that, is a function secret Quinn sets and appears nowhere in this repo.
///
/// Filled from precondition P1 in step 4 below. `KNOWLU_API_BASE` / `KNOWLU_ANON_KEY` override them
/// at run time, which is how a scratch profile is pointed at `knowlu-staging` (§11 R6) without a
/// second build — the same shape `KNOWLU_ENGINE_EXE` already uses.
pub const DEFAULT_API_BASE: &str = "<P1: the knowlu-prod project's functions URL>";
pub const DEFAULT_ANON_KEY: &str = "<P1: the knowlu-prod project's anon key>";

/// The policy versions a sign-up records. Dates, not numbers, because the pages carry a date too and
/// a reader comparing the two should not have to hold a mapping in their head. Bump BOTH the constant
/// and the page's date in the same commit, or the consent log points at text nobody can find.
pub const TOS_VERSION: &str = "2026-09-10";
pub const PRIVACY_VERSION: &str = "2026-09-10";

/// Where a session lives before there is a vault to key it to. The wizard signs in on panel 2 and
/// creates the vault on panel 9, so for those seven panels the profile id does not exist yet — and it
/// is derived from the vault path, which the user can still change. `move_session` walks it over at
/// Finish, exactly as `retarget_credentials` walks the coursework logins (R-P4a-23), and for exactly
/// the same reason: a credential filed under a path nothing will look at is worse than no credential.
pub const PENDING_TARGET: &str = "knowlu/pending/session";

/// GoTrue's `/verify` request body names a `type`. Ruling R-C1-25: precondition P1 (the staging
/// project) does not exist yet, so step 3a — one `curl` against staging to settle whether the pinned
/// GoTrue version wants `"magiclink"` or `"email"` — has not run. `"magiclink"` is written here
/// provisionally, behind this one named constant, so the eventual settle is a one-line change.
// P1: settled by Task 10 step 3a (magiclink | email)
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
/// between would leave the user signed out with no way back but retyping a password they may have
/// generated. `retarget_credentials` makes the same argument about the coursework logins.
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
/// tell "your password is wrong" from "there is no network" — the first is something a user can fix
/// on the panel, the second is what stands the upgrade overlay down instead of trapping someone
/// behind it. Everything else is the provider's own sentence.
///
/// **The body is sent compact, not through `send_json`.** `send_json` needs ureq's `json` feature,
/// which `app/Cargo.toml` does not enable (it carries the engine's `cookies` feature and no other) —
/// but the choice is not free of behaviour either way: `RequestBuilder::send_json` pretty-prints
/// with embedded newlines, and this crate's own loopback tests assert exact substrings like
/// `"type":"magiclink"` and `"password"` against the raw request text, which a pretty-printed body
/// would break. Serializing with `serde_json::to_string` — already a dependency — and sending the
/// compact result as a plain string is **required**, not merely equivalent.
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

/// Email + password, with the three things migration `20260910000100`'s trigger insists on. A sign-up
/// missing any of them is refused **by the database**, so sending them is not politeness.
pub fn sign_up_at(
    auth_base: &str,
    anon: &str,
    email: &str,
    password: &str,
    tos_version: &str,
    privacy_version: &str,
    now_unix: i64,
) -> Result<(String, Session), String> {
    let body = json!({
        "email": email,
        "password": password,
        "data": { "age_attested": "true", "tos_version": tos_version, "privacy_version": privacy_version },
    });
    let (status, v) = post_json(&format!("{auth_base}/signup"), anon, &body)?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    // With email confirmation on, a sign-up returns the user and NO session until the link is
    // clicked. GoTrue did not refuse anything — this `Err` is not reporting a fault, it is the one
    // channel this function has for handing the wizard a sentence to show while it waits; the
    // caller reads the string, not the variant, so "check your email" travels the same path a real
    // failure would without being one.
    if v.get("access_token").is_none() {
        return Err("check your email and click the link, then sign in".to_string());
    }
    session_from(&v, now_unix)
}

pub fn sign_in_at(auth_base: &str, anon: &str, email: &str, password: &str, now_unix: i64) -> Result<(String, Session), String> {
    let (status, v) = post_json(&format!("{auth_base}/token?grant_type=password"), anon, &json!({ "email": email, "password": password }))?;
    if !(200..300).contains(&status) { return Err(provider_error(status, &v)); }
    session_from(&v, now_unix)
}

/// A magic link never returns a session — it sends mail. Success is "we sent it", nothing more.
pub fn magic_link_at(auth_base: &str, anon: &str, email: &str) -> Result<(), String> {
    let (status, v) = post_json(&format!("{auth_base}/otp"), anon, &json!({ "email": email, "create_user": false }))?;
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
/// The `type` is the one thing here that is not certain across GoTrue versions — [`VERIFY_TYPE`]
/// records why it is provisional; if step 3a's eventual answer differs, that constant is the only
/// line that changes.
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

/// Create the account. **The three acceptances are Rust's, not the page's**: the versions are
/// constants here and the attestation is refused here, so a page that forgot a checkbox cannot make
/// an account that the consent log then describes wrongly.
#[tauri::command(async)]
pub fn sign_up(email: String, password: String, age_attested: bool) -> Value {
    if !age_attested {
        return json!({ "ok": false, "error": "Knowlu is for people 18 or older.", "account_id": Value::Null });
    }
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e, "account_id": Value::Null }) };
    match sign_up_at(&auth, &anon, email.trim(), &password, TOS_VERSION, PRIVACY_VERSION, now_unix()) {
        Ok((id, s)) => match save_session(PENDING_TARGET, &id, &s) {
            Ok(()) => ok_account(&id, &s.email),
            Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
        },
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}

#[tauri::command(async)]
pub fn sign_in(email: String, password: String) -> Value {
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e, "account_id": Value::Null }) };
    match sign_in_at(&auth, &anon, email.trim(), &password, now_unix()) {
        Ok((id, s)) => match save_session(PENDING_TARGET, &id, &s) {
            Ok(()) => ok_account(&id, &s.email),
            Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
        },
        Err(e) => json!({ "ok": false, "error": e, "account_id": Value::Null }),
    }
}

#[tauri::command(async)]
pub fn send_magic_link(email: String) -> Value {
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e }) };
    match magic_link_at(&auth, &anon, email.trim()) {
        Ok(()) => json!({ "ok": true, "error": Value::Null }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// The other half of the magic link: the code from the mail, traded for a session on this machine.
#[tauri::command(async)]
pub fn verify_email_code(email: String, code: String) -> Value {
    let (auth, anon, _) = match env_pair() { Ok(v) => v, Err(e) => return json!({ "ok": false, "error": e, "account_id": Value::Null }) };
    match verify_email_code_at(&auth, &anon, email.trim(), &code, now_unix()) {
        Ok((id, s)) => match save_session(PENDING_TARGET, &id, &s) {
            Ok(()) => ok_account(&id, &s.email),
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

/// Read through `pystr` and `serde_yaml_ng`, never by comparing bytes: a vault's files are whatever
/// Windows made them, and the engine translates line endings on every read for that reason.
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
