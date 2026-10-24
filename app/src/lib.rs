//! The shell's library target: `main.rs` is a thin binary over these modules, and
//! `tests/commands.rs` exercises them directly against a scratch vault copy without going through
//! Tauri's IPC or spinning up a window (console plan 1, Task 10).
pub mod account;
pub mod commands;
#[cfg(windows)] pub mod credentials;
#[cfg(debug_assertions)] pub mod debug_port;
pub mod grades;
pub mod lms_link;
pub mod mail;
pub mod mail_client;
pub mod mail_session;
pub mod mail_steps;
pub mod onboarding;
pub mod onboarding_gate;
pub mod onboarding_jobs;
pub mod onboarding_state;
pub mod profiles;
pub mod registrar;
pub mod report;
pub mod scaffold;
pub mod scheduler;
pub mod state;
pub mod telemetry;
pub mod tray;
pub mod updates;
pub mod week;
