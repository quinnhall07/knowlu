//! The mail pass's device half (email spec 2026-09-30, §6.1; plan T6).
//!
//! Pulls forwarded-mail claims through [`crate::cloudmodel`], plans them with
//! [`crate::mailreconcile`] (pure) and writes the planned actions through `write` as
//! `agent:knowlu.mail`, journal first. Transport and writing only: `rank` never reaches it and
//! no mail text, address or session enters a log, the journal or telemetry.
//!
//! T6.2 adds the writers; T6.3 the pass; T6.4 its call from `enrich`. Until then this holds only
//! its module doc, so the gate stays green with no unused code.
