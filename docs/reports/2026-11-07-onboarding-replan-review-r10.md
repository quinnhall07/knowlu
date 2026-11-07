# Onboarding replan draft, review r10 (2026-11-07)

**PASS at 562ddb2:** 0 Critical, 0 Important, 5 Minor. All of r9's findings are closed. The rulings note names no school, provider, host, class or account. Setup cannot strand.

## Minor (applied in the final pass)

1. **Where the backstop listener sits.** It is a capture-phase listener on `window` that always prevents the native default on a script submit. The pin is widened to ban `submit` used as a property.
2. **When the in-flight mark goes down.** It is written before the first submit that carries a credential.
3. **How the lock is keyed.** The lock is keyed by profile id, and `Capturing` stays the process-wide window guard.
4. **Empty `action`.** It is stated in the §7.5 navigation rule itself.
5. **Citing r9.** Cite the r9 report file.
