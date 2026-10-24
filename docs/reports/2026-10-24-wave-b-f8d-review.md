# Wave B F8d review (2026-10-24)

The reviewer read `94b2905..ad006ca`. The controller filed this report.

**Verdict: PASS**, with 3 Minor findings, all fixed in `cb2b2bf`. The controller checked the diff.

## Held up
- **Normal-console setup is unchanged.** Nothing new sets `Shared::stop`. Where a stop does fire, setup now ends within one POLL instead of a 15-minute confirm or up to 8 minutes of canary wait. That also lets sign-out's 60 s `MAIL_STOP_BOUND` hold during those waits.
- **The withdraw rule can't fire in the new stops.** It deletes only the Knowlu address, and nothing turns forwarding off.
- **A stopped setup resumes.** It is `Stopped`, saved as pending, never failed.
- **The provider string** is `provider_name`'s `m365`, `google` or `other`, not a Debug name.
- **Reasons and outcomes.** Reasons are closed keys. Coursework's named results are `Waiting`. Nothing writes the vault.
- **Abandoned watches.** The drain abandons the canary and first-pass watches at once.
- **Tests.** No existing assertion changed. `never_confirm` defaults to off.

## Fixed in cb2b2bf
1. **The provider write.** It now uses plain `update_answers`, stop or not. The drain waits for `mail.setup`, so the write lands before `on_sign_out` and `write_finish`. Before the fix, Open Knowlu during a canary wait recorded no provider.
2. **No canary after a stop.** The check now sits in `send_canary` itself. That covers `canary_check` (Gmail, and M365 with forwarding already on), Retry, and "I've done this".
3. **A source pin for the runner.** `run_mail_setup` holds exactly one `update_answers(`, for `mail_provider`, and no other answers or state write.

## Note
The new mail test for Retry or Done after a stop can pass on a lucky run. The race needs a hook inside `check_open` to reach deterministically.

## Glue needed
**Glue for the seams.** Whoever wires them must pass closed-set errors. An ingest error that quotes an iCal URL would otherwise land in `onboarding.json`. Owner: F8w1/F8w2 (§14.52).
