//! The app's client for the service's `mail` routes (email-forwarding plan T11.1, P15; spec §6.3).
//!
//! One method per route, each a rustls `ureq` call that carries the session bearer and answers a
//! `serde_json::Value` or a [`MailError`]. **Transport only**: nothing here decides anything about
//! mail, and the app's mail code reads these answers and does the rest.
//!
//! **The session** is read on every call through a token function, never kept: [`MailClient::for_session`]
//! reads it through `account::valid_access_token_at` (which refreshes a stale token), so a long setup
//! never sends one that expired. `account.rs` is not edited.
//!
//! **Failures are named** ([`MailError::name`]): no session, sign in again (401), lapsed (402 on a
//! route that needs an entitlement; the list and the delete never do, so a canceled account can always see
//! and disconnect its mailboxes, test 16b), not found, refused (the other 4xx, with the service's own
//! sentence), a server error, a timeout, unreachable, and an unreadable reply.
//!
//! **Privacy.** No error string, `Display` or `Debug`, holds the bearer, a mailbox address, a hint, a URL
//! or a transport error's text (which can carry a URL). A refusal carries the service's sentence only when
//! it holds no `@` and not the bearer: the caps and windows the student reads are sentences of fixed
//! words, and anything else is dropped to "". Ids and the provider's domain are checked before a request
//! is built, so nothing can inject a path.
use crate::account::{auth_base, check_api_base, valid_access_token_at};
use serde_json::{json, Value};
use std::time::Duration;

/// A hung service ends the call here, as a named outcome; the setup runner's steps are longer.
const TIMEOUT: Duration = Duration::from_secs(30);
/// The largest reply read: a reconcile of a long window answers every held fingerprint.
const MAX_REPLY: u64 = 8 << 20;

/// What a call can end as. Every variant is a sentence the student can be shown; none holds a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailError {
    /// No usable session on this device, or no valid `api_base`: nothing was sent.
    NoSession,
    /// 401: the session was refused; sign in again.
    SignIn,
    /// 402 on a route that needs an entitlement: the subscription lapsed (§4.11).
    Lapsed,
    /// 404: no such mailbox, batch or route for this account.
    NotFound,
    /// Any other 4xx: the service's refusal, with its sentence when it was safe to keep.
    Refused { status: u16, reason: String },
    /// 5xx.
    Server(u16),
    /// The call took longer than the client's timeout.
    TimedOut,
    /// The service could not be reached at all.
    Unreachable,
    /// The service answered 2xx with something that is not the JSON this route returns.
    BadReply,
    /// The request was never built: an id or a domain that could not be part of a route.
    Invalid,
}

impl MailError {
    /// The stable name tests and the setup runner branch on.
    pub fn name(&self) -> &'static str {
        match self {
            MailError::NoSession => "no-session",
            MailError::SignIn => "sign-in",
            MailError::Lapsed => "lapsed",
            MailError::NotFound => "not-found",
            MailError::Refused { .. } => "refused",
            MailError::Server(_) => "server",
            MailError::TimedOut => "timed-out",
            MailError::Unreachable => "unreachable",
            MailError::BadReply => "bad-reply",
            MailError::Invalid => "invalid",
        }
    }
}

impl std::fmt::Display for MailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailError::NoSession => f.write_str("there is no signed-in session on this device"),
            MailError::SignIn => f.write_str("the account service did not accept this sign-in; sign in again"),
            MailError::Lapsed => f.write_str("this account has no active subscription"),
            MailError::NotFound => f.write_str("the account service has no such mailbox"),
            MailError::Refused { status, reason } if reason.is_empty() => {
                write!(f, "the account service refused the request ({status})")
            }
            MailError::Refused { reason, .. } => f.write_str(reason),
            MailError::Server(status) => write!(f, "the account service failed ({status}); try again"),
            MailError::TimedOut => f.write_str("the account service did not answer in time"),
            MailError::Unreachable => f.write_str("the account service could not be reached"),
            MailError::BadReply => f.write_str("the account service's reply could not be read"),
            MailError::Invalid => f.write_str("the request could not be made"),
        }
    }
}

impl std::error::Error for MailError {}

/// D3's caps and the hint route's refusal, as `mail/handler.ts` sends them: the device shows these
/// sentences as they are, and matches a refusal by its status and sentence, never by a guess.
pub const LIVE_CAP: &str = "this account already has three live mail addresses";
pub const MINT_CAP: &str = "this account has made six mail addresses in the last 30 days";
pub const HINT_HELD: &str = "another of this account's mailboxes has this address";

impl MailError {
    /// 409 with [`HINT_HELD`]: another of the account's mailboxes already holds this mailbox's address.
    pub fn hint_held(&self) -> bool {
        matches!(self, MailError::Refused { status: 409, reason } if reason == HINT_HELD)
    }

