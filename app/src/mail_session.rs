//! The mail setup's kept browser profiles and `mail.json` (email-forwarding plan T9.1, spec §5.4
//! and D15). Doc-only until T9.1 fills it: the module is declared first (hand-off H3a) so the
//! gate stays green with no unused code. It will follow `grades`'s `is_session_dir` rule and the
//! `grades.json` precedent for `mail.json`, which is its own file and never a `Settings` field.
