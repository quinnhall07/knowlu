//! Journal sync (cloud design §5.5): the account's own encrypted copy of its own vault.
//!
//! **What leaves this machine, and what cannot.** Every journal record and every note's text is
//! sealed here with AES-256-GCM under a 32-byte key that exists in exactly one place — Windows
//! Credential Manager, at `knowlu/<profile_id>/sync-key` — and is never sent, never logged, never
//! in a `Debug`, never in an error string. What travels beside the ciphertext is an opaque 16-hex
//! device token, a 64-hex content MAC, a base64 twelve-byte IV and nothing else: no path, no
//! title, no field name, no note id.
//!
//! **And those three opaque values are KEYED, not bare digests** — `HMAC-SHA256(K_index, …)` with
//! `K_index = HKDF(sync key, "knowlu/index")`. A `SHA-256` of a guessable string *is* the string to
//! anyone holding the table: a vault path is `tasks/<slugified title>.md` over a known alphabet and
//! a journal record's shape is public. Keying them costs nothing and is the difference between "the
//! server cannot read this" and "the server would have to guess". The server stores blobs it cannot
//! open; that is the whole of why `site/privacy.html` can go on saying what it says, and it is why
//! the switch that turns this on has a screen of its own.
//!
//! **Transport, never judgment.** `rank` does not reach this module and neither does anything under
//! `cli.rs` except the `sync` subcommand itself. Nothing here calls a model.
//!
//! **Every failure is a named line and exit 0.** No account, no key, no entitlement, no network, a
//! 402, a 5xx: the day still ranks from the folder on disk, because the folder on disk is the
//! authority and the cloud copy never is (VISION, as amended §10).
//!
//! **Delivery is a pull, and the pull carries one thing.** `/sync-pull` returns records **another
//! device of this account** pushed. The service's own writes — judgment fields, Gmail-derived
//! notes, event verdicts, rule proposals — are delivered by C2, per item, inside the slot step that
//! asks for them, and are journalled on the device that asked; they then go up through this module
//! like any other record. A server-originated row here could not be sealed, because the server does
//! not have the key.

use base64::Engine as _;
use serde_json::Value;

use crate::cloudmodel::CloudConfig;
use crate::ledger::Record;

/// The largest journal record this device will seal. A record is a handful of scalars and, for a
/// `create`, one frontmatter mapping; 16 KiB is far past any that exists. A record over it is
/// reported by name and left in the journal, never truncated — a truncated record is a lie.
pub const MAX_RECORD_BYTES: usize = 16 * 1024;

/// The largest note text this device will seal. A note longer than 128 KiB is a pasted document,
/// not a task; it stays in the vault and the push says so in one line.
pub const MAX_NOTE_BYTES: usize = 128 * 1024;

