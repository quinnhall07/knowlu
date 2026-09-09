//! Writing Windows Credential Manager entries the engine already reads (plan 4a, Task 3; spec §5).
//! The engine's `src/wincred.rs` is the reader and does not change.
//!
//! **Rules, unchanged from every earlier document:** a secret is never logged, never in a run
//! record, a backup, a fixture, a test name or a plan. Nothing here ever formats a secret — no
//! `Debug`, no `Display`, no error message that carries one; `secret` is only ever read as bytes.
#![cfg(windows)]

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
    CRED_TYPE_GENERIC,
};

/// `knowlu/<profile_id>/<source>` (decision 6) — two profiles on one machine never share a login,
/// and Quinn's existing `quinn-ops/zybooks` and `quinn-ops/vhl` are a different namespace entirely.
pub fn target_for(profile_id: &str, source: &str) -> String { format!("knowlu/{profile_id}/{source}") }

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

/// Creates or replaces the generic credential at `target`. The blob is UTF-16LE and
/// `CredentialBlobSize` is a count of **bytes** — the trap `wincred.rs`'s
/// `blob_size_is_bytes_not_code_units` pins from the reading side; getting it wrong here fails
/// silently on the vendor login three layers away.
pub fn write(target: &str, user: &str, secret: &str) -> Result<(), String> {
    let mut t = wide(target);
    let mut u = wide(user);
    let blob: Vec<u16> = secret.encode_utf16().collect();
    let mut bytes: Vec<u8> = blob.iter().flat_map(|c| c.to_le_bytes()).collect();
    let cred = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(t.as_mut_ptr()),
        CredentialBlobSize: bytes.len() as u32,
        CredentialBlob: bytes.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(u.as_mut_ptr()),
        ..Default::default()
    };
    // SAFETY: every pointer in `cred` is to a local that outlives the call; the API copies.
    unsafe { CredWriteW(&cred, 0) }.map_err(|e| format!("credential write failed for {target}: {}", e.code().0))
}

/// True when a credential with this target exists. Never returns or logs its contents.
pub fn exists(target: &str) -> bool {
    let t = wide(target);
    let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: `t` is NUL-terminated and outlives the call; `ptr` is a valid out-parameter and is
    // freed immediately, without ever being read.
    unsafe {
        if CredReadW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, Some(0), &mut ptr).is_err() || ptr.is_null() {
            return false;
        }
        CredFree(ptr as *const std::ffi::c_void);
    }
    true
}

/// Removes the credential. A target that is not there is an error, not a silent success — a
/// "cleared" login the user can still authenticate with is the worst possible outcome.
pub fn delete(target: &str) -> Result<(), String> {
    let t = wide(target);
    // SAFETY: `t` is NUL-terminated and outlives the call.
    unsafe { CredDeleteW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None) }
        .map_err(|e| format!("credential delete failed for {target}: {}", e.code().0))
}
