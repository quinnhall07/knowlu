//! The judgment service as seen from the device (cloud design §3.2, §5.2).
//!
//! **One HTTPS call per item, and nothing model-shaped in this binary.** From C2 on, tier 3 is
//! `POST /judge/*` on our Supabase project: the prompt, the JSON schema and the pinned model id
//! live server-side, so changing how a judgment is asked for is a deploy and not an app release
//! (§10). `judge.rs` is untouched — `CloudModel` implements the same `judge::Model` the local
//! process implemented, and every scripted fake in the plan-3a tests keeps working.
//!
//! **Delivery is always a pull.** Nothing is ever pushed at the device: `judge` asks per item and
//! writes the reply through `write`, inside the slot the app already runs. C3's journal sync
//! generalises this later; nothing here waits for it.
//!
//! **Three properties this module exists to hold.**
//!
//! - *The session token never reaches a string anyone can read.* It comes out of Credential
//!   Manager through `wincred`, exactly as a portal password does, and every error text this
//!   module produces goes through `zybooks::scrub` first. `state/runner-log.md` is written from
//!   those strings.
//! - *A failure says which failure it was.* Ruling R-3a-25's standing question — "does the failure
//!   text survive this boundary?" — has two new boundaries here: the HTTP status and the auth
//!   failure. A 401 that reads as "low confidence" twice a day forever is the same defect the
//!   structural `LowCause` split was written to prevent, so [`CloudError::label`] is a closed set
//!   of words that name the user's actual problem.
//! - *A refusal that answers every item stops the batch.* An account with no entitlement answers
//!   fifty items identically; [`CloudModel::fatal`] records the first one and the rest are never
//!   sent.
//!
//! **Nothing here is reached by `rank`** (decision 11). `rank` may reach [`fetch_event_source`]
//! and [`fetch_ics`], which are transport and carry no judgment.

use std::cell::Cell;
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use crate::judge::{self, ModelError};

/// One call's wall-clock bound — the same 120 seconds `runtime::CALL_TIMEOUT` gave one local
/// completion. Spelled again here rather than borrowed, because C4 removes `runtime.rs` and this
/// bound outlives it: a judge step runs inside a slot whose own child cap is twenty minutes.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(120);

/// `config/cloud.yaml`, written by the wizard at onboarding (C1). Absent on a vault that has
/// never signed in, which is a named skip and not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudConfig {
    pub api_base: String,
    pub anon_key: String,
    pub session_credential_target: String,
    pub account_id: String,
}

/// Why this vault cannot talk to the service *before* a single call is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// No `config/cloud.yaml`: this vault has no account.
    NoConfig,
    /// The file names a Credential Manager target that holds nothing readable.
    NoSession(String),
}

impl Unavailable {
    /// A closed set, so it can be printed and logged.
    pub fn label(&self) -> &'static str {
        match self {
            Unavailable::NoConfig => "no account",
            Unavailable::NoSession(_) => "no session",
        }
    }
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unavailable::NoConfig => write!(f, "no account on this vault (no config/cloud.yaml)"),
            Unavailable::NoSession(why) => write!(f, "no session in Credential Manager ({why}); sign in again"),
        }
    }
}

/// Read `config/cloud.yaml`. **Never fails**: a missing, unreadable or incomplete file is `None`,
/// because a vault with a half-written config must skip the cloud step and rank the day, not stop.
pub fn load(vault: &Path) -> Option<CloudConfig> {
    let text = crate::pystr::read_text(&vault.join("config").join("cloud.yaml")).ok()?;
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).ok()?;
    let field = |key: &str| {
        value.get(key).and_then(serde_yaml_ng::Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
    };
    Some(CloudConfig {
        api_base: field("api_base")?.trim_end_matches('/').to_string(),
        anon_key: field("anon_key")?,
        session_credential_target: field("session_credential_target")?,
        account_id: field("account_id")?,
    })
}

