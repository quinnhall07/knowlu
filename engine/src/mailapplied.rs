//! `knowlu-engine mail-applied` (email spec 2026-09-30, §4.14 and §6.1; plan T7).
//!
//! Read-only: it lists what the mail pass wrote as `agent:knowlu.mail`, joined to each note's
//! current value, so the app can show the student what forwarded mail changed. It never writes
//! and is never entitlement-gated, like `surface`. Doc-only until T7 (plan H1c); the dispatch
//! line in `cli.rs` is H2.
