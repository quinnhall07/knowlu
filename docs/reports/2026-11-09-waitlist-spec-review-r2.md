FAIL

# Waitlist design, spec review round 2 (final)

**Reviewed:** `docs/specs/2026-11-09-waitlist-design.md`, revision 1, against round 1
(`2026-11-09-waitlist-spec-review-r1.md`).
**Code checked against:** `origin/main` at `530f5776`, with `git show 530f5776:<path>`. I did not use
the `security/rls-pgtap` branch the author read.
**Verdict:** the revision closes 23 of round 1's 25 findings in its text and partly closes two (I8, M1).
It adds design, and that design carries one Critical and five Important problems. All six are bounded
engineering fixes with no open design choice. Under the house rule there is no third round, so they go
to Quinn below as "accept the fix", and the planner folds them into the plan.

## Round 1, finding by finding

| ID | Status | Note |
|---|---|---|
| I1 | closed | Draft-first ordering, a `tag` input, and the pages taken from the tag's checkout (§11.3). The new mechanics raise N1-N3 |
| I2 | closed | The `pages` environment with a branch policy and a required reviewer, the separate `knowlu-preview` project, and a pin on the preview job. §11.5 states plainly what the gate covers. See m2 |
| I3 | closed | Both `workflows.rs` tests are named and changed, zizmor and actionlint must stay clean, concurrency runs in separate job-level groups, and there are negative pins (§11.6) |
| I4 | closed | `_shared/waitlist_db.ts` logs only a status, each function has its own catch, and four echoing-stub tests prove it (§4.1, §13.2) |
| I5 | closed | Suppression on every leave, complaint, bounce and deletion; 3 join-path mails per 30 days; lifting is the person's own act (§3.8). C1 undermines the confirm path itself |
| I6 | closed | A separate list account, key, sender and subdomain; joins refuse without the webhook secret; the budgets fit one quota (§7.6). See N5 |
| I7 | closed | The budget is read before the address and charged at the claim; IPv6 is grouped by /64; the IP limit is a burst limit; the residual oracle is stated honestly (§4.2). I found no further oracle in timing or budget behaviour |
| I8 | partly | §9 now names every stored item and Resend's copy, and drops "short". "Never more than two in any 30 days" is enforced per broadcast, not per recipient (N4) |
| I9 | closed | A version bump is recommended; the separate notice page is the alternative (§9.7) |
| I10 | closed | The offers list is empty and pinned; nothing under `site/` contains "6.99" or "founding" (none does at `530f5776`, so the pin starts green); `c1` names no price; Q4 is widened |
| I11 | closed | The site-text pins move to W7, the DOM test gets its own CI step in W7's files, W5 lands before W7, the form stays hidden without its config, the migration pins are listed, and W2 is split. See m9 |
| M1 | partly | Every revised cite holds at `530f5776` except `ci.yml` (m8) |
| M2 | closed | §11.1 and Q7 |
| M3 | closed | No machine path remains |
| M4 | closed | §6.3's arithmetic table is right: $62.91, $83.88, $79.98, $89.91 |
| M5 | closed | The PO box comes before the preview (§14.1) |
| M6 | closed | `confirm_template`, the consent restated in the confirm mail, and the consent facts kept on the suppression record |
| M7 | closed | `referred_by` is frozen at first set, and the server sets the offer |
| M8 | closed | 600 ms pacing, a per-call `limit`, and separate send and join budgets |
| M9 | closed | The tests named in §13.2 and §13.4 |
| M10 | closed | The `admin` grants mirror `admin.comp_*` (`testers.sql:176-179` verified); the SQL editor's history is named (U18) |
| M11 | closed | |
| M12 | closed | |
| M13 | closed | P0 |
| M14 | closed | W5 must be read by `contract-reviewer` |
| r1 §8 (CASL) | closed | Lawyer item 3, widened |

**Cites the revision touched, checked at `530f5776`.** These hold:
- `HANDOFF.md` `:42`, `:46`, `:50`, `:52`, `:510`, `:563`, `:566`, `:593`, `:596`;
- cloud design `:342`, `:763-797`, `:772-780`, `:798-837`, `:810`, `:829-831`;
- `db.ts:29-38`, `http.ts:65-69`, `entitlement.ts:111-126`, `account/index.ts:191`,
  `account/handler.ts:96`;
