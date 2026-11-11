FAIL

# Plan review r2: security leftovers (advisor gate, client table rights, guard gaps, loopback proxy)

**Document:** `docs/plans/2026-11-11-security-leftovers-plan.md` (r1, 383 lines), worktree `knowlu-leftovers`, branch
`security/leftovers` at `0926f0c1`. **Mode:** spec/plan, round 2, the last. **Reviewer:** `reviewer`, 2026-11-11.

**Verdict: FAIL.** Of the r1 findings, I3 and M1 to M10 are closed. Nothing new breaks the migration, the pgTAP
plan, T2, T3 or the close. I1 and I2 are mostly closed, but the new rules they introduced have two Important holes:
- **N1:** read literally, the normaliser fails two real definers in the corpus.
- **N2:** the re-grant reader does not say where the grantee list starts or how quotes are treated. One way to read
  it gives a false pass on the quoted form `supabase db diff` writes. Another gives false failures on three real
  definers.

Both are one-clause corrections. This is the last round, so they go to Quinn below, each with a recommendation.

## r1 findings

| r1 | Status | Checked against |
|---|---|---|
| I1, argument lists | **Mostly closed; see N1 and M-a.** `FUNCTION_DEFINITION` stays unchanged, and the scan balances parentheses and skips literals. By hand, every definer whose list is not empty normalises equal to its last revoke. | `20260911000200_google.sql:106-186`, `20260911000300_google_privileges.sql:28-87`, `20260929000200_gmail_disconnect_purge.sql:27-44`, `20261002000200_mail.sql:371-388` |
| I2, statement forms | **Mostly closed; see N2.** Cases 6 and 7 cover every form r1 named, plus `procedure` and `all routines`. | plan R4 (c), T1a |
| I3, the WARN's reason | **Closed.** F-A, R6 and §7 record it only as an Auth setting left to `HANDOFF.md:573`, and give no reason. | `HANDOFF.md:573` |
| M1, red predictions | **Closed, with one new slip (M-b).** `:249`/`:251`, four of the five F-G tests, and `:1988` staying green are all right. | `09:249-256`; `migrations_test.ts:1988, 3342, 3517, 3647` |
| M2, C10 and C11 | **Closed.** The naming uses `nspname`. C11 reads both namespaces and has a global plant. O8 is sound: a global entry's new ACL starts from the built-in defaults, so the plant reads `{PUBLIC:TRIGGER}`. | R3, T1a, O8 |
| M3, `/i` | **Closed.** | T1a corpus tests, §4 |
| M4, the pin | **Closed.** The exact `run:` line is pinned, `\|\| true` is planted, and the list of forbidden strings is complete against `workflows.rs:471-547`. | R1, T3 |
| M5, manual proof | **Closed.** Both tests run before and after, and once more with `NO_PROXY=example.org`. | T2 |
| M6, T2's agent | **Closed.** The test and config line are written out (checked below), and the controller does T2. | T2 |
| M7, one index | **Closed.** Tasks run in sequence, with `git commit -- <paths>`. | R7, Gates |
| M8, the close | **Closed.** C4 runs from a worktree cut from the branch's head. All three queries are inline (checked below). | close |
| M9, images | **Closed.** `docker images` is listed before and after the advisor at the green run. | T1b |
| M10, Deno | **Closed.** | §7, R6 |

## Important

**N1. An empty argument list fails closed, so the corpus fails T1b.** Location: R4 (b) step (1) and its fail-closed
clause, T1b.
- Splitting `()` on top-level commas gives one empty item, and R4 (b) fails closed on "an empty argument". Case 5's
  `(p uuid,)` throws only because that rule exists, so an implementer will build it.
- Two definers have `()`: `handle_new_user` (`20260910000100_accounts.sql:113`, re-created at
  `20260917000100_oauth_consent.sql:28`, revoked at `20261109000100:18`) and `trim_user_metadata`
  (`20260922000100_trim_user_metadata.sql:53`, revoked `:59`). Their revokes are `()` too.
- *Scenario:* T1b follows the plan as written, and `assertDefinerRevoked` throws on `handle_new_user`. The plan
  sends a failing real definer to the controller, so T1b stops for a round trip on a spec bug.
- *Fix:* "a list that is empty or only whitespace is zero arguments; an empty item inside a non-empty list fails
  closed". Add a synthetic case: `f()` revoked as `f()` passes.

**N2. The re-grant reader does not define where the grantee list starts or how quotes are treated.** Location: R4
(c), and (a) for the from-list. It says "grantees (read before any `with grant option` or `granted by`) include
public, anon or authenticated", but not where the grantees begin, and not whether `"anon"` is `anon`. The two
obvious ways to build it each fail.
- **False failures on the corpus.** Suppose the matcher tokenises the whole statement. Then `public` from a
  schema-qualified name counts as a grantee. Three definers have a qualified grant after their last revoke:
  `20261109000100:19` (`public.handle_new_user() to supabase_auth_admin`), `20260922000200:4`
  (`public.trim_user_metadata() to supabase_auth_admin`) and `20261002000200_mail.sql:388`
  (`public.delete_mail_address(uuid) to service_role`). Each would read as a client re-grant. Case 6's "a later grant
  to `service_role` passes" does not catch this if its grant is unqualified.
- **A false pass.** Suppose the matcher compares tokens without removing quotes. Then `grant execute on function
  "public"."f"(uuid) to "anon";`, the fully quoted form `supabase db diff` writes, is not seen as a re-grant. That is
  the exact gap (c) closes. `identPattern` already accepts quoted names for this reason
  (`migrations_test.ts:38-45`, R-C2-E37c).
