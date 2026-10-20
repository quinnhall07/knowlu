//! The entitlement gate (cloud design, amendment 2026-09-17, ruling 3).
//!
//! **Why the engine and not only the app.** Quinn's reason, on the record: the attacker is a person
//! repurposing this code to get its full utility without paying, or a competitor shipping a free
//! copy. With the fetch sequence, the parsers, the rules and every judgment server-side and
//! entitlement-gated, an orphaned binary ranks a hand-made folder and nothing else. The app's gate
//! is a UI decision; this one is the binary's.
//!
//! **It reads the app's cache and never the network.** `GET /entitlement` is the app's call, made at
//! launch and every six hours; an engine that made it too would race the app's own session refresh
//! and could spend a token the app is about to rotate. So this module is a file and a clock: the
//! same file, the same 72-hour grace, the same arithmetic — pinned to `app/src/account.rs` by a test
//! that reads both, because two constants that must agree and cannot be linked have to be checked.
//!
//! **A gate is a named line at exit 0, never a failure.** A student whose card expired is not a
//! machine with a problem, and retrying fixes nothing; `main.rs` prints the line and returns
//! SUCCESS, so the tray stays calm and the Runs view says what happened.
//!
//! **What is gated: five commands,** `coursework`, `grades`, `ingest`, `judge` and `sync`, the steps
//! that fill the folder (`main.rs::gated_vault`; `grades`, M1 spec §6, stops with `coursework`).
//! `engine/tests/entitlement_gate.rs` spawns the built binary once for each of the five.
//!
//! **What is not gated:** `surface` and `write` (the console reads and edits through them — gating
//! either freezes the window rather than the subscription), `runs`, `info`, `issues`,
//! `coursework-discover`, and — pending precondition P5 — `rank`.

use std::path::{Path, PathBuf};

/// Spec §5.1, and `app/src/account.rs::GRACE`. Seventy-two hours, so a weekend of bad wifi never
/// stops a slot for a student who is paying.
pub const GRACE_SECONDS: i64 = 72 * 60 * 60;

/// The two fields of the app's cache this decision reads. `current_period_end` and `plan` are in the
/// file and are deliberately not here: a decision that read them would be a second implementation of
/// Stripe's own rules on the wrong side of the wire.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Cached {
    pub status: String,
    pub checked_at: String,
}