- `migrations_test.ts` `:286`, `:347`, `:350`, `:368`, `:533`, `:584`, `:1988`, `:2080-2182`,
  `:2580-2640`, `:3336-3348`, `:3509-3520`, `:3642-3649`;
- `testers.sql` `:10`, `:62-63`, `:176-179`;
- `release.yml` `:273-312`, `:281`, `:294-298`, `:312`;
- `workflows.rs` `:13-23`, `:47-56`, `:66-77`, `:163-192`, `:322-443`;
- `security.yml:19-49`, `privacy-guard.ps1:34`, `cloud/supabase/README.md` `:92`, `:128`, `:151-152`.

Only `ci.yml` moved (m8).

**Personal data and single-user assumptions.** Nothing new names a personal address, student, school,
entity or co-founder. "Quinn" appears only as the decision-maker and as the `pages` reviewer, which is a
GitHub setting, not repository content.

## Critical

**C1. §3.5, the join rule (spec lines 297 and 301-307): a re-join that mails nothing still replaces the
token.**
- The `pending` row says "a new token (the old one is superseded)". Step 3 then says "Otherwise the
  record change above still happens for a `pending` entry (a re-join refreshes its token), and the
  action is `none`."
- *How it fails, for the person who joined.* A student presses "Get early access" twice. The first press
  mails token T1. The second, inside the 24-hour window, stores T2's hash and mails nothing. T1 now
  answers `invalid` (§4.3: "superseded by a newer token"), and the student has no working link until
  they join again more than 24 hours later. Each such cycle also spends one of the address's three
  mails.
- *How it fails, for a stranger.* Re-submitting a pending person's address within 24 hours of each mail
  voids that person's link every time. Three cycles use up the 30-day cap, so the person cannot confirm
  for a month.
- *The spec's own smoke shows it.* S4 ("Join again at once … then confirm") followed by S6 ("Confirm from
  the mail") would answer `invalid`.
- *The same bug on the leave page.* §4.4's `rejoin` issues a new token "under the usual claim", so a
  rejoin inside 24 hours of the last join-path mail, or with the cap spent, leaves a `pending` entry with
  an unmailed token, while the page says "check your inbox".
- *Smallest fix.*
  - Write a new token, and refresh the consent fields, only in the same statement that claims a mail.
  - When no mail is claimed, change nothing.
  - `rejoin` is the owner's own act from their mailbox, so it skips the 24-hour window but still respects
    the budget and the cap. With no mail claimed, it answers a "try again tomorrow" state.
  - Add a §13.4 row: a join, an in-window repeat, then a confirm with the first token answers
    `confirmed`.

## Important (new design)

**N1. §11.3 step 3 and §11.4 step 2: GitHub hides drafts from a read-only token.** GitHub's API lists
draft releases, and serves their assets, only to callers with push access, which for `GITHUB_TOKEN` means
`contents: write`.
- *How it fails.* Under §11.6's top-level `permissions: {}` and a read-only job, site-only mode's "refuses
  while any draft exists" sees an empty list and always passes. The guard that keeps a site-only deploy
  out of a release in flight is then decorative.
- In release mode, step 3 (U17) can only work if the job that holds the Cloudflare token also holds a
  write token.
- *Smallest fix.*
  - Release mode takes the bytes from this run's own `release` artifact (`actions/download-artifact`). A
    called workflow runs inside the caller's run, so the artifact is there, and it holds the same bytes
    the draft was made from. `go-public`, which already has `contents: write`, compares the draft's
    `latest.json` with the live one.
  - Site-only mode lists drafts in a separate job with `contents: write` and no Cloudflare token, and
    passes a yes or no to the deploy job.
  - S12 creates a draft and confirms that a site-only run refuses.