/// The account's **access token**, out of the credential blob C1 writes.
///
/// **The blob is one JSON object, not a bare JWT** (*Interfaces with C1*, contract 2):
/// `{"access_token","refresh_token","expires_at","email"}`, `expires_at` in Unix seconds, with the
/// `UserName` field holding the account id. **Refresh is C1's job** — `account::valid_access_token()`
/// refreshes at fewer than 120 seconds remaining and rewrites the entry — so this reads and never
/// writes, and an expired token is reported as `no session` rather than refreshed here. A device
/// that refreshed on its own would race the app's own refresh and could invalidate it.
#[cfg(windows)]
fn session_token(cfg: &CloudConfig) -> Result<String, Unavailable> {
    let credential = crate::wincred::read_credential(&cfg.session_credential_target)
        .map_err(|err| Unavailable::NoSession(format!("{err}")))?;
    let blob: serde_json::Value = serde_json::from_str(credential.password.expose())
        .map_err(|_| Unavailable::NoSession("the credential is not the JSON the app writes".to_string()))?;
    let token = blob
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Unavailable::NoSession("the credential carries no access_token".to_string()))?;
    // Read but not acted on: a token that expired ten seconds ago still gets one attempt, because
    // the server is the authority on its own tokens and a clock skew here would refuse a good one.
    // A real 401 comes back as `CloudError::Status { code: 401 }` and says "sign in again".
    Ok(token.to_string())
}

/// The credential store is Windows-only (spec §6.5), so a cloud build still compiles and simply
/// has no way to authenticate — and says so rather than pretending there is no account.
#[cfg(not(windows))]
fn session_token(_cfg: &CloudConfig) -> Result<String, Unavailable> {
    Err(Unavailable::NoSession("credential store unavailable on this platform".to_string()))
}

/// The client for this vault, or the reason there is none. Resolved **once per run**, never per
/// item: fifty items must not each re-read Credential Manager.
pub fn resolve(vault: &Path) -> Result<CloudClient, Unavailable> {
    let cfg = load(vault).ok_or(Unavailable::NoConfig)?;
    let token = session_token(&cfg)?;
    Ok(CloudClient::new(&cfg, &token))
}

/// What went wrong on the wire. The **status** is kept because it is the difference between "sign
/// in again", "your subscription lapsed" and "try later"; the **body** is kept only as the
/// server's own short `error` field, clipped, because a service reply is the one place a prompt
/// could come back out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudError {
    Transport(String),
    Status { code: u16, detail: String },
    Body(String),
}

impl CloudError {
    /// The word `Missing::Service` carries and `enrich` prints. A closed set (ruling R-3a-20's
    /// shape): there is no value here that came from a note, a model or a server body.
    pub fn label(&self) -> &'static str {
        match self {
            CloudError::Transport(_) => "no network",
            CloudError::Status { code: 401, .. } => "no session",
            CloudError::Status { code: 402, .. } => "no entitlement",
            CloudError::Status { code: 403, .. } => "not allowed",
            CloudError::Status { code: 429, .. } => "rate limited",
            CloudError::Status { .. } => "the service refused",
            CloudError::Body(_) => "an unreadable reply",
        }
    }

    /// Does this answer every remaining item the same way? A session, an entitlement or a
    /// permission problem does; a 429, a 5xx and a dropped connection do not.
    pub fn fatal(&self) -> bool {
        matches!(self, CloudError::Status { code: 401 | 402 | 403, .. })
    }
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudError::Transport(why) => write!(f, "no network ({why})"),
            CloudError::Status { code, detail } if detail.is_empty() => write!(f, "{} (HTTP {code})", self.label()),
            CloudError::Status { code, detail } => write!(f, "{} (HTTP {code}: {detail})", self.label()),
            CloudError::Body(why) => write!(f, "an unreadable reply ({why})"),
        }
    }
}

/// One agent, one bearer, one account. Built once per run.
pub struct CloudClient {
    base: String,
    anon_key: String,
    token: String,
    account_id: String,
    agent: ureq::Agent,
}