/// `knowlu/<profile_id>/session` → `profile_id`. Exactly three segments, and the middle one must be
/// the app's own profile-id shape (fix round 1, M4): a target of another shape, or a middle segment
/// that is not really an id, is a vault this engine does not understand, and guessing would mean
/// reading some other profile's cache — or, for `..` or a path separator, a path fragment somebody
/// typed rather than an id at all.
pub fn profile_id(target: &str) -> Option<&str> {
    let mut parts = target.split('/');
    let (Some(first), Some(id), Some(last), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    if first != "knowlu" || last != "session" { return None; }
    is_profile_id(id).then_some(id)
}

/// The exact shape `profiles::id_for` always produces (`app/src/account.rs::is_profile_id`,
/// R-C1-57 I3, copied here because the two crates cannot share a function any more than they can
/// share the grace constant): the literal `profile_` and exactly ten hex characters — either case,
/// since both sides test `is_ascii_hexdigit` (`profiles::id_for` itself only ever writes lowercase).
fn is_profile_id(id: &str) -> bool {
    id.strip_prefix("profile_")
        .map(|hex| hex.len() == 10 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or(false)
}

/// `app/src/state.rs::app_data_root_in`'s path, and the seam every test uses.
pub fn app_data_root_in(base: &Path) -> PathBuf { base.join("knowlu") }

pub fn cache_path_in(base: &Path, profile: &str) -> PathBuf {
    app_data_root_in(base).join("profiles").join(profile).join("entitlement.json")
}

/// Mirrors the two fields of `app/src/profiles.rs::Profile` this module needs — read straight off
/// `profiles.json`, never through the app crate, since the engine does not depend on `knowlu`
/// (`dependency_boundary.rs`) and must not start. The rest of that struct's fields are read past.
#[derive(Debug, Clone, serde::Deserialize)]
struct ProfileEntry {
    id: String,
    vault: PathBuf,
}

/// The profile id for this vault, resolved **exactly the way the app resolves it** (fix round 1,
/// I1): the registry entry whose `vault` equals this one, and only when none matches, the id
/// `config/cloud.yaml`'s `session_credential_target` names.
///
/// **Why the registry wins.** A vault's folder can move — a USB stick that comes back under a new
/// drive letter, a restore to a new path — and `profiles::id_for` derives a profile id from the
/// path string itself, so a moved vault is re-registered under a *fresh* id the moment the app next
/// opens it. Every slot after that refreshes that new id's `entitlement.json`; the old id, the one
/// `cloud.yaml` was written with at onboarding and never rewrites, is never touched again. Reading
/// `cloud.yaml`'s id first would silently read a cache the app has stopped writing to, forever, for
/// every student whose vault ever moves.
///
/// **The fallback is not weaker, only later-resorted-to.** A vault the app has never opened — a
/// hand-typed `knowlu-engine.exe` run against a freshly-onboarded vault before the console's first
/// launch has registered it — has no matching entry at all, and every fixture and every other test
/// in this file relies on exactly that: the plain `cloud.yaml`-derived id, unchanged from before
/// this fix.
fn resolve_profile_id(base: &Path, vault: &Path, cfg_target: &str) -> Option<String> {
    let registry = app_data_root_in(base).join("profiles.json");
    if let Ok(text) = std::fs::read_to_string(&registry) {
        if let Ok(entries) = serde_json::from_str::<Vec<ProfileEntry>>(&text) {
            if let Some(p) = entries.into_iter().find(|p| p.vault.as_path() == vault && is_profile_id(&p.id)) {
                return Some(p.id);
            }
        }
    }
    profile_id(cfg_target).map(str::to_string)
}

/// `app/src/account.rs::decide`'s arithmetic, for the two states this side needs.
pub fn decide(cache: Option<&Cached>, now: jiff::Timestamp) -> bool {
    let Some(c) = cache else { return false };
    if c.status != "active" && c.status != "trialing" { return false; }
    let Ok(checked) = c.checked_at.parse::<jiff::Timestamp>() else { return false };
    let mut age = now.as_second() - checked.as_second();
    // A clock a few minutes fast must not disentitle someone forever — each refresh would otherwise
    // write another "future" `checked_at` and the state would never recover on its own.
    if age < 0 && -age <= 3600 { age = 0; }
    age >= 0 && age <= GRACE_SECONDS
}

/// The gate, with its two inputs injected. `None` means "run".
pub fn gate_in(base: &Path, vault: &Path, now: jiff::Timestamp) -> Option<String> {
    // A vault with no account is not gated: it is a hand-made folder, it has no cloud step to run,
    // and every fixture vault in this repository is one.
    let cfg = crate::cloudmodel::load(vault)?;
    let profile = resolve_profile_id(base, vault, &cfg.session_credential_target)?;
    let cached = std::fs::read_to_string(cache_path_in(base, &profile))
        .ok()
        .and_then(|t| serde_json::from_str::<Cached>(&t).ok());
    if decide(cached.as_ref(), now) { return None; }
    Some("skipped: no entitlement".to_string())
}

/// The production entry point. `LOCALAPPDATA` unset is **not** a refusal: an engine run outside a
/// user session has no app data to read and gating it would turn a missing environment variable into
/// a licence check, which is a different thing from the one ruling 3 asked for.
pub fn gate(vault: &Path) -> Option<String> {
    let base = std::env::var("LOCALAPPDATA").ok().filter(|s| !s.is_empty())?;
    gate_in(Path::new(&base), vault, jiff::Timestamp::now())
}