**N2. §11.4 step 2 and §11.6, "stays a draft, which is visible and harmless": a stranded draft blocks
site-only deploys forever.**
- *How it fails.* Any leftover draft blocks every site-only deploy until someone deletes it. Drafts are
  left over when:
  - a tag is superseded (two pending runs in `pages-release`, the older cancelled);
  - the `site` job fails (N3's tree check, the policy guard, a rejected approval, or GitHub's 30-day
    approval expiry);
  - someone makes a draft by hand in the UI.

  It gets worse when the Pages deploy succeeded but `go-public` failed:
  - the site serves vN+1 while vN+1 stays a draft;
  - deleting the draft to unblock leaves site-only comparing the latest published vN with a live vN+1,
    which refuses forever.
- A rerun of `publish` after a partial failure creates a *second* draft with the same tag name.
  `gh release download` and `gh release edit` are then ambiguous.
- *Smallest fix.*
  - Scope the refusal to drafts whose `v*` tag is newer than the latest published release.
  - `publish` refuses if a draft for its tag already exists, or reuses it.
  - Write the recovery into §11 and W8. After a Pages deploy, rerun `go-public`
    (`gh run rerun <id> --failed`). Before one, delete the draft and keep the tag. A superseded tag's
    draft is deleted.

**N3. §11.3 step 1: `site/` at the tag must equal `site/` at `origin/main`.**
- *How it fails.*
  - The check runs after a build of about 40 minutes. Any merge that touches `site/` in that window (the
    port lane, a page fix) fails a release that is already signed, and strands its draft (N2).
  - A hotfix tagged on a branch from an older commit is refused once `main`'s `site/` has moved. The named
    fix, "tag `main`'s head", pulls unreleased app work into the hotfix.
- *Smallest fix.*
  - Make the rule what the check is for: never roll back pages already published. `deploy-site` writes
    the source commit into each deployment (for example `site-source.txt`). Release mode refuses only if
    the live source's `site/` tree differs from the tag's *and* that source is not an ancestor of the
    tag.
  - Name the hotfix procedure: bring `main`'s `site/` onto the hotfix branch.

**N4. §4.5's frequency gate counts broadcasts by their first send, not by recipient.**
- *How it fails.* A broadcast that takes days to send under `mail:send:day` (broadcast A, recipient X
  mailed on day 10) and a second broadcast B (day 5 to 12) let a third broadcast C pass the gate on day
  31. X then gets three updates between days 10 and 31. §9.2's per-person "never more than two in any 30
  days" breaks.
- *Smallest fix.* `waitlist_batch` skips any entry with two send rows in the last 30 days, and the 409
  gate stays. Add a test with a staggered send.

**N5. §4.6: an event without a project tag is dropped, with no fallback.**
- *How it fails.* Whether Resend puts tags in webhook payloads is unverified (U5). If it does not, every
  complaint and bounce is dropped silently. The "complaints are recorded before anyone is mailed" gate
  then checks only that a secret is set. The only live proof, S14, runs "if [Resend] offers" test
  addresses.
- *Smallest fix.* Name the fallback now: one sender address per project, matched on the payload's
  `from`, or one list account per project. Make S14 unconditional, using a real test inbox that marks a
  confirm mail as spam if no test address exists.

## Minor

