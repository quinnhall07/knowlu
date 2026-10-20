# L8 friends-testers spec, Amendment 2026-10-20: review

Spec: `docs/specs/2026-10-18-friends-testers-design.md`, Amendment 2026-10-20 (S1 result; account-id grants).
Reviewer, spec mode. Filed by the controller from the reviewer's returned text.

**Verdict: FAIL (0 critical, 3 important, 7 minor).** The rebinding is sound and nothing in it is critical. Three text
fixes are needed before Quinn re-reads it.

## Confirmed

- **No stale text.** No live use of the address binding, the trigger, P4/P7/P10/P11, `email_hash` or "waiting for
  sign-up". Every remaining mention is marked retired (:1033-1039, :1108, :1122, :822). The pin arithmetic holds:
  S2c1 adds three, S2c2 one and S2d one, five in all (:40, :779). The stamp note follows M2 (:855-858).
- **No free path for a non-tester.** Only the owner writes grants. The service role's writes are revoked and `admin`
  usage is revoked (:371-385). The view joins only on `g.account_id = a.id` and reads nothing from `auth.users` or any
  email (absence pin :1106-1109). P14 and P16 cover the wrong-account and no-account cases at the view.
- **The cascade claim is right.** PostgreSQL runs a referential action as the owner of the referencing table. A3s
  copies the real purge: `restDelete(rest, "accounts", …)` as the service role, `functions/account/index.ts:81`. So the
  service role's revokes cannot block it.
- **Export.** It reads public tables only. It exposes no `auth.users` column and no `encrypted_password`.
- **Rule 1.** The spec holds no personal identifier. Addresses are `@example.invalid` or elided, and tripwire (a)
  still makes sense (:527-530).
- **S2a0 stands.** `29dcdc2` touches only `migrations_test.ts`: a one-line `identPattern` change plus appended
  synthetic tests, no SQL and no pin. Its parent is on `origin/main`, and `git merge-tree` against `origin/main` is
  clean.

## Important

1. **§8.3 check 1 matches on a stale address** (:710-711). `accounts.email` is copied once at sign-up
   (`20260910000100_accounts.sql:127`, `20260917000100_oauth_consent.sql:32`) and never follows an auth email change.
   P15 itself relies on that gap (:1145-1146).
   - *Scenario:* a stranger password-signs-up with the friend's address. They then change their auth email away,
     which works with confirmations off (§8.3 already names email change). The friend has not actually finished
     signing in, or used Google on another address. Check 1 finds exactly one row, the stranger's. If it was created
     close to the friend's stated time, check 2 passes. The comp lands on an account the friend cannot use.
   - *Fix:* check 1 joins `auth.users u on u.id = a.id` and requires `lower(u.email) = lower(<address>)` as well.
     Check 2 also requires `u.last_sign_in_at` inside the friend's stated window, which proves a sign-in finished, not
     just a code request.
2. **"No database check can see it (S1)" claims more than S1 tested** (:415, :707). S1 read `auth.users` columns only
   (:6-11). It never looked at `auth.identities`, `auth.sessions` / `auth.mfa_amr_claims` (`authentication_method`)
   or `auth.audit_log_entries`. §8.3's recommendation ("the first prod grant does not wait for C1b", :719-721) and
   Quinn's answer rest on that claim.
   - *Fix:* before the controller deletes S1's two scratch users (:51-53), read those tables for both users and record
     the structure only. If any of them separates a password sign-in, add it as check 2b. Otherwise reword both lines
     to "no column of `auth.users`".
3. **Q7 sentence 2 leaves out a stored field** (:612-615). The grant row also holds `ended_reason`
   (`done`/`asked`/`misuse`/`other`, :390-391). It is exported (:417) and kept until the sweep (Q11 :652, "dates,
   note, reason"). `misuse` is a judgment about the person, and the page does not mention it.
   - *Consequence:* L4 bumps `PRIVACY_VERSION` on an incomplete page, and the version moves twice (§8.5 :753-754).
   - *Fix:* "…the dates of that access, a short note on why and, if it ended early, a one-word reason, kept until 30
     days after it ends…"

## Minor

1. **A3 and A3s should hit both referential paths in one delete** (:1047-1048). State that the deleted account holds a
   grant made through `admin.comp_grant`, so its audit row's `grant_id` is set. Then the delete fires the cascade from
   `accounts` and the set-null from `comp_grants` together, and has a `route_daily` row too.
2. **The `comp_audit` export columns are not pinned** (:1199-1203). §3.5 :418 lists four fields. A `select=*`, the
   style of :94-96, would also export `db_role` and `grant_id`. Pin the select list or widen the text.
3. **§8.3 is missing two checks.** Before granting, confirm `entitlements.status = 'none'`; a friend who pressed a
   price has a `trialing` row that wins, Q1 :566-569. After granting, confirm the `admin.testers` row shows the
   friend's address.
4. **"Never saved as a snippet"** (:717-718, :621-622). Recent Supabase Studio, as far as the reviewer knows,
   auto-saves editor tabs as private "Untitled" snippets. Say how this is verified (delete the tab and check the
   snippet list), or soften the sentence.
5. **The step-back visit shows the wrong line.** `console.js:2693` sets `wiz-sub-note` to "Your subscription is
   active." for any entitled account. The amendment makes the step-back visit part of the live proof (:1216-1217), so
   a tester reads "subscription". S5 and S5t's pin should cover that note when `plan === 'tester'`.
6. **Retired case rows** (S2b :1243, "kept as retired rows with no case"). Say these IDs are reserved in a comment and
   are not data rows in `testers_cases.ts`, so the loops in layers ii and iii never meet a row with no expectation.
7. **Header and rebase target.**
   - Line 3 says Quinn re-reads "Q1 and Q7", but :54 and :1263 add §8.3. Make line 3 match.
   - "rebases onto `main` (the email branch has merged)" (:48-49) is true of `origin/main` (`ba7eb0b`). The local
     `main` is stale. Name `origin/main`.
