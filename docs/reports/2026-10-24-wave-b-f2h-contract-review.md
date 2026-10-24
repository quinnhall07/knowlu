# Wave B F2h contract review (2026-10-24)

The contract-reviewer read `e136c6e..dddf187` (`app/src/onboarding_state.rs`, +38/−3). The controller filed this report.

## r1: FAIL, on a test date bomb
- **Important.** Test 22's `entitlement_body` hard-coded `checked_at: 2026-10-24T09:00Z`. `decide` reads an entitlement as fresh only for 72 hours, so the test would go red on every branch from 2026-10-27T09:00Z.
  - **Controller fix (test data only, assertion unchanged):** `checked_at` is now `journal::now_ts(None)`.
  - The five test-22 tests pass. `onboarding_state` is at 61 passing and 11 failing; all 11 are F8b/F8lb's `save_preview`/`save_pairs` stubs.

## Minor (open)
- `cloud_config`'s errors start with the vault path, so the Windows user name can reach the page's JSON. It stays on the device, and the existing `open_portal` and `entitlement_now` do the same. A later pass could strip the path, as `file_only` does.
- The wake condition (`ok && Entitled`) sits behind the app handle and has no test. A pure `should_wake` helper would fix that.

## Held up
- **Bearer.** The bearer is always the vault's own target from `cloud.yaml`, never the pending one, and `PENDING_TARGET` is still pinned to F2g's wrapper.
- **Cache.** The cache is unchanged on any failure.
- **Errors.** No error carries a token, email or account id; the missing-session error names only the opaque derived profile id.
- **Checkout.** Consent is sent before the checkout link. `open_in_browser` is limited to http(s), and the host is Stripe's.
- **Wake.** No wake happens on a stale cached Entitled.
- **Commands.** `AppHandle` with `try_state` is sound, and both commands are async. F5 registers both.
- **Tests and contract list.** The tests hold `CREDMAN_LOCK` and bind only `127.0.0.1:0`. No contract-list file is touched.
