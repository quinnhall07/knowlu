# T9a: pre-push review of PR A (output policy)

**Reviewed:** `b2d8fb4b` on `security/output-policy`, rebased onto `origin/main` `530f5776`.
**Verdict: PASS.** No Critical or Important findings; two Minor. Filed by the controller from the
reviewer's returned message (the reviewer's write hook refuses paths outside this checkout).

## Checks re-run on `b2d8fb4b`

- CI's `deno test` line: 1432 passed, 0 failed; the worktree stays clean. The new tests pass with
  `--allow-read` alone (no network, no env).
- `deno check` with `ci.yml`'s new glob exits 0; `deno lint` clean over 203 files; `deno fmt --check`
  clean on the touched files.
- LF in index and worktree for all 8 files.
- Scope: nothing under `docs/`, `engine/`, `app/`, migrations or `judge_prompts.ts`; no version bump;
  prompt-hash tests pass unedited. `EMAIL_TIERS` is re-wrapped only, same values in the same order.
- `tlds.ts:18` `IANA_VERSION` equals §9 exactly. Nothing fetches the list.

## Integration

- `validate()`: task `importance_reason`, event `why`, email `why` and `title` go through §4.3's
  order (line characters to spaces, `oneLine` unclipped, `enforceFreeText`). Tier-2 rule answers use
  the same `validate()`. The email `due` goes through `realDue`.
- `mailClaim()`: model `why`, `evidence`, `change` `target` and `item.title` are policed only for
  `source === "model"`; a refusal drops that claim alone.
- Template claims are byte-identical to before (`report.pdf.example.com` kept, H9).
- No other model text reaches a device: the three `model.complete` call sites all feed these
  validators; `mail_resolve` returns no free text.
- Probes: look-alike dots, angle-bracketed and protocol-relative URLs and full-width hosts are
  neutralised in `why`; a clean value with a soft hyphen is returned byte for byte; a two-claim
  message keeps the clean claim; the worst input found (560 × U+FDFA) takes 20 ms.

## What honest students lose

Only refused names: an email `title` (the email answer is lost) or a mail `item.title` or `target`
(that claim is lost).

- Kept: `Node.js project`, `Python os.path exercise`, `console.log debugging`, `Ch. 3.2 problems`,
  `U.S. History essay`, `Ph.D. seminar response`, `C# Program.cs`, `Midterm 1 study guide (Ch.1-5)`,
  `Quiz at 5:30pm`.
- Refused, already accepted: `Amazon.com case study` (Q1's cost); `java.io`, `self.name`;
  `hw3.ml`, `alu.sv`; `Review.Study`.
- An `evidence` quote over 560 characters refuses its claim (4× ruling; the prompt asks for ≤140).

## Public-repo pass

The commit message and test names describe the policy, not what earlier validators missed. Hosts
use `example` or reserved names; IPs are documentation ranges; addresses are `@example.*` or
`.invalid`. No real person, school, LMS, project ref or secret; no single-user assumptions.

## Findings

**Critical:** none. **Important:** none.

**Minor 1.** Honest titles with generics or an inequality are refused: `Lab 7: Stack<T>
implementation`, `ArrayList<String>`, `Prove a<b for all n` (`output_policy.ts:318`, from
`judge_validate.ts:189`, `:369`, `:391`). The student gets no card for that email or claim. The
code follows §4.3; no accepted list names the case. Recommendation: add it to §4.3.1's accepted
residual, give T4 one honest twin of this shape so T7 counts it, and do not narrow the rule in PR A.

**Minor 2.** Stale or dangling comments: `detect_test.ts:1-2` (says the file fails to load),
`detect.ts:1` and `hostile_test.ts:1` (cite a plan path not yet on origin), and the test name at
`output_policy_test.ts:700`. None reveals anything about earlier validators. Fix in PR B.

## Notes

- `eval-gate` runs live on PR A (`judge_validate.ts` matches its pattern). The seed is task and event
  only, whose fields are neutralised rather than refused, so a failure is unlikely.
- §9's base SHA still says `c61c7cec`; PR A now sits on `530f5776`.

## T10 deploy list

16 functions bundle `_shared/judge_validate.ts` (static import walk and `deno info` agree):
`events`, `gmail-read`, `google-callback`, `google-connect`, `ingest-calendar`, `ingest-coursework`,
`ingest-ics`, `judge-email`, `judge-event`, `judge-rules`, `judge-task`, `mail`, `mail-process`,
`mail-pull`, `mail-resolve`, `telemetry`.

Not bundling it: `account`, `billing-checkout`, `billing-jobs`, `billing-portal`, `entitlement`,
`issues`, `mail-canary`, `mail-inbound`, `stripe-webhook`, `sync-pull`, `sync-push`.

In each project, deploy only the listed functions that `supabase functions list` already shows there.
