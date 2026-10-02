//! The mail reconciler (email spec 2026-09-30, §6.1; plan T5).
//!
//! **Pure.** Input: a view of the vault's notes (id, path, course, title, due, status,
//! `source_uid`, `created_by`, `mail_asof`, and, per field it may touch, whether
//! `Journal::human_set` finds a record), the vault day, and the ordered forwarded messages with their
//! claims, phase and authenticity word. Output: an ordered list of actions (create, set, archive,
//! card, notice, skip with reason). No file, clock or network access: `today` and the vault day are
//! parameters, so the same vault and the same claims always plan the same actions.
//!
//! **The one place card or direct write is decided (D11).** Nothing else makes that choice. The
//! view's `human_set` is a snapshot; `mail.rs`'s direct-set writer re-checks it from a fresh journal
//! read just before it writes and files the amend card instead if a human set the field in between,
//! and that re-check never turns a card into a direct write.
//!
//! This file holds only its doc until T5.1 adds the action type (controller hand-off H1a).
