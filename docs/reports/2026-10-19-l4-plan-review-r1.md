# L4 plan review, round 1 (2026-10-19)

Plan: `docs/plans/2026-10-19-l4-honest-page-plan.md` (draft), checked against the email branch at
`bf3e135`. Reviewer, plan mode. Filed by the controller from the reviewer's returned text (the reviewer
cannot write files).

**Verdict: FAIL.** 1 critical, 4 important, 5 minor.

## The plan's code facts check out

- `PRIVACY_VERSION` lives only at `app/src/account.rs:25`. `workflows.rs:365` and `privacy-guard.ps1`
  read it from there.
- The consent route returns a silent 200 once any `tos` row exists (`handler.ts:173`).
- `sign_out_with` (`account.rs:668-696`) never touches `pending_setups`; `forget_all` only deletes
  profile folders.
- `site/` deploys only in `release.yml`'s `publish` job (main :278-282; email branch :284-288).
- `record_consent_at` and its three callers are where the plan says.

## Critical

**C1. The L8 ordering deadlocks.**
- Q-L8 (:363) and T13 (:788) make the bump wait for L8's merge.
- L8 cannot merge without its S5. L8's spec starts S5 only after L6 and L4 have merged
  (friends-testers spec :525-526, :1036), and Q-QUEUE (:418) repeats it.
- So the recommended wait never ends, and the fallback (sentence 2 only, `ROUTE_COUNTS` held back)
  becomes forced rather than chosen.

Fix: before T9, ask Quinn to choose one of:
- (a) Move L8 S5 ahead of L4's T8 in the queue.
- (b) T13 waits only for L8's S4, split into its own PR.
- (c) Adopt the fallback now.

Then fix Q-QUEUE and §6 to match.

## Important

**I1. T6's tests would reach the network and expect the wrong rows** (T6 :667-678 against §4.5 :268).
- The slot calls `refresh_privacy` whenever the gate is not Accepted, using the scratch vault's
  `api_base` and the real session credential.
- The expected rows miss a `privacy (check failed)` row, or an unnamed success row.
- A successful refresh would overwrite the test's `Older` cache.

Fix: point `cloud.yaml` at a `127.0.0.1:0` server that answers `GET /account/consent`. Name the success
row, or say there is none. Use a generated session target under `CREDMAN_LOCK`, or leave it unset.

**I2. The sign-out order in §4.7 (:315-321) cannot be built as written.**
- `clean_unfinished` deletes `mail-session-*` profiles (`mail/setup.rs:419`), so it needs the mail
  window's hold.
- `MailClient::for_session` reads the token from Credential Manager on each request
  (`mail_client.rs:186-190`), so the clean must run before the credential delete.
- Today the hold is taken after the delete (`account.rs:679-683`), and `close_mail_window` is an
  `FnOnce`.

Fix: spell out T4's order:
1. Close the mail window and take its hold.
2. Clean.
3. Clear the records through `MailState::update`.
4. `forget_all`.
5. Drop the hold.
6. Delete the credential.
7. Revoke the token.

Name how sign-out builds its client: `MailClient::with_token_fn` with the token loaded at :677 plus
`api_base()`, or the profile's vault and `client_for`. Bound the clean's total time.

**I3. T9's committed red tests break "workspace green" and the rebases** (T9 :744-746, T10 :762).
- T9's red page tests stay red until T13.
- T10's uncommitted diff blocks the rebases T6 and T13 need (:655, :788). Agents may not stash.

Fix: either hold T9's tests as a saved patch (seen red, recorded in the ledger) and commit them in T13,
or exclude the named tests from T5/T6/T8's green. The controller resets and re-applies T10's patch
around each rebase.

**I4. Delete my data while consent is owed: the way in is not pinned** (§4.6 :294-296, T7 :690-701).
- `#reconsent` is a fixed `.setpanel` at z-index 20 (`console.css:471`) with no dismiss.
- The gear (`console.js:88`) is the only in-page way into Settings, and nothing checks the panel cannot
  cover it.

Fix: a "Settings" button inside the panel that calls `openSettings()`, required by T7's first test.

## Minor

- **M1.** Q-AGENT is handled correctly in substance, but Amendment 2026-10-19 is uncommitted and never
  mentions a lane PR. Add "the Amendment is committed on main" to T0, and have T16's check 6 cite the
  Q-AGENT ledger row.
- **M2.** §4.5 :271's `mail (skipped: privacy…)` would appear with no mailbox connected, where today there
  is no row (`scheduler.rs:972-974`). Name it only when `mail_step` would have run.
- **M3.** §4.5 gives no precedence between skips. When the state is both Older and NotEntitled, which name
  wins? State it and test one case.
- **M4.** `privacy.json` also holds `account_id` and `checked_at` (§4.4 :245). This is not a new data
  class; list the three keys in T14's `app.md` entry.
- **M5.** T15's sign-out line (:840) should record its expected outcome (a failed list keeps nothing).

## Gate design and privacy

**The gate keeps CLAUDE.md's rules.**
- Skips are named rows at exit 0, made app-side by filtering `slot_argv`'s output.
- `rank`, backup and telemetry still run.
- Under Q-GATE, `Unknown` (no cache, or an outage) leaves the slot unchanged, so an outage cannot stall
  every slot.
- A cached `Older` holds through an outage.

**Privacy.** Nothing new is logged. The new cloud path writes one `consents` row of the existing kind.