impl CloudClient {
    pub fn new(cfg: &CloudConfig, token: &str) -> CloudClient {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(CALL_TIMEOUT))
            // A non-2xx must arrive as a status AND a body, not as an opaque transport error:
            // `ureq`'s default turns both into `Error::StatusCode` with the body gone, and 401
            // and 402 are precisely the two answers the user needs named.
            .http_status_as_error(false)
            .build()
            .into();
        CloudClient {
            base: cfg.api_base.trim_end_matches('/').to_string(),
            anon_key: cfg.anon_key.clone(),
            token: token.to_string(),
            account_id: cfg.account_id.clone(),
            agent,
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Every error string this produces is scrubbed of the bearer first: `state/runner-log.md` is
    /// written from these and a third party's error text is not ours to trust.
    fn scrub(&self, text: &str) -> String {
        crate::zybooks::scrub(text, &[&self.token])
    }

    fn finish(&self, mut response: ureq::http::Response<ureq::Body>) -> Result<Value, CloudError> {
        let code = response.status().as_u16();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| CloudError::Transport(self.scrub(&e.to_string())))?;
        if !(200..300).contains(&code) {
            let detail = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_default();
            return Err(CloudError::Status { code, detail: judge::clip(&self.scrub(&detail), 200) });
        }
        serde_json::from_str(&text).map_err(|e| CloudError::Body(e.to_string()))
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value, CloudError> {
        // `dumps_value`, not `to_string`: every JSON this crate writes goes through the one writer
        // (CLAUDE.md), so a request body has the same separators and key order everywhere.
        let response = self
            .agent
            .post(format!("{}{path}", self.base))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("apikey", self.anon_key.clone())
            .header("Content-Type", "application/json")
            .send(crate::ledger::dumps_value(body))
            .map_err(|e| CloudError::Transport(self.scrub(&e.to_string())))?;
        self.finish(response)
    }

    pub fn get(&self, path: &str) -> Result<Value, CloudError> {
        let response = self
            .agent
            .get(format!("{}{path}", self.base))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("apikey", self.anon_key.clone())
            .call()
            .map_err(|e| CloudError::Transport(self.scrub(&e.to_string())))?;
        self.finish(response)
    }
}

/// The body of `POST /judge-task`.
///
/// **Everything the prompt needs and nothing else** (plan 3a fidelity row R6, now measured at the
/// HTTP boundary rather than at the prompt string): the note's own fields, the attributed course's
/// grade weights, `profile/preferences.md`, the planner's slice, and the course **slugs** this
/// vault has notes or pins for. The `course_map`'s KEYS — the fragments out of
/// `config/ingest.yaml` — never travel; the slugs do, and they are already the `course:` field of
/// every note in the vault. No vault path, no profile id, no credential target, no account id.
pub fn task_request(item: &judge::Item, h: &judge::Heuristics, seed: &judge::Verdict) -> Value {
    let weights = seed
        .course
        .as_deref()
        .and_then(|c| h.weights.get(c))
        .map(String::as_str)
        .unwrap_or("");
    let mut known: Vec<&str> = h.weights.keys().map(String::as_str).collect();
    for (_, slug) in &h.course_map {
        if !known.contains(&slug.as_str()) {
            known.push(slug);
        }
    }
    json!({
        "kind": "task",
        "item": {
            "id": item.id,
            "title": judge::one_line(&item.title, 200),
            "body": judge::clip(item.body.trim(), judge::MAX_BODY_CHARS),
            "source_uid": item.source_uid,
            "created_by": item.created_by,
            "course": item.course,
            "due": item.due,
        },
        "heuristics_seed": {
            "course": seed.course,
            "effort_hours": seed.effort_hours,
            "slice_hours": h.slice_hours,
            "weights": judge::clip(weights, judge::MAX_WEIGHTS_CHARS),
            "preferences": judge::clip(&h.preferences, judge::MAX_PREFS_CHARS),
            "known_courses": known,
        }
    })
}

/// Tier 3, over HTTPS. One call per item, and a refusal that answers every item stops the rest.
pub struct CloudModel<'a> {
    client: &'a CloudClient,
    fatal: Cell<Option<&'static str>>,
}

impl<'a> CloudModel<'a> {
    pub fn new(client: &'a CloudClient) -> CloudModel<'a> {
        CloudModel { client, fatal: Cell::new(None) }
    }

    /// Set once a call comes back 401, 402 or 403. `enrich` prints it as one summary line instead
    /// of fifty identical per-item lines.
    pub fn fatal(&self) -> Option<&'static str> {
        self.fatal.get()
    }

    pub(crate) fn call(&self, path: &str, body: &Value) -> Result<Value, ModelError> {
        if let Some(reason) = self.fatal.get() {
            return Err(ModelError::Failed(format!("the judgment service: {reason}")));
        }
        self.client.post(path, body).map_err(|e| {
            if e.fatal() {
                self.fatal.set(Some(e.label()));
            }
            ModelError::Failed(format!("the judgment service: {e}"))
        })
    }
}