    /// One fixed sentence for the progress view: a refusal whose status and sentence the service is known
    /// to send is that sentence; anything else is this client's own words for the failure.
    pub fn fixed_sentence(&self) -> &'static str {
        match self {
            MailError::Refused { status: 409, reason } if reason == LIVE_CAP => LIVE_CAP,
            MailError::Refused { status: 429, reason } if reason == MINT_CAP => MINT_CAP,
            MailError::Refused { status: 409, reason } if reason == HINT_HELD => HINT_HELD,
            MailError::Refused { status: 422, reason } if reason == HINT_MISFIT => HINT_MISFIT,
            MailError::Refused { status: 409, reason } if reason == HINT_SET => HINT_SET,
            MailError::Refused { .. } => "the account service refused the request",
            MailError::NoSession => "there is no signed-in session on this device",
            MailError::SignIn => "the account service did not accept this sign-in; sign in again",
            MailError::Lapsed => "this account has no active subscription",
            MailError::NotFound => "the account service has no such mailbox",
            MailError::Server(_) => "the account service failed; try again",
            MailError::TimedOut => "the account service did not answer in time",
            MailError::Unreachable => "the account service could not be reached",
            MailError::BadReply => "the account service's reply could not be read",
            MailError::Invalid => "the request could not be made",
        }
    }
}

/// The hint route's refusal of an address that does not fit the mailbox's provider (`mail/handler.ts`).
const HINT_MISFIT: &str = "this address does not fit the mailbox's provider";
/// The hint route's refusal when this mailbox's own address is already set (`mail/handler.ts` `ALREADY`).
const HINT_SET: &str = "this mailbox's own address is already set";

/// A backfill batch opens a window on a date; a recover batch on an instant (spec §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Backfill,
    Recover,
}

/// What `POST /addresses/<id>/state` takes (§4.11 step 2, P21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Off,
    Elsewhere,
    Reenabled,
    Confirming,
}

type TokenFn = Box<dyn Fn() -> Result<String, String> + Send + Sync>;

pub struct MailClient {
    base: String,
    token: TokenFn,
    timeout: Duration,
}

/// Hand-written: the base names the service and the token function reaches the session.
impl std::fmt::Debug for MailClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailClient").field("base", &"<redacted>").field("token", &"<redacted>").finish()
    }
}

/// An id the service minted (a UUID) or a domain: letters, digits, `-` and `.` only, so it cannot
/// change which route is called.
fn route_safe(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        && !s.starts_with('.')
        && !s.contains("..")
}

impl MailClient {
    /// A client whose bearer comes from `token` on every call. `api_base` is checked as the account
    /// code checks it: `https://`, or the test loopback `http://127.0.0.1:`.
    pub fn with_token_fn(api_base: &str, token: TokenFn) -> Result<MailClient, MailError> {
        check_api_base(api_base).map_err(|_| MailError::NoSession)?;
        Ok(MailClient { base: format!("{}/mail", api_base.trim_end_matches('/')), token, timeout: TIMEOUT })
    }

    /// The production client: the session stored at `session_target`, refreshed through `account`'s
    /// reader when it has under two minutes left.
    pub fn for_session(api_base: &str, anon_key: &str, session_target: &str) -> Result<MailClient, MailError> {
        let auth = auth_base(api_base).map_err(|_| MailError::NoSession)?;
        let (anon, target) = (anon_key.to_string(), session_target.to_string());
        let now = || jiff::Timestamp::now().as_second();
        MailClient::with_token_fn(api_base, Box::new(move || valid_access_token_at(&auth, &anon, &target, now())))
    }

    pub fn with_timeout(mut self, timeout: Duration) -> MailClient {
        self.timeout = timeout;
        self
    }

    // ---- the routes (spec §6.3) --------------------------------------------------------------

    /// `GET /addresses`: the account's mailboxes. A session is enough (a lapsed account can list).
    pub fn list_addresses(&self) -> Result<Vec<Value>, MailError> {
        let v = self.call("GET", "/addresses", None, false)?;
        match v.get("addresses") {
            Some(Value::Array(a)) => Ok(a.clone()),
            _ => Err(MailError::BadReply),
        }
    }

    /// `POST /addresses {provider}`: mints a mailbox's address (`m365`, `google` or `other`).
    pub fn mint_address(&self, provider: &str) -> Result<Value, MailError> {
        self.call("POST", "/addresses", Some(json!({ "provider": provider })), true)
    }

    /// `POST /addresses/<id>/hint {hint}`: the mailbox's own address, set once.
    pub fn set_hint(&self, id: &str, hint: &str) -> Result<Value, MailError> {
        self.call("POST", &format!("/addresses/{}/hint", self.id(id)?), Some(json!({ "hint": hint })), true)
    }

    /// `POST /addresses/<id>/rotate`.
    pub fn rotate_address(&self, id: &str) -> Result<Value, MailError> {
        self.call("POST", &format!("/addresses/{}/rotate", self.id(id)?), None, true)
    }

    /// `DELETE /addresses/<id>`: a session is enough (a lapsed account can disconnect).
    pub fn delete_address(&self, id: &str) -> Result<Value, MailError> {
        self.call("DELETE", &format!("/addresses/{}", self.id(id)?), None, false)
    }