const B64: base64::engine::general_purpose::GeneralPurpose = base64::engine::general_purpose::STANDARD;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Everything this module can refuse. **A closed set of words** (ruling R-3a-20's shape): no value
/// here came from a note, a server body or a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// No `config/cloud.yaml`: this vault has no account.
    NoAccount,
    /// There is an account but no sync key: the student has not turned sync on.
    NoKey,
    /// A typed recovery code is not 32 bytes.
    BadKey,
    /// A key IS stored and does not decode — hand-edited, or truncated by something, or Credential
    /// Manager refused the read for a reason other than "there is nothing there". Distinct from
    /// `NoKey` on purpose: "not turned on" would hide a real problem behind a normal-looking skip.
    /// Carries the cause, folded into the message below the fixed label — never any key bytes: the
    /// only things that can appear here are a credential target name, a `GetLastError` code, or the
    /// fixed reason string `wincred::CredError::Blob` names.
    KeyUnreadable(String),
    /// **This machine's key is not the one the account's copy is sealed under.** Another device of
    /// the same account turned the switch off and on again and re-uploaded everything under a fresh
    /// key; this machine's key is fine and its vault is fine, and what it has lost is the ability to
    /// read or add to the shared copy. Carries the server's own sentence, which names the account's
    /// current fingerprint — not a secret, and what the student compares against on the machine that
    /// has the key.
    StaleKey(String),
    /// Sealing or opening failed — a tampered row, or the wrong key.
    Crypto(&'static str),
    /// A pulled envelope opened but is not what it claims to be.
    Shape(&'static str),
    /// The vault, or the cursor file.
    Io(String),
}

impl SyncError {
    /// The word the `sync` step prints and the page shows. Never a path, never a body.
    pub fn label(&self) -> &'static str {
        match self {
            SyncError::NoAccount => "no account",
            SyncError::NoKey => "not turned on",
            SyncError::BadKey => "that is not a recovery code",
            SyncError::KeyUnreadable(_) => "the stored key is unreadable; restore with your recovery code",
            SyncError::StaleKey(_) => "this machine's key is not the account's; enter your recovery code in Settings",
            SyncError::Crypto(_) => "the copy could not be opened with this key",
            SyncError::Shape(_) => "an unreadable row",
            SyncError::Io(_) => "the vault could not be read",
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Crypto(where_) => write!(f, "{} ({where_})", self.label()),
            SyncError::Shape(what) => write!(f, "{} ({what})", self.label()),
            SyncError::Io(why) => write!(f, "{} ({why})", self.label()),
            SyncError::KeyUnreadable(cause) if cause.is_empty() => write!(f, "{}", self.label()),
            SyncError::KeyUnreadable(cause) => write!(f, "{} ({cause})", self.label()),
            SyncError::StaleKey(detail) if detail.is_empty() => write!(f, "{}", self.label()),
            SyncError::StaleKey(detail) => write!(f, "{} ({detail})", self.label()),
            other => write!(f, "{}", other.label()),
        }
    }
}

// ---------------------------------------------------------------------------
// The key
// ---------------------------------------------------------------------------

/// The account's sync key: 32 bytes, made on this device, kept in Credential Manager, never sent.
#[derive(Clone, PartialEq, Eq)]
pub struct SyncKey([u8; 32]);

/// Hand-written, for the reason `wincred::Secret` and `account::Session` have theirs: a derived
/// `Debug` is one `{:?}` — a `dbg!`, an `unwrap()` panic, a stray log line — away from putting the
/// one thing that protects a student's whole vault somewhere that is not Credential Manager.
impl std::fmt::Debug for SyncKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncKey").field("key", &"<redacted>").field("fingerprint", &self.fingerprint()).finish()
    }
}

/// Crockford's base32 alphabet: no `I`, `L`, `O` or `U`. A recovery code is read off a screen and
/// typed on a different machine, possibly by someone reading it aloud.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

impl SyncKey {
    pub fn from_bytes(bytes: [u8; 32]) -> SyncKey {
        SyncKey(bytes)
    }

    /// A fresh key from the platform CSPRNG. The only place a key is ever created.
    pub fn generate() -> Result<SyncKey, SyncError> {
        use ring::rand::SecureRandom;
        let mut bytes = [0u8; 32];
        ring::rand::SystemRandom::new().fill(&mut bytes).map_err(|_| SyncError::Crypto("random"))?;
        Ok(SyncKey(bytes))
    }

    pub fn from_base64(text: &str) -> Result<SyncKey, SyncError> {
        let raw = B64.decode(text.trim()).map_err(|_| SyncError::BadKey)?;
        let bytes: [u8; 32] = raw.try_into().map_err(|_| SyncError::BadKey)?;
        Ok(SyncKey(bytes))
    }

    pub fn to_base64(&self) -> String {
        B64.encode(self.0)
    }

    /// The first eight hex characters of `SHA-256(key)`. **Not a secret** and not reversible: it is
    /// what the restore screen shows so a student can tell two keys apart, and what Credential
    /// Manager's `UserName` field carries so the app can say which key a profile holds without
    /// reading the blob.
    pub fn fingerprint(&self) -> String {
        hex(ring::digest::digest(&ring::digest::SHA256, &self.0).as_ref())[..8].to_string()
    }

