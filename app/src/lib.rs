//! The shell's library target: `main.rs` is a thin binary over these modules, and
//! `tests/commands.rs` exercises them directly against a scratch vault copy without going through
//! Tauri's IPC or spinning up a window (console plan 1, Task 10).
pub mod account;
pub mod commands;
#[cfg(windows)] pub mod credentials;
pub mod grades;
pub mod inference;
pub mod lms_link;
pub mod onboarding;
pub mod profiles;
pub mod report;
pub mod scaffold;
pub mod scheduler;
pub mod state;
pub mod telemetry;
pub mod tray;
pub mod updates;
pub mod week;
