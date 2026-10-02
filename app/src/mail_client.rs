//! The app's client for the service's `mail` routes (email-forwarding plan T11.1, P15). Doc-only
//! until T11.1 fills it: the module is declared first (hand-off H3d) so the gate stays green with no
//! unused code. It will hold the rustls `ureq` calls, each carrying the session bearer read through
//! `account`'s public reader, and the map from every failure to a named outcome that carries no
//! token and no address.
