//! The mail setup's step machines (email-forwarding plan T10.1, spec §4.2 to §4.4). Doc-only until
//! T10.1 fills it: the module is declared first (hand-off H3c) so the gate stays green with no
//! unused code. It will hold each provider's ordered step list, the per-step fall-back to guided
//! and the fixed 120-day backfill window.
