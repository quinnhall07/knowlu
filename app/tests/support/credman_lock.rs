//! The one lock every test file that touches the real Windows Credential Manager takes, shared by
//! `#[path]` include (`app/tests/{account,onboarding,scheduler}.rs`).
//!
//! **Why not a plain `std::sync::Mutex`** (the cross-process flake, 2026-09-29): Credential Manager is
//! one store per Windows user, and `account::PENDING_TARGET` is one fixed name in it. A process-local
//! mutex serialises the threads of ONE test binary; two processes running `tests/onboarding.rs` at
//! once (two lanes, two worktrees, a workspace run beside a targeted one) raced on that name — one
//! moved or deleted the other's pending session, and a restore-on-drop put a test session back at the
//! real pending target. So the lock is two layers: the in-process mutex first (threads of this
//! binary queue here, and a panic still poisons it the usual way), then a named kernel mutex in the
//! session namespace, which every test process on the machine's desktop session waits on.
//!
//! `lock()` returns a `LockResult`, so every call site keeps its
//! `.lock().unwrap_or_else(|e| e.into_inner())` shape. The guard is `!Send` (it holds a
//! `MutexGuard`), which a Windows mutex needs: only the owning thread may release it. A process that
//! dies holding it leaves it abandoned, and the next waiter takes it (`WAIT_ABANDONED`).

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject, INFINITE};

/// The one name every real-store test file shares. `tests/onboarding.rs` probes it by this literal.
pub const MUTEX_NAME: windows::core::PCWSTR = windows::core::w!("Local\\knowlu-tests-credman");

pub struct CredmanLock(std::sync::Mutex<()>);

pub struct CredmanGuard {
    named: HANDLE,
    // Dropped after `Drop::drop` releases the named mutex, so a thread of this process can only
    // reach the named mutex once this one has let go of it.
    _local: std::sync::MutexGuard<'static, ()>,
}

impl CredmanLock {
    pub const fn new() -> Self {
        CredmanLock(std::sync::Mutex::new(()))
    }

    pub fn lock(&'static self) -> std::sync::LockResult<CredmanGuard> {
        let (local, poisoned) = match self.0.lock() {
            Ok(g) => (g, false),
            Err(e) => (e.into_inner(), true),
        };
        let named = unsafe {
            let h = CreateMutexW(None, false, MUTEX_NAME).expect("create the cross-process Credential Manager test mutex");
            let r = WaitForSingleObject(h, INFINITE);
            assert!(r == WAIT_OBJECT_0 || r == WAIT_ABANDONED, "waiting on the Credential Manager test mutex failed: {r:?}");
            h
        };
        let guard = CredmanGuard { named, _local: local };
        if poisoned { Err(std::sync::PoisonError::new(guard)) } else { Ok(guard) }
    }
}

impl Drop for CredmanGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.named);
            let _ = CloseHandle(self.named);
        }
    }
}
