//! One place where a child process is told not to open a console window.
//!
//! Knowlu is a GUI application. Every console-subsystem child it spawns — `git`, `taskkill`, the
//! engine exe, a model runtime — gets its own console window unless `CREATE_NO_WINDOW` is set, and
//! that window flashes open and shut as the child runs. The housekeeping thread syncs **every 60
//! seconds** (`app/src/main.rs`), and each sync shells out to `git`, so before this module existed
//! the app blinked a terminal at the user once a minute for the whole session.
//!
//! Found by Quinn on 2026-09-09, in the first scratch session long enough to notice — which is
//! exactly what that session was for. It had been latent since the console first spawned a child.
//!
//! The flag is harmless where it is not needed: a GUI child has no console to suppress, and off
//! Windows this is a no-op. Every non-test `Command` in this workspace goes through here, and a
//! guard test in each crate keeps it that way.
//!
//! It is safe on every current call site because none of them wants a console: all of them either
//! capture the child's output through pipes or discard it, and `history.rs` additionally sets
//! `GIT_TERMINAL_PROMPT=0` so git can never wait on input it has no window to receive.

/// `CREATE_NO_WINDOW`, from `processthreadsapi.h`. Spelled out rather than pulled from
/// `windows-sys`: the engine's dependency budget should not grow by a crate for one constant that
/// has not moved since NT 3.5.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Chainable so it drops into an existing builder without reshaping it.
pub trait NoConsole {
    /// Spawn this child without giving it a console window.
    fn no_console(&mut self) -> &mut Self;
}

impl NoConsole for std::process::Command {
    fn no_console(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The flag must not break a child whose output we read: `CREATE_NO_WINDOW` removes the
    /// console, not the pipes. This runs a real child and reads it back, because the failure this
    /// guards against — a chained call that silently returns a different builder, or a flag that
    /// closes the handles — is invisible to a type check.
    #[test]
    #[cfg(windows)]
    fn a_child_spawned_without_a_console_still_writes_down_its_pipe() {
        use std::io::Read;
        let mut child = std::process::Command::new("cmd")
            .no_console()
            .args(["/C", "echo knowlu"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn cmd");
        let mut out = String::new();
        child.stdout.take().expect("stdout pipe").read_to_string(&mut out).expect("read");
        let status = child.wait().expect("wait");
        assert!(status.success(), "{status:?}");
        assert_eq!(out.trim(), "knowlu");
    }
}