    /// 256 bits as 52 Crockford characters, in thirteen groups of four. Printed once, on the screen
    /// that turns sync on, and typed once, on the screen that restores.
    pub fn to_recovery_code(&self) -> String {
        let mut bits = 0u32;
        let mut have = 0u32;
        let mut out = String::new();
        for byte in self.0 {
            bits = (bits << 8) | u32::from(byte);
            have += 8;
            while have >= 5 {
                have -= 5;
                out.push(CROCKFORD[((bits >> have) & 0x1f) as usize] as char);
            }
        }
        if have > 0 {
            out.push(CROCKFORD[((bits << (5 - have)) & 0x1f) as usize] as char);
        }
        out.as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap_or_default().to_string())
            .collect::<Vec<String>>()
            .join("-")
    }

    /// The inverse, and deliberately forgiving. Case is ignored; dashes, spaces and tabs are
    /// ignored; `I` and `L` read as `1` and `O` as `0`, which are the three substitutions people
    /// make without noticing. Anything else is `BadKey` — refused by name, never half-decoded.
    pub fn from_recovery_code(code: &str) -> Result<SyncKey, SyncError> {
        let mut bits: u64 = 0;
        let mut have: u32 = 0;
        let mut bytes: Vec<u8> = Vec::with_capacity(32);
        for ch in code.chars() {
            if ch == '-' || ch.is_whitespace() {
                continue;
            }
            let up = ch.to_ascii_uppercase();
            let up = match up {
                'I' | 'L' => '1',
                'O' => '0',
                other => other,
            };
            let Some(index) = CROCKFORD.iter().position(|c| *c as char == up) else {
                return Err(SyncError::BadKey);
            };
            bits = (bits << 5) | index as u64;
            have += 5;
            if have >= 8 {
                have -= 8;
                bytes.push(((bits >> have) & 0xff) as u8);
            }
        }
        let bytes: [u8; 32] = bytes.try_into().map_err(|_| SyncError::BadKey)?;
        Ok(SyncKey(bytes))
    }
}

/// Where this vault's sync key lives, derived from the session target C1 already writes into
/// `config/cloud.yaml` (*Interfaces* contract 1). **A derivation, not a fifth key in that file** —
/// `account::auth_base` derives the auth base from `api_base` for the same reason and says so.
///
/// A target that is not the `…/session` shape C1 writes gets a suffix rather than a guess: two
/// different vaults must never derive the same key target, and a silent collision would hand one
/// profile's key to another.
pub fn key_target(cfg: &CloudConfig) -> String {
    match cfg.session_credential_target.strip_suffix("/session") {
        Some(root) if !root.is_empty() => format!("{root}/sync-key"),
        _ => format!("{}-sync-key", cfg.session_credential_target),
    }
}

/// Tells a genuinely missing key from a key that IS stored and did not come back clean.
///
/// `wincred::CredError::NotFound { code: 1168, .. }` is `ERROR_NOT_FOUND`, the ordinary
/// missing-credential case — the switch is off, which is `NoKey`, a normal state and a named skip.
/// Any other `GetLastError` (permission denied, a locked profile, anything else Credential Manager
/// can refuse a read for) and a blob that is not valid UTF-16LE are both `KeyUnreadable`: the
/// credential exists in some form and the read did not come back clean, which `NoKey` must never
/// hide behind a normal-looking skip. `CredError`'s own `Display` never carries blob bytes — only a
/// target name, a code, or a fixed reason string — so folding it into `KeyUnreadable`'s message is
/// safe.
#[cfg(windows)]
fn key_error(e: crate::wincred::CredError) -> SyncError {
    use crate::wincred::CredError;
    match e {
        CredError::NotFound { code: 1168, .. } => SyncError::NoKey,
        other => SyncError::KeyUnreadable(other.to_string()),
    }
}

/// The key, or the reason there is none. **Precondition P1 decides this function's body and nothing
/// else in this plan**: (a) as written, the key was made on this device and kept here; (b) would
/// derive it from the password at sign-in; (c) would fetch it from the service.
#[cfg(windows)]
pub fn load_key(cfg: &CloudConfig) -> Result<SyncKey, SyncError> {
    let target = key_target(cfg);
    // A missing credential (`ERROR_NOT_FOUND`, 1168) is `NoKey` via `key_error` — the switch is
    // off, the normal state and a named skip. Any other Credential Manager error, or a stored value
    // that is not a 32-byte base64 key, is `KeyUnreadable`, which names the cause.
    let credential = crate::wincred::read_credential(&target).map_err(key_error)?;
    SyncKey::from_base64(credential.password.expose())
        .map_err(|_| SyncError::KeyUnreadable("stored value is not a 32-byte base64 key".to_string()))
}

/// The credential store is Windows-only (spec §6.5), so a build for anything else compiles and
/// reports that sync is not turned on, rather than pretending there is no account.
#[cfg(not(windows))]
pub fn load_key(_cfg: &CloudConfig) -> Result<SyncKey, SyncError> {
    Err(SyncError::NoKey)
}

