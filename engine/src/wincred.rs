//! Port of `engine/wincred.py` — read secrets from Windows Credential Manager.
//!
//! Carrying the Python docstring's warning over verbatim, because it is the reason this file is
//! shaped the way it is: **all platform-specific code in this project lives here, and passwords
//! must never enter the repo — Obsidian Git commits and pushes this tree every ~5 minutes.**
//!
//! Two credentials exist in production, written by
//! `cmdkey /generic:<target> /user:<u> /pass:<p>`: `knowlu/zybooks` and `knowlu/vhl`.
//! `engine/coursework.py` reads them at the top of each fetcher and hands them straight to
//! `signin` / `login_and_fetch_dashboard`.
//!
//! # The one trap that has already cost a debugging session
//!
//! `CredentialBlobSize` is a count of **bytes**, and the blob is UTF-16LE — so there are half that
//! many UTF-16 code units. Reading the field as a code-unit count doubles the password (with
//! garbage past the end of the buffer); halving it twice truncates it. Both fail *silently*: the
//! login POST returns HTTP 200 with a login page, and the run WARNs about an empty parse three
//! layers away. `blob_size_is_bytes_not_code_units` pins it.
//!
//! # Documented deviations, stated once
//!
//! 1. Python returns a bare `tuple[str, str]`. This returns [`Credential`], whose password is a
//!    [`Secret`] — a newtype with a redacting `Debug` and *no* `Display`, so `{}`- or
//!    `{:?}`-printing it is a compile error rather than a leak into `runner-log.md`. The string
//!    content is identical; only the container differs.
//! 2. Python raises `LookupError` for both "not on Windows" and "no such credential". The first
//!    arm is unreachable here — the whole module is `cfg(windows)` (see below) — so [`CredError`]
//!    carries only the second, with a byte-identical message.
//! 3. A blob that is not valid UTF-16LE raises `UnicodeDecodeError` in Python (a *different*
//!    exception type, uncaught by anything narrower than `except Exception`). Here it is
//!    [`CredError::Blob`]. Both are fatal to the caller; neither is retried. Decoding stays
//!    **strict** — a lossy decode would hand out a subtly wrong password, which is the same silent
//!    auth failure the byte/code-unit trap causes.
//!
//! # Why the module is gated twice
//!
//! `lib.rs` declares it `#[cfg(windows)] pub mod wincred;`, and the file repeats the gate as an
//! inner attribute. The redundancy is deliberate: the spec (§6.5) requires this module to compile
//! away cleanly so the Linux cloud build never needs `advapi32`, and if the declaration in
//! `lib.rs` ever loses its `cfg`, the inner gate keeps the Linux build green instead of turning
//! one platform-specific file into a portability blocker.

#![cfg(windows)]

use std::ffi::c_void;
use std::fmt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Security::Credentials::{
    CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
};

/// A password, wrapped so it cannot be printed by accident.
///
/// There is no `Display`, and `Debug` redacts. The only way to reach the characters is
/// [`Secret::expose`], which greps as a single obvious call site in any future diff.
///
/// This stops *accidental* logging. It does not scrub memory — the value goes on to an HTTP
/// request body regardless, and Python holds it in an ordinary `str` for the same lifetime.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Secret(value)
    }

    /// The plaintext. Call this only where the secret is being *used* (an auth POST body), never
    /// where it is being formatted.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Redacted by hand. Deriving `Debug` here would put the production zyBooks and VHL passwords
/// into any `dbg!`, `unwrap()` panic message or `{:?}` log line that touched a [`Credential`].
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

/// Python's `(username, password)` tuple.
pub struct Credential {
    /// Python: `cred.UserName or ""` — a null `UserName` becomes the empty string, never an error.
    pub username: String,
    pub password: Secret,
}

/// Manual, for the same reason as [`Secret`]'s. The username is shown: it is an account email,
/// which the plan's own verification step printed, and hiding it would make a failed login
/// undiagnosable.
impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credential")
            .field("username", &self.username)
            .field("password", &self.password)
            .finish()
    }
}

/// Nothing in here ever holds credential material — that invariant is what makes `Debug` safe to
/// derive on this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredError {
    /// Python: `LookupError(f"no credential {target!r} in Credential Manager (error {code})")`.
    /// `code` is `GetLastError()` — 1168 (`ERROR_NOT_FOUND`) for the ordinary missing-credential
    /// case, which is what the caller sees when the credential has not been written yet.
    NotFound { target: String, code: u32 },
    /// The blob was not valid UTF-16LE. Carries a fixed reason string, never any blob bytes.
    Blob(&'static str),
}

