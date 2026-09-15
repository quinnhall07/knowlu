// R-C2-E35: one home for the two scope literals. §11a — the calendar scope is asked for first and
// alone (sensitive, lighter review, no CASA); `gmail.readonly` is restricted and asked for later,
// incrementally, and only if the student takes that step. Every reader of a Google grant checks
// `google_accounts.scopes` against these two exact strings, so they live in exactly one place.
export const CALENDAR_SCOPE = "https://www.googleapis.com/auth/calendar.readonly";
export const GMAIL_SCOPE = "https://www.googleapis.com/auth/gmail.readonly";