    /// `POST /canary {address_id}`.
    pub fn canary(&self, address_id: &str) -> Result<Value, MailError> {
        self.call("POST", "/canary", Some(json!({ "address_id": self.id(address_id)? })), true)
    }

    /// `POST /batches`: opens a backfill (`since` a `YYYY-MM-DD` date) or a recover (an instant) batch
    /// and answers its nonce, the only time it leaves the service.
    pub fn open_batch(&self, address_id: &str, phase: Phase, since: &str, expected: u32) -> Result<Value, MailError> {
        let phase = match phase {
            Phase::Backfill => "backfill",
            Phase::Recover => "recover",
        };
        let body = json!({ "address_id": self.id(address_id)?, "phase": phase, "since": since, "expected": expected });
        self.call("POST", "/batches", Some(body), true)
    }

    /// `POST /batches/<id>/sent {sent}`: the device's count, once.
    pub fn batch_sent(&self, batch_id: &str, sent: u32) -> Result<Value, MailError> {
        self.call("POST", &format!("/batches/{}/sent", self.id(batch_id)?), Some(json!({ "sent": sent })), true)
    }

    /// `POST /reconcile {address_id, since, until}`: the held fingerprints in the window and the key.
    /// The body is the window and nothing about any message (§4.11 step 3).
    pub fn reconcile(&self, address_id: &str, since: &str, until: &str) -> Result<Value, MailError> {
        let body = json!({ "address_id": self.id(address_id)?, "since": since, "until": until });
        self.call("POST", "/reconcile", Some(body), true)
    }

    /// `POST /addresses/<id>/state {state}`.
    pub fn set_state(&self, id: &str, state: State) -> Result<Value, MailError> {
        let state = match state {
            State::Off => "off",
            State::Elsewhere => "elsewhere",
            State::Reenabled => "reenabled",
            State::Confirming => "confirming",
        };
        self.call("POST", &format!("/addresses/{}/state", self.id(id)?), Some(json!({ "state": state })), true)
    }

    /// `GET /provider?domain=`: the MX fallback when no curated row names the school's provider (PS).
    pub fn provider(&self, domain: &str) -> Result<Value, MailError> {
        if !route_safe(domain, 253) {
            return Err(MailError::Invalid);
        }
        self.call("GET", &format!("/provider?domain={domain}"), None, true)
    }

    // ---- the one call ------------------------------------------------------------------------

    fn id<'a>(&self, id: &'a str) -> Result<&'a str, MailError> {
        if route_safe(id, 64) { Ok(id) } else { Err(MailError::Invalid) }
    }

    /// One request. `entitled` is whether the route needs an active subscription: a 402 reads as
    /// lapsed only there.
    fn call(&self, method: &str, path: &str, body: Option<Value>, entitled: bool) -> Result<Value, MailError> {
        let token = (self.token)().map_err(|_| MailError::NoSession)?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.timeout))
            // FALSE: the service says why it refused in the body of a 4xx.
            .http_status_as_error(false)
            .build()
            .into();
        let url = format!("{}{path}", self.base);
        let auth = format!("Bearer {token}");
        let sent = match (method, body) {
            ("GET", _) => agent.get(&url).header("authorization", &auth).call(),
            ("DELETE", _) => agent.delete(&url).header("authorization", &auth).call(),
            (_, Some(b)) => agent
                .post(&url)
                .header("authorization", &auth)
                .header("content-type", "application/json")
                .send(b.to_string().as_bytes()),
            (_, None) => agent.post(&url).header("authorization", &auth).send_empty(),
        };
        let mut res = sent.map_err(transport)?;
        let status = res.status().as_u16();
        let text = res
            .body_mut()
            .with_config()
            .limit(MAX_REPLY)
            .read_to_string()
            .map_err(|e| if is_timeout(&e) { MailError::TimedOut } else { MailError::BadReply })?;
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        match status {
            200..=299 if value.is_object() => Ok(value),
            200..=299 => Err(MailError::BadReply),
            401 => Err(MailError::SignIn),
            402 if entitled => Err(MailError::Lapsed),
            404 => Err(MailError::NotFound),
            500..=599 => Err(MailError::Server(status)),
            _ => Err(MailError::Refused { status, reason: safe_sentence(&value, &token) }),
        }
    }
}

fn is_timeout(e: &ureq::Error) -> bool {
    match e {
        ureq::Error::Timeout(_) => true,
        ureq::Error::Io(io) => matches!(io.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock),
        _ => false,
    }
}

/// A transport failure by name only: ureq's text can carry the URL.
fn transport(e: ureq::Error) -> MailError {
    if is_timeout(&e) { MailError::TimedOut } else { MailError::Unreachable }
}

/// The service's `{"error": "<sentence>"}`, kept only when it is short, holds no `@` (an address or a
/// hint) and not the bearer; otherwise "".
fn safe_sentence(v: &Value, token: &str) -> String {
    let s = v.get("error").and_then(Value::as_str).unwrap_or("");
    if s.len() > 200 || s.contains('@') || (!token.is_empty() && s.contains(token)) {
        return String::new();
    }
    s.to_string()
}