impl fmt::Display for CredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // `{target!r}` in Python: single quotes around the target name.
            CredError::NotFound { target, code } => {
                write!(f, "no credential '{target}' in Credential Manager (error {code})")
            }
            CredError::Blob(why) => write!(f, "credential blob is not valid UTF-16LE ({why})"),
        }
    }
}

/// Frees the buffer `CredReadW` allocated, on every exit path including an unwind.
///
/// Python leans on a `try/finally` around the decode; ctypes does not free anything for you, and
/// neither does Rust. This guard is that `finally`.
struct CredBuffer(*mut CREDENTIALW);

impl Drop for CredBuffer {
    fn drop(&mut self) {
        // SAFETY: only constructed from a pointer `CredReadW` returned successfully and that is
        // non-null, and nothing else frees it — the guard owns it for its whole lifetime.
        unsafe { CredFree(self.0 as *const c_void) };
    }
}

/// Return `(username, password)` for a generic credential.
///
/// `cmdkey /generic:<target> /user:<u> /pass:<p>` writes the blob as UTF-16LE, which is what this
/// decodes.
pub fn read_credential(target: &str) -> Result<Credential, CredError> {
    // NUL-terminated UTF-16, held in a local for the whole call so the pointer stays valid.
    // Python hands ctypes a `str` and lets `LPCWSTR` do this conversion.
    let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();

    let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: `wide` is NUL-terminated and outlives the call; `ptr` is a valid out-parameter.
    let called = unsafe { CredReadW(PCWSTR(wide.as_ptr()), CRED_TYPE_GENERIC, Some(0), &mut ptr) };

    // The null check folds into the failure arm rather than becoming a dereference: a success
    // with a null out-pointer cannot happen per the API contract, and Python would have died on
    // `pointer.contents` with a ValueError. Treating it as "not found" keeps this function free of
    // undefined behaviour without inventing a reachable new outcome.
    if called.is_err() || ptr.is_null() {
        // Python's `ctypes.get_last_error()` after `use_last_error=True`. Read straight from the
        // thread rather than unpicking a Win32 code back out of windows-rs's HRESULT, so the
        // number in the message is the same one Python prints.
        let code = unsafe { GetLastError() }.0;
        return Err(CredError::NotFound { target: target.to_string(), code });
    }

    let buffer = CredBuffer(ptr);
    // SAFETY: `CredReadW` succeeded and the pointer is non-null, so it points at an initialised
    // `CREDENTIALW` that stays valid until `buffer` drops — which is after this borrow ends.
    let result = unsafe { fields_of(&*buffer.0) };
    drop(buffer);
    result
}

/// Pull the two fields Python reads out of a `CREDENTIALW`.
///
/// Split out from [`read_credential`] so the byte/code-unit arithmetic and the null-`UserName`
/// path can be tested without a real credential existing on the machine.
///
/// # Safety
///
/// `cred`'s `UserName` must be null or NUL-terminated, and `CredentialBlob` must be valid for
/// `CredentialBlobSize` bytes.
unsafe fn fields_of(cred: &CREDENTIALW) -> Result<Credential, CredError> {
    let username = if cred.UserName.is_null() {
        // Python: `cred.UserName or ""` — ctypes yields None for a null LPWSTR.
        String::new()
    } else {
        // Lossy to match ctypes, which builds a `str` from the wide buffer without erroring on a
        // lone surrogate. The username is not a secret and never blocks the read.
        String::from_utf16_lossy(unsafe { cred.UserName.as_wide() })
    };

    let size = cred.CredentialBlobSize as usize;
    let bytes: &[u8] = if size == 0 || cred.CredentialBlob.is_null() {
        // `ctypes.string_at(NULL, 0)` returns `b""`; `from_raw_parts` on a null pointer is UB even
        // for length 0, so the empty case never forms a slice.
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(cred.CredentialBlob, size) }
    };

    Ok(Credential { username, password: Secret(decode_blob(bytes)?) })
}

