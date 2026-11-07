# Onboarding replan draft, review r8 (2026-11-07)

**FAIL at c071c25:** 0 Critical, 4 Important, 6 Minor. Reviewed 2ea7d85..c071c25.

**What holds.**
- No ending can re-send the password through Knowlu: navigation, `Lost`, a timeout, an app close, sleep, or a failed mark write.
- r7's Important 2, 3 and 4 are closed. Important 3 is closed in design and held at checkpoint 1.
- Minors 1–5 are closed, and the layers, counts and rulings are consistent.

## Important

1. **The `POST` rule may refuse the founder's real page (§7.5 l.1051-1056, P1t l.1451-1453).** A form with no `method` resolves to `GET`, and script-submitted widgets often render such a form. r7 dropped r6's "submits by navigation" qualifier. Fix: read the real form's attributes, then state the rule for navigation and script submits separately.
   - **Controller's spike reading (2026-11-07, values only, nothing filled):** the founder's sign-in page has one form with no `method`, an empty `action`, and no `formaction` or `formmethod` on any control. It is script-submitted.
2. **What `wrong_password_text` is matched against is unspecified (§7.5 l.1066-1072).** Hidden error strings would match on an MFA page. Fix: match visible text only, after `SUBMIT_SETTLE`, and add a P1t case for hidden or script text.
3. **P5 cannot read the message without a second wrong attempt (l.1569-1575).** The message may also hold the username, and `CAMPUSES` is public. Fix: take the text from public documentation, or plan a counted second attempt with Quinn's go. Add a `CAMPUSES` test that refuses `@` and digits-only tokens.
4. **The in-flight mark is not required to be durable (§7.4 l.984-990, P1 l.1487-1489).** Fix: write the mark, then `sync_all` it before the submit (as `scaffold.rs:933` and `ledger.rs:108` do). A failed flush makes the attempt `CannotRead`.

## Minor

1. A save can race a fill. The save should take `school_session`'s lock, or answer `Busy`.
2. Say that both marks are read after the lock is taken.
3. Two orders are ambiguous:
   - when the in-flight mark is checked: at the start of a read, or only before a fill;
   - what the row opens when the rejected mark is set and no password is saved (table rows 2 and 4 disagree).
4. §2.3 states "one push" as a fact. The re-offer's "keep signing in for you" should be tied to R0c.
5. A 307 or 308 redirect from a listed host re-POSTs the body. Add this to §7.5's accepted limit.
6. P1 and P2's "about 80 lines" is stale. Split P2 into the pure decision and the window script.

## Authority

At checkpoint 1, cite where Quinn's own words for RQ17 and RQ18 are recorded.
