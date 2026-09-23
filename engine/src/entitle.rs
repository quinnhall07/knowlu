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

/// `knowlu/<profile_id>/session` → `profile_id`. Exactly three segments, all non-empty: a target of
/// another shape is a vault this engine does not understand, and guessing would mean reading some
/// other profile's cache.
pub fn profile_id(target: &str) -> Option<&str> {
    let mut parts = target.split('/');
    let (Some(first), Some(id), Some(last), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    if first != "knowlu" || id.is_empty() || last != "session" { return None; }
    Some(id)
}

/// `app/src/state.rs::app_data_root_in`'s path, and the seam every test uses.
pub fn app_data_root_in(base: &Path) -> PathBuf { base.join("knowlu") }

pub fn cache_path_in(base: &Path, profile: &str) -> PathBuf {
    app_data_root_in(base).join("profiles").join(profile).join("entitlement.json")
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
    let profile = profile_id(&cfg.session_credential_target)?;
    let cached = std::fs::read_to_string(cache_path_in(base, profile))
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
