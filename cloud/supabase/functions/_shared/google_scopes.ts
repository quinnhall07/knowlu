// R-C2-E35: one home for the two scope literals. §11a — the calendar scope is asked for first and
// alone (sensitive, lighter review, no CASA); `gmail.readonly` is restricted and asked for later,
// incrementally, and only if the student takes that step. Every reader of a Google grant checks
// `google_accounts.scopes` against these two exact strings, so they live in exactly one place.
export const CALENDAR_SCOPE = "https://www.googleapis.com/auth/calendar.readonly";
export const GMAIL_SCOPE = "https://www.googleapis.com/auth/gmail.readonly";

/**
 * The 503 body `/google-connect` and `/gmail-read` answer when P2 (the Google OAuth client) was
 * never set on this deployment — one exported constant because it is an EXACT-MATCH CONTRACT with
 * the device (C2 final review S-4).
 *
 * `engine/src/cloudmodel.rs`'s `GMAIL_NOT_CONFIGURED_DETAIL` compares this text character for
 * character (R-C2-E45 (3)) to tell "Google sign-in was never configured here" — a named, quiet
 * skip — from an ordinary platform 503, which is "try later". Before this constant the sentence
 * was written out five times across two handlers, two tests and the engine; a rewording in any one
 * of them turns the quiet skip into a failure line with nothing to say why.
 *
 * "Google", not "Gmail": the FIRST ask this pair makes is the calendar, and a student who has never
 * heard of the Gmail step must not read it as a mail-only failure (§11a, ruling R-C2-E31).
 */
export const GOOGLE_NOT_CONFIGURED = "Google sign-in is not configured on this deployment";
