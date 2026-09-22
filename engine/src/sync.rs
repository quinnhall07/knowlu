//! Journal sync (cloud design §5.5, as amended 2026-09-17): the account's own copy of its own vault.
//!
//! **What leaves this machine.** Every journal record this device writes, as `ledger::dumps_value`
//! wrote it, and the whole text of every note that changed. The account is the source of truth for
//! that data (amendment ruling 2) and this folder is a mirror of it; our service can read both, and
//! says so on the privacy page in the words Quinn and the lawyer approved. What does **not** leave
//! is a portal password, a session token in a log, an app-data path or a machine name: the `device`
//! column is `sha256(account_id + "\n" + hostname)` truncated to sixteen hex characters, which is an
//! identity for "the same machine as last time" and nothing else.
//!
//! **What this module is not.** It is transport, never judgment: `rank` does not reach it and
//! neither does anything under `cli.rs` except the `sync` subcommand itself. Nothing here calls a
//! model. And it never deletes a local note because the account does not have one — the mirror is
//! filled by the account, never emptied by it.
//!
//! **Every failure is a named line and exit 0.** No account, no session, no entitlement, no network,
//! a 402, a 5xx: the day still ranks from the folder on disk, because a slot step that exits
//! non-zero is retry backoff and an amber tray twice a day for ever.
//!
//! **Delivery is a pull, and the pull carries one thing.** `/sync-pull` returns records **another
//! desktop of this account** pushed. The service's own writes — judgment fields, Gmail-derived
//! notes, event verdicts, rule proposals — are delivered by C2, per item, inside the slot step that
//! asks for them, and are journalled on the device that asked; they then go up through this module
//! like any other record. The fidelity ledger's `§5.5 Down` row argues it.

use ring::digest;

/// The largest journal record this device will send, in bytes, matching
/// `sync_records.body`'s `octet_length` check. A record is a handful of scalars and, for a
/// `create`, one frontmatter mapping; 16 KiB is far past any that exists. A record over it is
/// reported by name and left in the journal, never truncated — a truncated record is a lie.
pub const MAX_RECORD_BYTES: usize = 16 * 1024;

/// The largest note text this device will send, in bytes, matching `sync_notes.body`'s check. A
/// note longer than 128 KiB is a pasted document, not a task; it stays in the vault and the push
/// says so in one line.
pub const MAX_NOTE_BYTES: usize = 128 * 1024;

/// Everything this module can refuse. **A closed set of words** (ruling R-3a-20's shape): no value
/// here came from a note or a server body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// No `config/cloud.yaml`: this vault has no account.
    NoAccount,
    /// There is an account but no usable session: the student is signed out on this machine.
    NoSession(String),
    /// The service refused or could not be reached. Carries `CloudError`'s own sentence, which is
    /// already scrubbed of the bearer.
    Service(String),
    /// A pulled row is not what it claims to be.
    Shape(&'static str),
    /// The vault, or the cursor file.
    Io(String),
}

impl SyncError {
    /// The word the `sync` step prints and the page shows. Never a path, never a body.
    pub fn label(&self) -> &'static str {
        match self {
            SyncError::NoAccount => "no account",
            SyncError::NoSession(_) => "no session",
            SyncError::Service(_) => "the service refused",
            SyncError::Shape(_) => "an unreadable row",
            SyncError::Io(_) => "the vault could not be read",
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NoSession(why) | SyncError::Service(why) | SyncError::Io(why) => {
                write!(f, "{} ({why})", self.label())
            }
            SyncError::Shape(what) => write!(f, "{} ({what})", self.label()),
            other => write!(f, "{}", other.label()),
        }
    }
}

/// Lowercase hex of `SHA-256(bytes)`. The one hash this module computes, and the one the server
/// re-derives: `sync-push` refuses a row whose `hash` is not this of its `body`, so the idempotence
/// key is a fact about the row rather than a claim about it.
///
/// `ring`, not `sha2`: this binary already links `ring` for TLS, so the primitive is free, and the
/// alternative would put a second `digest` major beside `sha1`'s (`dependency_boundary.rs` argues it
/// at length and refuses the alternative by name).
pub fn sha256_hex(bytes: &[u8]) -> String {
    digest::digest(&digest::SHA256, bytes).as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Which machine pushed a row, to a server that has no reason to learn which machine that is.
///
/// The hostname is in the mix so two desktops of one account differ; the account id is in the mix so
/// the same hostname under two accounts does not collide; sixteen hex characters is enough to
/// separate the handful of desktops one student owns and short enough that it is obviously not a
/// name. `journal::device_name()` is the same value every journal record already carries **inside**
/// the row, where the student can read it — this is the outside, where nobody needs to.
pub fn device_token(account_id: &str) -> String {
    let mut input = String::with_capacity(account_id.len() + 1 + 32);
    input.push_str(account_id);
    input.push('\n');
    input.push_str(&crate::journal::device_name());
    sha256_hex(input.as_bytes())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_content_hash_is_sha256_of_the_canonical_bytes() {
        // The empty-string vector, so a reader can check it against any SHA-256 implementation.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(sha256_hex(b"abc").len(), 64);
    }

    #[test]
    fn a_device_token_is_sixteen_hex_stable_and_not_the_hostname() {
        let a = device_token("acct-1");
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(a, device_token("acct-1"), "stable across calls");
        assert_ne!(a, device_token("acct-2"), "and scoped to the account");
        let host = crate::journal::device_name().to_lowercase();
        assert!(!host.is_empty() && !a.contains(&host), "the hostname is not in the token");
    }

    #[test]
    fn a_refusal_never_carries_a_path_or_a_body() {
        let e = SyncError::Io("C:\\Users\\someone\\Knowlu\\Vault".to_string());
        assert_eq!(e.label(), "the vault could not be read");
        // The LABEL is what a line prints; `Display` is what a report may carry, and it is allowed
        // the cause. What matters is that the two are different functions and the step prints the
        // first — `run_lines_with` is where that is asserted (Task 7).
        assert!(!e.label().contains("Users"));
    }
}
