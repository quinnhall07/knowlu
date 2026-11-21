PASS

# v0.4.1 pricing, cloud tasks: re-review of fix round 1

Reviewer, 2026-11-21. This is the second round of `2026-11-21-v041-cloud-review.md`. My filing rule is
that a new round is a new file and an earlier report is never written over, so this re-review is filed
here and not appended to the first report. Read the two together.

**Scope.** Three commits after `aa96fb8b` on `v041/t01-cloud`: `648350a4` (T1), `04519f20` (T2) and
`c929cc2f` (T3). The fix diff is six files and +71/−10 lines. Each commit touches only its own task's
files, and no tag was moved. All three changed test files are LF. I read the whole fix diff and the
"Fix round 1" section of the engineer's report. I did not re-run the suite; the report records
`ok | 1598 passed (588 steps) | 0 failed`, which is the earlier 1596 plus the two new tests.

## The four findings

### Finding 1 (T1, note D3): ADDRESSED

- **The fix:** `billing-checkout/handler.ts:171-183`. The two type checks are gone. A present value that
  is not `=== entry.cents` or `=== termsVersion` now reaches the 409 at `:183`, which carries D3's
  "changed" sentence. An absent value or JSON `null` is still 400.
- **The test:** `a_changed_offer_is_refused` gains three cases (`billing-checkout/handler_test.ts:245-248`):
  `amount_cents: "1299"`, `amount_cents: 12.99` and `terms_version: 20260910`. The old code answered all
  three with 400, so the test fails on it.
- **The trade-off is harmless.** The request's `terms_version` is never written. The consent row takes
  `deps.catalog.termsVersion` (`:239`), so removing the type check lets nothing malformed reach
  `consents.version`.

### Finding 2 (T1, note D4): ADDRESSED

- **The fix:** `billing-checkout/handler.ts:212-214`. When `getPrice` gives `null`, the function logs the
  plan and answers `fail(500, NOT_CONFIGURED)`.
- **The test:** the unreadable-price case in `the_price_is_checked_before_it_is_sold`
  (`handler_test.ts:388-403`) now asserts 500, the body, no consent row, no session and no customer. The
  old code answered 502, so it fails on the old code.

### Finding 3 (T2): ADDRESSED

- **The fix:** `stripe-webhook/handler.ts:295-302`. After a successful send, `recordEvent` is inside a
  `try/catch`. On failure it logs `stripe-webhook: <account id>: the reminder was sent but not recorded`
  and answers 200. A failed send is unchanged at `:288-294`: 502, with nothing recorded.
- **The test:** `a_reminder_sent_but_not_recorded_is_not_retried` (`handler_test.ts:900`). Its
  `recordEvent` rejects with a message that carries an address. The test asserts 200, one mail, no
  entitlement written, a log line naming the account id, and no "@" in any log line. On the old code the
  rejection propagated out of `handle`, and the status assertion fails.
- **What stays open:** two deliveries arriving at the same time can still send twice. That is the known
  limit the first round recorded, and it was not asked of this fix.

### Finding 4 (T3): ADDRESSED

- **The fix:** `billing-jobs/handler.ts:162-163`. The raise rule
  (`selling && trialing && selling.cents > kept.cents`, which returns `null`) now runs before the
  missing-`priceId` throw.
- **The test:** `the_raise_rule_skips_a_trial_before_the_selling_price_is_needed` (`handler_test.ts:454`).
  A move entry at 999 meets an unset selling variable at 1299, which is a raise, and a trialing
  subscriber renews in 12 days. The test asserts `{reminded: 0, moved: 0, failed: 0}`, no mail and no
  write. The old code threw first and gave `failed: 1`, so the test fails on it.
- **The cases the throw still covers:** a cut with the selling variable unset, an active subscriber on a
  raise with it unset, and a key the catalog does not sell. Each still throws and counts in `failed`,
  before any notice. That is correct: no notice is sent for a change that cannot be made.

## New breakage in the fix diff

None, Critical, Important or Minor.

## Out of scope (code this fix did not touch)

- In the webhook's two no-mail branches (`stripe-webhook/handler.ts:280` and `:285`), `recordEvent` is
  still not caught. A failure there answers 500 and Stripe retries. No email has gone at that point, so a
  retry is harmless and lands in the same branch again. No change is needed.

## Status of the first round's other findings

- **5 (T3, deploy coupling):** moved by the controller to the docs as a deploy note. It is not a code
  change.
- **6 (T1, note D5) and 7 (T2, note D6):** with Quinn, as note gaps. The code does what the note says.

## Verdicts (updated)

T1a (price catalog): PASS
T1 (checkout): PASS. Findings 1 and 2 are closed; 6 is with Quinn.
T2 (webhook and reminder): PASS. Finding 3 is closed; 7 is with Quinn.
T3 (billing jobs): PASS. Finding 4 is closed; 5 is a docs deploy note.
Whole: PASS. All four findings from round 1 are addressed.