/// Python's `blob.decode("utf-16-le")`, strict.
///
/// `bytes` is the blob as `CredentialBlobSize` **bytes**; pairs of them make one UTF-16 code unit.
/// Read byte-wise rather than casting to `*const u16` — the cast would also assume an alignment
/// the API does not promise.
///
/// No NUL trimming: the blob is exactly the bytes the writer stored, and Python keeps whatever is
/// there. A leading BOM likewise stays as U+FEFF, since `utf-16-le` (unlike `utf-16`) does not
/// strip one.
fn decode_blob(bytes: &[u8]) -> Result<String, CredError> {
    if bytes.len() % 2 != 0 {
        // Python: UnicodeDecodeError, "truncated data".
        return Err(CredError::Blob("odd byte count"));
    }
    let units: Vec<u16> =
        bytes.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
    String::from_utf16(&units).map_err(|_| CredError::Blob("unpaired surrogate"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::PWSTR;

    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    /// Build a `CREDENTIALW` over borrowed buffers. Never passed to `CredFree` — it is ours.
    fn fake(blob: &[u8], username: Option<&mut Vec<u16>>) -> CREDENTIALW {
        CREDENTIALW {
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_ptr() as *mut u8,
            UserName: match username {
                Some(u) => PWSTR(u.as_mut_ptr()),
                None => PWSTR::null(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn blob_size_is_bytes_not_code_units() {
        // "pw!" is 6 bytes and 3 code units. Treating the size as a code-unit count reads six
        // units (three of them past the end of the buffer); halving twice reads one. Both produce
        // a wrong password and a silent auth failure.
        let blob = utf16le("pw!");
        assert_eq!(blob.len(), 6);
        let mut user: Vec<u16> = "quinn\0".encode_utf16().collect();
        let cred = fake(&blob, Some(&mut user));

        let got = unsafe { fields_of(&cred) }.unwrap();
        assert_eq!(got.username, "quinn");
        assert_eq!(got.password.expose(), "pw!");
        assert_eq!(got.password.expose().chars().count(), 3);
    }

    #[test]
    fn a_null_username_is_the_empty_string() {
        // Python's `cred.UserName or ""`.
        let blob = utf16le("x");
        let cred = fake(&blob, None);
        assert_eq!(unsafe { fields_of(&cred) }.unwrap().username, "");
    }

    #[test]
    fn an_empty_blob_is_an_empty_password_not_a_crash() {
        // `ctypes.string_at(NULL, 0)` is `b""`.
        let cred = CREDENTIALW { ..Default::default() };
        let got = unsafe { fields_of(&cred) }.unwrap();
        assert_eq!(got.password.expose(), "");
        assert_eq!(got.username, "");
    }

    #[test]
    fn non_ascii_and_astral_characters_survive() {
        // A surrogate pair is two code units and four bytes for one character — the case that
        // catches an off-by-one in the pairing loop.
        for text in ["pässwörd", "pw\u{1D11E}", "«¿»"] {
            let blob = utf16le(text);
            let cred = fake(&blob, None);
            assert_eq!(unsafe { fields_of(&cred) }.unwrap().password.expose(), text);
        }
    }

    #[test]
    fn a_malformed_blob_errors_rather_than_decoding_lossily() {
        // Python raises UnicodeDecodeError for both of these.
        assert_eq!(decode_blob(&[0x70, 0x00, 0x77]), Err(CredError::Blob("odd byte count")));
        // A high surrogate with no low surrogate after it.
        assert_eq!(
            decode_blob(&[0x00, 0xD8, 0x70, 0x00]),
            Err(CredError::Blob("unpaired surrogate"))
        );
    }

    #[test]
    fn a_secret_cannot_be_printed_by_accident() {
        // The single most important property of this file.
        let cred = Credential {
            username: "user@example.com".into(),
            password: Secret::new("correct-horse-battery-staple".into()),
        };
        let debug = format!("{cred:?}");
        assert!(!debug.contains("correct-horse"), "Debug leaked the password: {debug}");
        assert!(debug.contains("<redacted>"), "{debug}");
        assert!(debug.contains("user@example.com"), "{debug}");
        assert!(!format!("{:?}", cred.password).contains("correct-horse"));
        // There is deliberately no `Display for Secret`, so `format!("{}", secret)` will not
        // compile. Nothing to assert here beyond its absence.
    }

    #[test]
    fn the_not_found_message_matches_pythons() {
        let err = CredError::NotFound { target: "knowlu/zybooks".into(), code: 1168 };
        assert_eq!(
            err.to_string(),
            "no credential 'knowlu/zybooks' in Credential Manager (error 1168)"
        );
    }

    /// Port of `tests/test_coursework.py::test_missing_credential_raises_lookup_error`, target and
    /// all. The only test here that touches the real API, and it needs no credential to exist —
    /// which is the point: a missing credential is a clean error, never a panic.
    #[test]
    fn a_missing_credential_is_a_clean_not_found() {
        let err = read_credential("knowlu/definitely-not-a-real-target-9c2f1a")
            .err()
            .expect("a target that does not exist must not resolve");
        match &err {
            CredError::NotFound { target, .. } => {
                assert_eq!(target, "knowlu/definitely-not-a-real-target-9c2f1a");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
        assert!(err.to_string().starts_with("no credential 'knowlu/definitely"), "{err}");
    }

    #[test]
    fn an_empty_target_fails_the_same_way() {
        // `str(cfg.get("credential_target") or "")` in coursework.py yields "" when the config key
        // is missing. Python passes it through to CredReadW unchanged and gets a LookupError; so
        // does this. No special case.
        assert!(matches!(read_credential(""), Err(CredError::NotFound { .. })));
    }
}
