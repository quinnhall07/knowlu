# Wave B F8c review (2026-10-24)

The reviewer read `e136c6e..a2dbec5`. The controller filed this report.

**Verdict: PASS**, with 1 Important and 6 Minor findings. Fixes are in `759a42e`, which the controller checked.

## Held up
- Feed links never reach a log, error, JSON or the answers file.
- Links are stored only through this vault's own `cloud.yaml` session. `PENDING_TARGET` does not appear, and there is one `store_source(` call.
- No pending link is deleted before it is safe.
- `config.ingest` never runs while a feed is `validated`.
- Vault writes are create-new, through F6b's helper.
- No contract-list file is touched.
- The tests use no Credential Manager and no network.

## Fixed in 759a42e
- **I-1. No job reported a kept link (§4.11).** The Done proof now names per feed:
  - `feed_kept_on_device` or `calendar_kept_on_device` when the link went into `ingest.yaml`.
  - `lms_link_not_placed` or `calendar_link_not_placed` when an existing file kept other bytes.
  - `pending_link_not_deleted` when a pending link's delete failed.
  - The link itself never appears.
- **M1.** Try again on a `failed` store whose link `config.ingest` already placed returns Done. It used to return a false "link missing".
- **M2.** The login flags use the mapping's own key-plus-slug filter.
- **M3.** A failed pending-link delete is reported in the proof.
- **M6.** New tests: every feed `stored`, a failed save after the account call, the calendar feed holding `config.ingest`, the kept-on-device keys, and a delete failure. A Drop guard cleans up the scratch directories.

## Routed elsewhere
- **M4.** `change_school` leaves the old school's courses, registrar rows and buckets in the answers file. `recapture` can carry the old pairs across a shared LMS key. A `school.capture` still running can also write the old link after the clear. Goes to the planner, then F8e/F8f.
- **M5.** `registrar.confirm`'s runner, which is R24's write behind F8h's `retry_on_sync_running`, is not wired. The production `Runner` dispatch goes to F8g, and the retry to F8h.
