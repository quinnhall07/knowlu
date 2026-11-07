# Onboarding replan draft, review r9 (2026-11-07)

**FAIL at be4fe4e:** 0 Critical, 1 Important, 3 Minor. All r8 findings are closed. The split, layers, counts (9 password tasks), ledger, lock on save and in-flight ordering hold. Setup cannot strand: with the text empty, every setup failure opens the school's page.

## Important

1. **A script submit can put the password in a URL (§7.5).** If the fill calls `form.submit()`, or the page's handler doesn't call `preventDefault`, a form with no method and an empty action falls back to a native GET of its own URL with the fields attached. The password then lands in the IdP's logs and in the folder's history.

   **Fix:**
   - Submit with the control's `click()` or `requestSubmit(control)`, never `form.submit()`.
   - Add a last-running listener that prevents the default if the page has not, and records `Rejected(Unconfirmed)`.
   - Add a P1t fake-page case.

## Minor

1. **Navigation submit with an empty action:** it resolves to the page's own URL and goes through the same checks. Add a P1t case.
2. **The lock:** S2 exposes a per-profile, non-blocking lock. P1a uses it and does not edit `school_session.rs`.
3. **Records:**
   - Record the rulings in a dated note.
   - The "Review r8, Minor 4" ledger row should name P4t's pin.
