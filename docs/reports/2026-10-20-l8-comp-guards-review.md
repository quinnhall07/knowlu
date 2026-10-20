# L8 follow-up: comp-grant guards (20261020000600): pre-push review (2026-10-20)

**Reviewer:** `reviewer`. **Branch:** `l8/comp-guards` from `498cc05`. **Verdict: PASS** (0 critical, 0 important, 2 minor).
**Purpose:** L8 review S6's minors 2-3 (`docs/reports/2026-10-20-l8-testers-s6-review.md`): `admin.comp_grant` refuses an
`ends_at` not after `now()` or more than a year out, `comp_extend` gains the ceiling, `admin.testers` gains
`entitlement_status` and `has_subscription` at the end, and §8.3 step 3 refuses a grant while a Stripe subscription is live.

## Checked

- Function bodies byte-identical to 000500 apart from the guards and `create or replace`; security invoker, `search_path = ''`,
  owner and ACL kept; revokes restated word for word; `compGuardsProblems` catches all nine mutations it is tested with.
- `admin.testers` keeps its eight columns (names, order, types); the two new ones come last (text, boolean); no API role
  has select on the view or execute on either function (asserted by the proof).
- Money paths: null, past, `now()` and over-a-year dates refused before any write; a grant to 2027-01-31 on 2026-10-20 is
  103 days out; `comp_extend` up to 365 days succeeds, the exact one-year edge checked in seconds.
- The 000500 proof's P8/P12/P13 keep their meaning (grant a day out, then backdate); no residue (`pg_temp` only, rollback).
- Pins: functions 49 -> 51, views 9 -> 10. "models" absent from the migration. LF; `@example.invalid` only.
- Controller's ruling (delegated): an `incomplete_expired` subscription, stored as `canceled`, may be granted; Stripe's
  `paused` (stored as `none` with an id) is refused, the safe direction.
- Deno re-run: 1235 passed, 0 failed.

## Minors (closed before commit)

1. The spec's §3.3, §5 and §9 did not mention the guards or columns: dated follow-up notes added to §3.3 and §5, a
   G1-G10 row to §9.
2. The new §8.3 line was one 520-character line: wrapped to the document's indent and width.

## Staging rehearsal (controller)

`begin; 000600; comp_guards proof; testers proof; rollback;` ran clean. A copy with one refusal message changed raised G2,
so the proof is live. Residue after: 000600 not applied, `admin.testers` 8 columns, `comp_grants` 0, `comp_audit` 0,
`@example.invalid` users 0.