- The same quote problem in (a) makes `from "public", "anon", "authenticated"` a false failure.
- *Fix:*
  - Grantees are the comma list after the top-level `to` (or `from`, for a revoke) that follows the object list's
    last item, up to `with grant option`, `granted by`, `cascade`, `restrict` or `;`.
  - Each grantee is unquoted and compared as a whole token.
  - Case 6 adds `grant execute on function public.f(uuid) to service_role` (passes) and `… to "anon"` (throws).
    Case 1 or 2 gains a quoted from-list that passes.

## Minor

**M-a. Two details of the normaliser.** Location: R4 (b) steps (4) to (6).
- **The steps run in the wrong order.** Step (4) splits into tokens before step (6) collapses whitespace. So case 5's
  revoke item `numeric(10, 2)` splits into `numeric(10,` and `2)`, the first is taken for a name, and the expected pass
  throws. Collapse whitespace inside parentheses, or split only on top-level whitespace, before step (4).
- **"Keep type modifiers" (step 5) does not match Postgres.** Postgres ignores typmods in a function's signature. So
  `revoke … f(numeric)` is a correct full revoke of `f(p numeric(10,2))`, and the guard fails it closed. In the same
  way, `varchar(20)` misses step (6)'s whole-token `varchar` mapping. Both are false failures only, and the corpus
  has none.
- *Recommended:* drop the modifiers, then change case 5's middle half to "revoked as `(numeric)` passes".

**M-b. One red prediction is still wrong.** Location: T1a "Expected red". On the unchanged guard, case 5's
`(numeric(10, 2))` half is red, not green. `revokeRe`'s `\([^)]*\)` stops at the inner `)`, the `\s+from` that
follows does not match, and the unchanged guard throws. If M-a is taken, the half becomes `(numeric)`, which matches
`revokeRe` and is green, and the prediction then holds.

## Checked and sound

- **The other definers by hand.** These normalise equal to their last qualifying revoke and have no client grant
  after it: `store_google_grant` (both definitions, `uuid,text,text,text,text[]`), `delete_google_grant` (both),
  `read_google_grant`, `read_google_grant_any` and `delete_mail_address`. Search from the definition's offset (d),
  then the last revoke: each one is later in the scan than its definition. `google.sql:183-186`'s `from public`
  revokes do not qualify, and none of them is the last. The comment-stripped corpus has no `grant` inside a body with
  `on function`: the `'… grant …'` literals in `20261020000500_testers.sql` and `20261020000600_comp_guards.sql` have
  no `on (function|routine)` target.
- **Cases 3, 4 and 7.** Steps (1) to (6), with M-a's order, produce what each case expects. In case 4, the `inout`
  item keeps `boolean`, and the revoke's named `p_n integer` loses its name; the result is `integer,timestamp with
  time zone,boolean,text[]` on both sides. The red predictions for 1, 2, 3, 6, 7 and 8 hold.
- **T2's code.** It compiles as written: `hosts.contains(&host)` is `&&str`, and `include_str!("../../.cargo/config.toml")`
  resolves the same way as `WORKSPACE`. The asserted line equals the TOML line byte for byte, and the file is LF
  (`lines()` would strip a CR anyway). It goes red on (ii) without the line, and on (i) without `force` on CI. The
  comment line sits inside `[env]` and does not change the table.
- **The inline queries.** They are valid PG 17:
  - `0::regnamespace::text` and the PUBLIC grantee's `::regrole::text` both print `-`, so the `nullif` gives
    `PUBLIC`.
  - The `oid`/`regrole` and `int`/`regnamespace` `IN` lists resolve through implicit casts.
  - `privilege_type` is upper case, `MAINTAIN` included.
  - F2's query matches `tests.client_functions` without the session-only helper.
  - At the red run all three return rows (26 relations; postgres's schema entry; the two triggers).
- **pgTAP:** 870 = 858 + 2 (01) + 10 (09). The global C11 plant is transactional and rolls back with `under_fault`.
  O8's fifth statement keeps the migration test exact at five.
- **I3's public record** now makes no claim about the hosted Auth API.

## For Quinn

Each item is a correction to the plan's text, not a choice between designs. Under each recommendation, the
controller folds the clause into the plan before T1a, without a third review, and T5 checks it in the diff.

1. **N1, the empty list.** *Recommended:* add "an empty list is zero arguments; an empty item in a non-empty list
   fails closed", and add the case `f()` revoked as `f()` passes. *Cost if left:* T1b stops on `handle_new_user` and
   `trim_user_metadata`.
2. **N2, grantees and quotes.** *Recommended:* define the grantee and from-list as the list after the top-level
   `to`/`from` that follows the object list. Unquote each grantee and compare it as a whole token. Add the qualified
   `service_role` case (passes), the quoted `"anon"` case (throws) and a quoted from-list (passes). *Cost if left:*
   either three real definers fail falsely, or a db-diff-style quoted grant to `anon` passes the static guard. F1
   would still catch the second in `cloud-rls`, but that check is not required yet.
3. **M-a and M-b, the normaliser's order and modifiers.** *Recommended:* collapse whitespace before step (4),
   drop modifiers instead of keeping them, change case 5's middle half to `(numeric)`, and keep the prediction as it
   stands. *Cost if left:* case 5's passing half throws at T1b; there is no false pass.
