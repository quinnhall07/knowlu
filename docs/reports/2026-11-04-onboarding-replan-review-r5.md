# Onboarding replan draft, review r5 (2026-11-04)

**PASS with minors at ed7f584:** 0 Critical, 0 Important, 3 Minor. Reviewed the diff from 5e252f7 only. All five r4 findings closed. The term test holds, RQ15's exception cannot reach anyone mid-setup, and the second read can only stop a delete.

## Minor

1. **§6.5 466-471:** `refresh_series_held` can still store a drop unlocked, after a failed lock (`commitments.rs:2313-2317`) or a pre-read miss. Fix: `held` stores for a drop only when it holds the lock, otherwise the block waits for the next `rank`. Add a C2b case for a failed lock.
2. **§6.12 722:** `purge_class_search(vault, scope)` has no `today`, so the term test would read the real clock in the engine. Fix: add `today`, passed by all four callers.
3. **§11 row "Quinn, RQ6, RQ14":** "true, literally" overclaims. A re-attached vault, or a moved vault with a new registry id, keeps its rows and bundle after a failed Finish purge until sign-out. Name the exception in the row.

## Outside the ask

Recount C2b's line estimate before dispatch. §11 has no "Review r4" row for the lock requirement.