// ---------------------------------------------------------------------------
// The envelope
// ---------------------------------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Seal one plaintext. Returns `(iv, ciphertext)`, both base64.
///
/// **A fresh random nonce per envelope**, because identical ciphertext for identical plaintext tells
/// a reader of the table which records repeat — a leak the IV exists to close. The tag is appended
/// to the ciphertext, which is what WebCrypto's `AES-GCM` does too, so C1's `_shared/crypto.ts`
/// opens what this seals and the cross-language vectors in Task 3 prove it.
pub fn seal(key: &SyncKey, plaintext: &[u8]) -> Result<(String, String), SyncError> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
    use ring::rand::SecureRandom;
    let unbound = UnboundKey::new(&AES_256_GCM, &key.0).map_err(|_| SyncError::Crypto("key"))?;
    let sealing = LessSafeKey::new(unbound);
    let mut nonce = [0u8; NONCE_LEN];
    ring::rand::SystemRandom::new().fill(&mut nonce).map_err(|_| SyncError::Crypto("random"))?;
    let mut buf = plaintext.to_vec();
    sealing
        .seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut buf)
        .map_err(|_| SyncError::Crypto("seal"))?;
    Ok((B64.encode(nonce), B64.encode(&buf)))
}

/// Open one envelope. A tampered row, a truncated row or the wrong key are all one refusal: GCM's
/// tag is what makes a flipped bit a failure instead of a different note.
pub fn open(key: &SyncKey, iv: &str, ciphertext: &str) -> Result<Vec<u8>, SyncError> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
    let raw_iv = B64.decode(iv).map_err(|_| SyncError::Crypto("iv"))?;
    let nonce: [u8; NONCE_LEN] = raw_iv.try_into().map_err(|_| SyncError::Crypto("iv"))?;
    let mut buf = B64.decode(ciphertext).map_err(|_| SyncError::Crypto("ciphertext"))?;
    let unbound = UnboundKey::new(&AES_256_GCM, &key.0).map_err(|_| SyncError::Crypto("key"))?;
    let opening = LessSafeKey::new(unbound);
    let plain = opening
        .open_in_place(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut buf)
        .map_err(|_| SyncError::Crypto("open"))?;
    Ok(plain.to_vec())
}

/// The MAC key the three index values are computed under — **never the AEAD key itself**.
///
/// `HKDF-SHA256(salt = "", ikm = the sync key)` expanded once under the label `knowlu/index`. One
/// key, one purpose: a key used both to seal and to MAC is the kind of reuse that is fine until the
/// day it is not, and HKDF costs microseconds.
///
/// Derived **once per run** and passed down, not recomputed per row: `apply` MACs the whole local
/// journal to build its `known` set, and a fresh extraction per record would be the only slow thing
/// in this module.
pub struct IndexKey(ring::hmac::Key);

/// Hand-written for the reason [`SyncKey`]'s is: it is key material, and a `{:?}` lands in panic
/// messages and log lines.
impl std::fmt::Debug for IndexKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexKey").field("key", &"<redacted>").finish()
    }
}

impl SyncKey {
    pub fn index(&self) -> IndexKey {
        let prk = ring::hkdf::Salt::new(ring::hkdf::HKDF_SHA256, &[]).extract(&self.0);
        let okm = prk
            .expand(&[b"knowlu/index"], ring::hmac::HMAC_SHA256)
            .expect("HKDF-Expand of one fixed label into a SHA-256 MAC key cannot fail");
        IndexKey(ring::hmac::Key::from(okm))
    }
}

fn mac_hex(index: &IndexKey, message: &[u8]) -> String {
    hex(ring::hmac::sign(&index.0, message).as_ref())
}

/// The identity of one journal record: `HMAC-SHA256(K_index, …)` over `ledger::dumps_value`'s
/// canonical bytes.
///
/// **Keyed, and that is the whole promise.** A bare `SHA-256` of a journal record is an offline
/// oracle: the shape is public (`{actor, device, field, id, new, old, op, path, seq, ts, via}`,
/// sorted, with Python's separators), `op` is one of six, `via` one of five, `field` a small closed
/// set, `old`/`new` usually small integers, and `received_at` bounds `ts` to a narrow window — so
/// anyone holding the table could enumerate candidates until one matched. With `K_index` they
/// cannot, and every other property is unchanged.
///
/// Canonical matters twice. It is what makes the server's `sync_records_once` a real deduplication —
/// a record re-sent after a dropped connection MACs the same — and it is what lets **this** device
/// recognise its own records coming back down a pull without decrypting them.
pub fn record_hash(index: &IndexKey, record: &Record) -> String {
    let body = crate::ledger::dumps_value(&Value::Object(record.clone()));
    mac_hex(index, format!("record\n{body}").as_bytes())
}