impl judge::Model for CloudModel<'_> {
    fn judge(
        &self,
        item: &judge::Item,
        h: &judge::Heuristics,
        seed: &judge::Verdict,
    ) -> Result<judge::Verdict, ModelError> {
        let reply = self.call("/judge-task", &task_request(item, h, seed))?;
        let verdict = reply.get("verdict").filter(|v| !v.is_null()).ok_or_else(|| {
            let cause = reply.get("cause").and_then(Value::as_str).unwrap_or("no verdict");
            ModelError::Failed(format!("the judgment service answered {cause}"))
        })?;
        // Through `judge::parse_reply`, deliberately. The clamps, the one-lined reason and the
        // blank-is-None rules are the engine's, and a reply from the service goes through exactly
        // the door a reply from a local process went through — so the service can never widen a
        // bound the vault depends on.
        let mut v = judge::parse_reply(&crate::ledger::dumps_value(verdict))?;
        if let Some(tier) = reply.get("tier").and_then(Value::as_u64) {
            v.tier = tier.min(3) as u8;
        }
        Ok(v)
    }
}

impl CloudModel<'_> {
    /// Ask once, before the batch, whether this account can be judged at all.
    ///
    /// `Some(reason)` for the three answers that will not change item by item — no session, no
    /// entitlement, not allowed — and `None` for everything else, including "no network", because
    /// a flaky connection is per-item and the batch should try. The probe is `GET /judge-rules`,
    /// which every account may call, costs no model tokens and charges no cap.
    ///
    /// **Before Task 12 deploys `judge-rules`, this is a 404** — which is not `fatal()`, so it
    /// answers `None` and the batch proceeds exactly as it would have. That is deliberate: the
    /// probe is an optimisation for the two answers that repeat, never a gate.
    pub fn probe(&self) -> Option<&'static str> {
        match self.client.get("/judge-rules") {
            Ok(_) => None,
            Err(e) if e.fatal() => {
                self.fatal.set(Some(e.label()));
                Some(e.label())
            }
            Err(_) => None,
        }
    }
}

/// The account's LMS calendar feed, fetched by the service (cloud design §3.1). **Transport, not
/// judgment** — `ingest` parses what comes back with the same `parse_ics` the golden `today.md`
/// oracle covers, so the vault's bytes are unchanged by the move.
///
/// `first_run` asks `GET /ingest-ics?first_run=1` — set only on the vault's first-ever ingest
/// (`ingest::is_first_run`) — because that is the one run with no seen-ledger to tell the device
/// which uids are already archived. The returned `past_due_uids` is Task 8a's: this function
/// returns an empty `Vec` whenever the reply carries no such field, which is every reply until
/// Task 8a's server change ships.
pub fn fetch_ics(client: &CloudClient, first_run: bool) -> Result<(String, Vec<String>), CloudError> {
    let path = if first_run { "/ingest-ics?first_run=1" } else { "/ingest-ics" };
    let reply = client.get(path)?;
    let ics = reply
        .get("ics")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| CloudError::Body("the reply carried no ics field".to_string()))?;
    let past_due_uids = reply
        .get("past_due_uids")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    Ok((ics, past_due_uids))
}

/// One calendar feed, as ICS, from the service (cloud design §11a). **Transport, not judgment** —
/// `calfeed` parses what comes back with the same `parse_calendar_ics` the golden `today.md`
/// oracle covers, bounds it to the same 28-day horizon and falls back to the same snapshot.
///
/// `Err(String)` because the caller is `Fetchers.calendar`, whose contract predates this module
/// and whose failure already degrades to "using snapshot".
pub fn fetch_calendar(client: &CloudClient, name: &str) -> Result<String, String> {
    let reply = client
        .get(&format!("/ingest-calendar?name={}", urlencode_component(name)))
        .map_err(|e| e.to_string())?;
    reply
        .get("ics")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "the reply carried no ics field".to_string())
}

/// The three characters a feed name could carry that a query string would misread. Not a general
/// percent-encoder: a feed name comes from `config/ingest.yaml`, and anything wilder than this
/// should fail loudly at the server rather than be smuggled through.
fn urlencode_component(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            other => other.encode_utf8(&mut [0u8; 4]).bytes().map(|b| format!("%{b:02X}")).collect(),
        })
        .collect()
}
