# L8 friends testers: S6 review

Reviewer, 2026-10-20. Verdict: PASS. Filed by the controller.

S6 review of the whole L8 lane: S2a0 to S5 plus S7a. Worktree .claude/worktrees/l8-testers, branch l8/testers, diff origin/main...HEAD (base 9bcc09a, 20 commits, 17 files).

Verdict: PASS. No critical and no important findings; four minors, listed under findings. Contract list: not touched. The engine's entitlement test is engine/tests/entitlement_gate.rs, which is unchanged; cloud/_shared/entitlement_test.ts is S3's, assigned to cloud-engineer by the signed spec.

Rule 1 (single-user assumptions):
- I scanned every added line for real addresses and ids. The only address is the synthetic friend@example.com inside the rule-1 tripwire's own test. The proof and the case tables use @example.invalid throughout.
- The two uuids are the nil uuid and one marked synthetic (migrations_test.ts:2141).
- Grants are by account id. No view or function reads an auth table. "Quinn" appears only in comments, as the admin.

Rule 2 (frozen references): no fixture, oracle or surface-today reference is touched. No applied migration is edited. The lane adds one new file, 20261020000500_testers.sql. It is byte-identical between fab9466 (applied on staging at S7a) and HEAD: `git diff fab9466 HEAD -- cloud/` is empty.

Money and entitlement:
- effective_entitlements starts from accounts and left-joins everything, so the paid row always wins. The grant branch is the complement of active/trialing, and an expired or ended grant falls through to the Stripe row.
- Every gate path now reads the view through lookupFrom: requireActiveEntitlement, GET /entitlement, mail-inbound, mail-process and mail-canary. No other function or SQL reads the status of public.entitlements to decide access.
- billing_jobs' view requires stripe_subscription_id, so testers get no renewal mail.
- billing-checkout answers 409 before the Stripe customer, the consent row and the session are created; a failed view read fails closed.
- enforce_budget's per-account ceiling still bounds testers.
- The engine does not read plan, so 'tester' is safe there.
- No workflow auto-deploys functions, so S7's order holds: prod migration and layer-iii proof first, then function deploys.

Rights:
- comp_grants and comp_audit: everything revoked, then select granted to service_role only.
- admin schema and functions: closed to every API role.
- count_route: execute for service_role only.
- route_daily: RLS on, anon and authenticated revoked.
- The views are owner-run, but every API role except service_role is revoked from them.
- Deleting an account cascades to comp_grants and comp_audit as the table owner; A3s proved this on staging under service_role.
- The deferred count races harmlessly with DELETE: either the cascade removes the row or the foreign key refuses it and the error is swallowed.

Privacy:
- ROUTE_COUNTS stays off unless it reads exactly "on" (tests at entitlement_test.ts cover off, other values and on). Prod writes nothing until Q7 sentence 1 is live (S7/S8).
- PRIVACY_VERSION is untouched.
- The export adds comp_grants (no db_role or grant_id), comp_audit (its four columns), route_daily and usage_daily, all paged and ordered by key.
- No telemetry change, and no tokens or content in logs.

App:
- console.js holds the one status-and-plan helper, used at all four WIZ.entitled sites and on entry. Prices are hidden for any entitled account and while the on-entry check is out.
- afterUpgradeSignIn now asks before it un-hides the prices.
- No Rust command was added, so no generate_handler change is needed.
- The existing static_assets edits rename checks to the helper and loosen nothing that matters.

Gates:
- All 17 files are LF, and `git diff --check` is clean.
- The ledger reports Deno at 1224/0 after the merge; workspace cargo 0 failed, 3 ignored (the runtime smoke test went with L6), 0 warnings beyond .rsrc.
- No workflow or action change.

S7a, staging: I accept the controller's record. The rehearsal ran in a rolled-back transaction and left nothing behind, and a mutated V1 raised. After db push, the proof ran clean, and comp_grants, comp_audit and the @example.invalid users were all at 0 afterwards.

Open S2-S4R minors, judged:
- The last_lapsed_at exclusion is a consistency nit, not a gap; ordering is still enforced transitively.
- The purge-list comment is correct for what it covers. The header's completeness claim was already untrue before this lane and belongs to L4.

Note: origin/main has moved one docs-only commit (e4dd046, CLAUDE.md) past the merge base. Merging it should not conflict.