/// The identity of one note: `HMAC-SHA256(K_index, …)` over its vault-relative path,
/// POSIX-separated — `journal::json_path`'s rule, so a machine that handed us a backslash gets the
/// same ref. **The path itself never leaves the device**; it is inside the ciphertext, where a
/// restore needs it and nothing else does.
///
/// Keyed for the same reason: a vault path is `tasks/<slugified assignment title>.md` over a known
/// alphabet, which is a few million candidates, not a secret.
pub fn note_ref(index: &IndexKey, rel_path: &str) -> String {
    mac_hex(index, format!("note\n{}", rel_path.replace('\\', "/")).as_bytes())
}

/// An opaque, stable name for this machine under this account.
///
/// **Not the hostname, and keyed.** `journal::device_name()` is a person's name on plenty of
/// machines; it belongs inside the ciphertext, where `reconcile` reads it, and not in a column. A
/// bare digest would be reversible from the row it sits in, because the account id is in the same
/// row and the hostname space is small.
pub fn device_token(index: &IndexKey, account_id: &str) -> String {
    mac_hex(index, format!("device\n{account_id}\n{}", crate::journal::device_name()).as_bytes())[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed 32-byte key, base64. **Not a secret**: it is a test vector, it protects nothing, and
    /// it is the same string `cloud/supabase/functions/_shared/sync_vectors.json` carries so the two
    /// languages can open each other's envelopes (Task 3).
    const SYNC_TEST_KEY_NOT_A_SECRET: &str = "bm90LWEtc2VjcmV0LTMyLWJ5dGUtdGVzdC1rZXktISE=";

    fn key() -> SyncKey {
        SyncKey::from_base64(SYNC_TEST_KEY_NOT_A_SECRET).expect("the test vector is 32 bytes")
    }

    #[test]
    fn a_sealed_envelope_opens_back_to_the_same_bytes() {
        let k = key();
        let plain = b"{\"op\": \"set\", \"field\": \"importance\"}";
        let (iv, ct) = seal(&k, plain).expect("seal");
        assert_eq!(iv.len(), 16, "twelve bytes of nonce is sixteen base64 characters: {iv}");
        assert_ne!(ct.as_bytes(), plain, "the ciphertext is not the plaintext");
        assert_eq!(open(&k, &iv, &ct).expect("open"), plain.to_vec());
    }

    #[test]
    fn two_seals_of_one_plaintext_differ_and_both_open() {
        // A fresh random nonce per record. Identical ciphertext for identical plaintext would tell a
        // reader of the table which records repeat, which is exactly the leak the IV exists to stop.
        let k = key();
        let (iv1, ct1) = seal(&k, b"same").expect("seal");
        let (iv2, ct2) = seal(&k, b"same").expect("seal");
        assert_ne!(iv1, iv2);
        assert_ne!(ct1, ct2);
        assert_eq!(open(&k, &iv1, &ct1).expect("open"), b"same".to_vec());
        assert_eq!(open(&k, &iv2, &ct2).expect("open"), b"same".to_vec());
    }

    #[test]
    fn a_tampered_ciphertext_fails_to_open_rather_than_opening_to_something_else() {
        // GCM, not CBC: the tag is what makes a flipped bit a refusal instead of a different note.
        let k = key();
        let (iv, ct) = seal(&k, b"importance: 5").expect("seal");
        let mut bad: Vec<char> = ct.chars().collect();
        bad[0] = if bad[0] == 'A' { 'B' } else { 'A' };
        let bad: String = bad.into_iter().collect();
        assert!(open(&k, &iv, &bad).is_err(), "a tampered envelope must refuse");
    }

    #[test]
    fn another_key_cannot_open_it() {
        let (iv, ct) = seal(&key(), b"importance: 5").expect("seal");
        let other = SyncKey::from_bytes([7u8; 32]);
        assert!(open(&other, &iv, &ct).is_err());
    }

    #[test]
    fn the_recovery_code_round_trips_through_typing_mistakes() {
        // Crockford base32: no I, L, O or U, case-insensitive, and the three characters people
        // substitute anyway are mapped rather than refused. A student reads this off a screen.
        let k = key();
        let code = k.to_recovery_code();
        assert_eq!(code.chars().filter(|c| *c != '-').count(), 52, "{code}");
        assert!(code.chars().all(|c| c == '-' || c.is_ascii_uppercase() || c.is_ascii_digit()), "{code}");
        assert_eq!(SyncKey::from_recovery_code(&code).expect("exact").to_base64(), k.to_base64());
        assert_eq!(SyncKey::from_recovery_code(&code.to_lowercase()).expect("lower").to_base64(), k.to_base64());
        assert_eq!(SyncKey::from_recovery_code(&code.replace('-', " ")).expect("spaces").to_base64(), k.to_base64());
        let confused = code.replace('0', "O").replace('1', "l");
        assert_eq!(SyncKey::from_recovery_code(&confused).expect("O for 0, l for 1").to_base64(), k.to_base64());
    }

    #[test]
    fn a_code_that_is_not_a_key_is_refused_by_name() {
        assert_eq!(SyncKey::from_recovery_code("").unwrap_err().label(), "that is not a recovery code");
        assert_eq!(SyncKey::from_recovery_code("ABCD-EFGH").unwrap_err().label(), "that is not a recovery code");
        assert_eq!(SyncKey::from_base64("nope").unwrap_err().label(), "that is not a recovery code");
    }

    #[test]
    fn the_fingerprint_is_short_stable_and_not_the_key() {
        let k = key();
        assert_eq!(k.fingerprint().len(), 8);
        assert_eq!(k.fingerprint(), key().fingerprint());
        assert!(!k.to_base64().contains(&k.fingerprint()));
        assert!(k.fingerprint().chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn the_key_never_appears_in_a_debug_or_a_display() {
        // The argument `wincred::Secret` and `account::Session` both make: one stray `{:?}` is all
        // it takes, and the places a `{:?}` lands are a panic message and a log line.
        let k = key();
        let shown = format!("{k:?}");
        assert!(!shown.contains(SYNC_TEST_KEY_NOT_A_SECRET), "{shown}");
        assert!(shown.contains("redacted"), "{shown}");
        assert!(!format!("{}", SyncError::NoKey).contains(SYNC_TEST_KEY_NOT_A_SECRET));
    }

    #[test]
    fn a_record_hash_is_over_the_canonical_bytes_and_not_over_key_order() {
        // Two machines must agree. `dumps_value` sorts keys and uses Python's separators, so a record
        // built in a different insertion order hashes the same — which is what makes the server's
        // unique constraint a real deduplication and not a coincidence.
        let index = key().index();
        let a: crate::ledger::Record = serde_json::from_str(
            r#"{"ts":"2026-09-14T00:00:00.000Z","op":"set","id":"task_0000000001","seq":3}"#,
        ).expect("a");
        let b: crate::ledger::Record = serde_json::from_str(
            r#"{"seq":3,"id":"task_0000000001","op":"set","ts":"2026-09-14T00:00:00.000Z"}"#,
        ).expect("b");
        assert_eq!(record_hash(&index, &a), record_hash(&index, &b));
        assert_eq!(record_hash(&index, &a).len(), 64);
        let mut c = a.clone();
        c.insert("seq".into(), serde_json::Value::from(4));
        assert_ne!(record_hash(&index, &a), record_hash(&index, &c), "the seq is part of the identity of a record");
    }

    #[test]
    fn a_note_ref_is_keyed_and_the_path_is_posix() {
        // `journal::json_path`'s rule, because the ref has to be the same on a machine that happens
        // to have handed us a backslash.
        let index = key().index();
        assert_eq!(note_ref(&index, "tasks/a.md"), note_ref(&index, "tasks\\a.md"));
        assert_eq!(note_ref(&index, "tasks/a.md").len(), 64);
        assert_ne!(note_ref(&index, "tasks/a.md"), note_ref(&index, "archive/a.md"));
        assert!(!note_ref(&index, "tasks/a.md").contains("tasks"));
    }

    #[test]
    fn the_index_is_keyed_so_a_guessable_string_is_not_recoverable_from_its_row() {
        // **The property the whole encryption promise rests on.** A plain SHA-256 of a guessable
        // string IS the string to anyone holding the table: a vault path is `tasks/<slug>.md` over a
        // known alphabet, and a journal record's shape is public (`op` one of six, `via` one of five,
        // `field` a small closed set, `ts` bounded by `received_at`). A few million offline guesses
        // would recover most of a task list. Keying it removes the oracle at no cost to any other
        // property — same 64 hex characters, same uniqueness, same cursor.
        use base64::Engine as _;
        let index = key().index();
        let other = SyncKey::from_bytes([9u8; 32]).index();
        assert_ne!(note_ref(&index, "tasks/a.md"), note_ref(&other, "tasks/a.md"), "two accounts, two indexes");

        // And it is not the bare digest anyone could compute without the key.
        let bare = hex(ring::digest::digest(&ring::digest::SHA256, b"tasks/a.md").as_ref());
        assert_ne!(note_ref(&index, "tasks/a.md"), bare, "an unkeyed digest is the guessing oracle");

        // The index key is DERIVED, never the AEAD key itself: one key, one purpose.
        let raw = B64.decode(SYNC_TEST_KEY_NOT_A_SECRET).expect("the vector");
        let same_key_as_mac = hex(
            ring::hmac::sign(&ring::hmac::Key::new(ring::hmac::HMAC_SHA256, &raw), b"note\ntasks/a.md").as_ref(),
        );
        assert_ne!(note_ref(&index, "tasks/a.md"), same_key_as_mac, "HKDF sits between them");
    }

    #[test]
    fn the_three_index_values_cannot_collide_across_kinds() {
        // Domain separation: a path, a record and a device name are MACed under the same key, so each
        // message carries its own prefix. Without it a crafted path could be made to equal a record's
        // hash and take its place in `sync_records_once`.
        let index = key().index();
        let mut rec = crate::ledger::Record::new();
        rec.insert("ts".into(), serde_json::Value::String("tasks/a.md".into()));
        assert_ne!(note_ref(&index, "tasks/a.md"), record_hash(&index, &rec));
    }

    #[test]
    fn a_device_token_is_opaque_keyed_stable_and_not_the_hostname() {
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("KNOWLU_DEVICE", "ThisIsAPersonsLaptop");
        let index = key().index();
        let t = device_token(&index, "acct-1");
        assert_eq!(t.len(), 16);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(t, device_token(&index, "acct-1"), "stable across calls");
        assert_ne!(t, device_token(&index, "acct-2"), "and scoped to the account");
        assert!(!t.contains("This"));
        // Keyed, for the same reason as the other two: a hostname is a small guessable space and the
        // account id sits in the same row.
        let bare = hex(ring::digest::digest(&ring::digest::SHA256, b"acct-1\nThisIsAPersonsLaptop").as_ref());
        assert_ne!(t, bare[..16], "an unkeyed device token is reversible from its own row");
        std::env::remove_var("KNOWLU_DEVICE");
    }

    #[test]
    fn the_index_key_never_appears_in_a_debug() {
        let shown = format!("{:?}", key().index());
        assert!(shown.contains("redacted"), "{shown}");
        assert!(!shown.contains(SYNC_TEST_KEY_NOT_A_SECRET), "{shown}");
    }

    #[test]
    fn the_key_target_is_the_session_target_with_sync_key_in_place_of_session() {
        let cfg = crate::cloudmodel::CloudConfig {
            api_base: "https://x.example.invalid/functions/v1".into(),
            anon_key: "anon".into(),
            session_credential_target: "knowlu/profile_0123456789/session".into(),
            account_id: "acct-1".into(),
        };
        assert_eq!(key_target(&cfg), "knowlu/profile_0123456789/sync-key");
        // A target that is not the shape C1 writes gets a suffix rather than a guess: two different
        // vaults must never derive the same key target, and a silent collision is worse than a
        // slightly odd name.
        let odd = crate::cloudmodel::CloudConfig { session_credential_target: "some/other/target".into(), ..cfg };
        assert_eq!(key_target(&odd), "some/other/target-sync-key");
    }

    #[cfg(windows)]
    #[test]
    fn key_error_tells_a_missing_credential_from_one_that_did_not_come_back_clean() {
        use crate::wincred::CredError;
        // 1168 is `ERROR_NOT_FOUND`: the ordinary missing-credential case, the switch is off.
        assert_eq!(
            key_error(CredError::NotFound { target: "knowlu/p/sync-key".into(), code: 1168 }),
            SyncError::NoKey,
        );
        // Any other code means Credential Manager refused the read for some other reason — the
        // credential exists in some form, so this must not read as "not turned on".
        let other_code = key_error(CredError::NotFound { target: "knowlu/p/sync-key".into(), code: 5 });
        assert_eq!(other_code.label(), "the stored key is unreadable; restore with your recovery code");
        assert!(matches!(other_code, SyncError::KeyUnreadable(_)));
        let shown = format!("{other_code}");
        assert!(shown.contains("error 5"), "{shown}");
        assert!(!shown.contains(SYNC_TEST_KEY_NOT_A_SECRET), "{shown}");
        // A blob that fails to decode is the same story: something is there, and it is not clean.
        let blob = key_error(CredError::Blob("not valid UTF-16LE"));
        assert_eq!(blob.label(), "the stored key is unreadable; restore with your recovery code");
        assert!(matches!(blob, SyncError::KeyUnreadable(_)));
        assert!(format!("{blob}").contains("not valid UTF-16LE"));
    }

    #[test]
    fn rust_opens_the_envelope_deno_sealed() {
        // The other direction of the same promise. Read by relative path out of `cloud/`, never
        // copied into `engine/tests/fixtures/`: one file, two readers, no chance of two copies
        // drifting apart.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("cloud")
            .join("supabase")
            .join("functions")
            .join("_shared")
            .join("sync_vectors.json");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let v: serde_json::Value = serde_json::from_str(&text).expect("the vector file is JSON");
        let k = SyncKey::from_base64(v["key"].as_str().expect("key")).expect("a 32-byte test vector");
        let plain = open(&k, v["deno"]["iv"].as_str().expect("iv"), v["deno"]["ciphertext"].as_str().expect("ct"))
            .expect("Deno's envelope must open here");
        assert_eq!(String::from_utf8(plain).expect("utf-8"), v["plaintext"].as_str().expect("plaintext"));
    }

    #[test]
    fn generate_makes_a_fresh_random_key_each_time() {
        // The suite would stay green if `generate` returned all zeros; this pins that it does not.
        let a = SyncKey::generate().expect("csprng");
        let b = SyncKey::generate().expect("csprng");
        assert_ne!(a.to_base64(), b.to_base64(), "two calls must not repeat");
        let zero = SyncKey::from_bytes([0u8; 32]);
        assert_ne!(a, zero, "not all-zero");
        assert_ne!(b, zero, "not all-zero");
        assert_eq!(a.to_base64().len(), 44);
        assert_eq!(b.to_base64().len(), 44);
        assert_eq!(SyncKey::from_base64(&a.to_base64()).expect("round trip").to_base64(), a.to_base64());
        assert_eq!(SyncKey::from_base64(&b.to_base64()).expect("round trip").to_base64(), b.to_base64());
    }

    #[test]
    fn the_domain_prefixes_are_load_bearing_not_just_the_message_that_happens_to_differ() {
        // `the_three_index_values_cannot_collide_across_kinds` compares two DIFFERENT messages
        // ("tasks/a.md" vs a record containing it), so it would keep passing even if the
        // `record\n`/`note\n`/`device\n` prefixes were deleted entirely — the two messages would
        // still differ from each other. This test MACs the SAME bytes under two different kinds, so
        // it fails the moment a prefix is removed and the two calls start hashing identical input.
        let _guard = crate::journal::DEVICE_ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let index = key().index();

        let mut rec = crate::ledger::Record::new();
        rec.insert("ts".into(), serde_json::Value::String("shared".into()));
        let canonical = crate::ledger::dumps_value(&serde_json::Value::Object(rec.clone()));
        assert_ne!(
            record_hash(&index, &rec),
            note_ref(&index, &canonical),
            "a record's canonical bytes and a note ref over those same bytes must not collide"
        );

        let s = "shared-string";
        let device = device_token(&index, s);
        let note_first_16 = &note_ref(&index, s)[..16];
        assert_ne!(device, note_first_16, "a device token and a note ref over the same string must not collide");
    }
}