- **m1.** The `pages` environment puts Quinn's approval in front of every app release's site deploy.
  - That fits "tags on Quinn's word".
  - §11.3, §12 and W8 (`docs/reference/app.md`'s release steps) should say it explicitly.
  - "Prevent self-review" must stay off, or the person who pushed the tag cannot approve.
  - An approval left for 30 days strands the draft (N2).
- **m2.** The `pages` branch policy admits any `v*` tag. Anyone who can push a tag on an arbitrary commit
  runs that commit's `site.yml` with the token once the deploy is approved, and the approver does not see
  the workflow diff. A tag ruleset that limits who can create `v*` tags closes this. The same exposure
  exists today for `release`'s signing secrets.
- **m3.** The `--branch deploy-check` alias publicly serves a production-configured site, installers
  included, without noindex. Either the deploy adds `X-Robots-Tag` for that branch, or the spec says it
  accepts that.
- **m4.** `waitlist_forget_key` with zero rows (§3.5): say that no suppression record is written.
  Otherwise every deleted account leaves a list fingerprint that §9.5 does not disclose.
- **m5.** §4.8 says `WAITLIST_ADDR_KEY` is "never rotated", while §3.8 describes a rotation that checks
  under both keys. Align them, and keep the old key for lookups only until its records expire (3 years at
  most).
- **m6.** §4.6: the events path must HMAC the *lower-cased* address, because `key_hash` is the HMAC of
  `email_key`.
- **m7.** §10.5's "rejoined" state assumes a mail was sent. See C1's fix for the state it needs when none
  was.
- **m8.** `ci.yml` cites are still `bfea03be` numbers. `:165-167` is now `:158-171` (the
  `--allow-net=127.0.0.1` comment and the `deno test` line). `:171-173` is now `:174-177` (the mail
  scripts' step).
- **m9.** W1 edits `migrations_test.ts`'s corpus counts and `files.at(-1)`, and those pass only once W2a's
  migration exists. W1 and W2a land in one pull request, or the pin edits ride with W2a.
- **m10.** U19: GitHub's reusable-workflow documentation says a called workflow's job that declares
  `environment:` reads that environment's secrets. Keep W5's dry run as the proof.
- **m11.** U21: to my knowledge, Cloudflare's Pages token permission is account-wide, not per project.
  Plan as if the preview token can deploy production. §11.5 already holds without the scoping.

## New design that passed

- **Keyed suppression.** The `admin` grants, the sweep, erasure by `lift` or `forget`, under-13s with
  `suppress => false`, and the §9 disclosure. Erasure against the record stays with the lawyer.
- **The join and send budgets.** They are separate; the join budget is charged at the claim; sends never
  close the form; the residual probe is stated and accepted. I found no other membership oracle in
  answers, status codes, timing or budget behaviour.
- **Separate concurrency groups.** A manual run can no longer cancel a release's pending deploy.
- **The `knowlu-preview` project.** Its own environment, no `releases/`, noindex, and staging
  configuration.
- **The configuration gate.** It refuses without the postal address, the webhook secret or the list's own
  key.
- **No egress in tests.** The DOM test runs without `--allow-net`.

## For Quinn

### Accept as fixes (no design choice; the planner folds them into the plan)

1. **C1:** write a token only together with a mail claim; `rejoin` skips the 24-hour window. *Recommend:
   accept.*
2. **N1:** release bytes from the run's artifact, and a separate write-scoped job to look for drafts.
   *Recommend: accept.*
3. **N2:** scope the draft refusal, guard against a second draft, and write down the recovery.
   *Recommend: accept.*
4. **N3:** compare the tag with the live deployment's source, not `main`'s head, and name the hotfix
   procedure. *Recommend: accept.*
5. **N4:** a per-recipient frequency limit in `waitlist_batch`. *Recommend: accept.*
6. **N5:** a sender-based fallback for webhook events, and an unconditional S14. *Recommend: accept.*
7. Minors m1-m11 go to the plan as written.

### §16, keep, merge or drop

- **Keep Q2** (version bump). *Recommend: the bump.*
- **Keep Q3** (one script, no download button). *Recommend: as the spec.*
- **Keep Q4** (offer, widened). *Recommend: show no offer until it is signed; one discount for list
  members and quiet-release buyers alike.*
- **Keep Q5** (move up). *Recommend: as the spec.*
- **Keep Q6** (a separate Resend account). *Recommend: yes, before any list mail.*
- **Keep Q7, widened** to include m1: every app release now waits for your approval in the `pages`
  environment, and a `v*` tag ruleset (m2). *Recommend: accept both, and supersede the HANDOFF:563
  ruling.*
- **Keep Q8** (preview against staging). *Recommend: as the spec.*
- **Keep Q9** (when the lawyer reads). *Recommend: as the spec.*
- **Merge Q10 and Q11** into one decision. Both concern the same suppression record and its consent
  facts: what it holds, how long, and whether an erasure request removes it. *Recommend: a keyed
  fingerprint plus the consent facts, 3 years by default, the period set by the lawyer (§8 items 8 and
  9).*
- **W-D5** (double opt-in) still needs Quinn's word before the plan. *Recommend: double opt-in.*
- **Q1 stays dropped**, as a default.
