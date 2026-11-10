FAIL

# Waitlist design, spec review round 1

**Reviewed:** `docs/specs/2026-11-09-waitlist-design.md` (draft of 2026-11-09), in spec/plan mode.
**Code checked against:** `origin/main` at `530f5776`, read with `git show 530f5776:<path>`. The local
checkout was not used for code.
**Verdict:** needs a revision round. I found no Critical findings: nothing as written breaks an
installed updater or opens the list to anon. Eleven Important findings would ship a broken or dishonest
piece if executed as written. Four of them touch the release path or the privacy page.

## §17, settled or moved by this read

- **U1.** Five commits after `bfea03be` do not change any file the spec's design relies on, apart from
  these:
  - `engine/tests/workflows.rs`: now `all_workflows()`, which pins every workflow automatically.
  - `.github/workflows/security.yml`: new. It runs zizmor and actionlint on every workflow.
  - `release.yml`: the cache was removed and `persist-credentials: false` added, so lines move by about 1.
  - `migrations_test.ts`: a new definer guard and new "stamped last" pins.
  - `20261109000100_definer_trigger_execute.sql`: revokes `handle_new_user`, nothing else.

  §3.4's posture already satisfies the new definer guard (`assertDefinerRevoked`: a revoke that names
  both `anon` and `authenticated`). The cites that drifted are listed in M1.
- **U12, settled.** Tag `v0.1.5` carries `PRIVACY_VERSION = "2026-10-20"` and `TOS_VERSION =
  "2026-09-10"`. `site/privacy.html:12` (version 2026-10-20) and `site/terms.html:12` (2026-09-10) on
  `530f5776` match. §11.3 step 6 would therefore pass today, provided `v0.1.5` is the latest *published*
  GitHub Release. git cannot show that; S12 does.
- **U16, mostly settled.** The purge closure in `functions/account/index.ts:99` takes only `id`. But the
  same handler's deps already include `tombstone(hash, at)`, so the handler holds the address. W3 passes
  the key into `purge` or adds a dep beside it.

## Important

**I1. §11.3, "`release.yml`'s `publish` changes order" (spec lines 1095-1100), and step 3 (line 1067).**
The reorder makes the app public before its page, and it assembles from an ambiguous "latest".
- *How it fails, part 1.* Today Pages deploys first and then `gh release create` runs
  (`release.yml` publish job). The reorder runs `gh release create` first.

  Take a tag that moves `PRIVACY_VERSION`, the alternative under Q2. The repository is public, so the
  installer is downloadable from GitHub Releases at once. `knowlu.com/privacy` still shows the old
  version until `deploy-site` finishes. That wait is longer under the new production environment's
  required reviewer (§11.3, line 1059), and indefinite if the step-6 guard fails.

  Anyone who installs from GitHub in that window accepts a page version that is not published. That
  breaks "PRIVACY_VERSION never ships before a release deploys that page".
- *How it fails, part 2.* Step 3 rebuilds `releases/` from "the latest published GitHub Release" (U10).
  If "latest" is not the tag that just ran (GitHub can fall back to commit-date ordering, or a second tag
  can land in between), the release-mode deploy assembles the previous version. Step 4's version check
  passes against that release's own tag, and the run goes green with the new version never reaching
  the updater.
- *How it fails, part 3.* Step 1 says "the checkout of `main`". In a `workflow_call` from a tag push,
  `actions/checkout` defaults to the tag. With an explicit `ref: main`, a tag cut behind a `main` that
  carries an unreleased privacy bump fails guard 6, and that release never deploys.
- *Smallest fix.*
  - `publish` runs `gh release create --draft`.
  - `deploy-site` takes a `tag` input. In release mode it assembles from that tag's draft assets and
    that tag's `site/`. Site-only mode uses the latest published release, `main`'s `site/` and the live
    byte check.
  - Only after the production checks pass does the release job publish the draft (`gh release edit <tag>
    --draft=false`).

  This keeps "page live before app public" and removes U10 from release mode. It also leaves site-only
  deploys during the window failing closed, because the live bytes are new while "latest" is still the
  old version.

**I2. §11.3, the production environment (line 1059) and the preview target (lines 1053-1054,
1091).** The reviewer gate can be bypassed, and the secrets may not reach the job.
- *How it fails.*
  - A Cloudflare API token with Pages rights can deploy any branch of the project. If the preview job
    holds the same token, it can `pages deploy … --branch main` without the production reviewer.
  - `workflow_dispatch` runs `site.yml` *as it exists on the chosen branch*. A branch edit that changes
    the preview step's `--branch` deploys production unreviewed.
  - Separately, a job that calls a reusable workflow cannot declare `environment:`. If the Cloudflare
    secrets are `release`-environment secrets (U11; `publish` declares `environment: release`),
    `release.yml` cannot pass them down. The called job must declare an environment that holds them.
- *Smallest fix.* Put the token in one environment that both targets use (or make preview a separate
  Pages project with its own token). Give that environment a deployment-branch policy of `main` and
  `v*` tags, and have each job declare it. Then say plainly in §11.3 and §12 what the reviewer gate
  covers. Settle U11 before W5, because it decides which of these shapes works.

**I3. §13.5 and §15 W5: CI contracts the spec does not list.**
- `workflows.rs::release_runs_only_on_version_tags_and_names_every_secret_the_script_needs` requires
  `secrets.CLOUDFLARE_API_TOKEN` and `secrets.CLOUDFLARE_ACCOUNT_ID` in `release.yml`. Under I2's
  environment shape (or `secrets: inherit`), those names leave `release.yml` and the test goes red.
- `release_builds_and_publishes_in_two_jobs_joined_by_an_artifact` requires `publish` to carry
  `cloudflare/wrangler-action@`. It also counts `environment: release` exactly twice, with a comment
  saying the gate on `publish` is "the half that puts bytes in front of people". After the move, that
  gate no longer covers Pages.
- `security.yml` (new) runs zizmor and actionlint on every workflow. `secrets: inherit`,
  `${{ inputs.target }}` or `${{ vars.* }}` interpolated into a `run:`, a missing `persist-credentials:
  false`, or a missing per-job `permissions` would each fail it.
- GitHub concurrency keeps one *pending* run per group and cancels the older pending one. A site-only
  dispatch queued behind a running deploy therefore cancels a release's pending `deploy-site`. That run
  shows as cancelled, not red, and the updater stays on the old version until someone reruns it.
- Neither §11.3 nor W5 names any of this. "Added to the SHA-pin list at `:32-47`" is moot now:
  `all_workflows()` pins every file.
- *Smallest fix.* W5 names all three tests, states the explicit `secrets:` mapping or the environment
  shape, and requires zizmor and actionlint to be clean. Concurrency goes at job level. The release job
  re-checks that the live `latest.json` equals its tag's manifest and fails red if not. W5 also adds a
  negative pin: `site.yml` names no `TAURI_SIGNING_*`, no `AZURE_*` and no `id-token`.

**I4. §4.1, "Nothing is logged that names a person" (line 321), against `functions/_shared/db.ts:28-36`.**
- *How it fails.* `db.ts`'s `ok()` writes the upstream body to the function log verbatim, and its comment
  says that is "on purpose". A Postgres check-constraint failure carries "Failing row contains (…)", the
  whole row with the address in it. A raised exception or a unique violation on `confirm_hash` or
  `invite_code` carries "Key (…)=(…)". Any waitlist RPC through `db.ts` therefore puts an address in the
  Supabase function log.
- §13.2's log test passes unless the stub database echoes the address.
- *Smallest fix.* The waitlist modules use their own RPC wrapper that logs the status and a fixed label
  only. §13.2 adds a case where the stub returns 400 with the address in the body, and asserts the
  captured log does not contain it. The same goes for resolver errors, which carry the domain.

**I5. §3.5's join table (lines 228-236), §3.6 (lines 260-262), §4.4's effects (lines 421-423) and §4.7
layer 2 (line 476): opt-outs are not durable.**
- *How it fails.*
  - A stranger types a victim's address. The victim presses "remove this address now" in the confirm
    mail, which deletes the pending row. The next day the stranger re-submits and the victim gets
    another confirm mail. This repeats daily without end, because each re-join also resets the 30-day
    sweep.
  - The same holds 30 days after any unsubscribe or **complaint**, when the row is deleted.
  - §7.3 treats the confirm mail as commercial. By the spec's own reading, that is commercial mail to an
    address that opted out, which CAN-SPAM forbids once 10 business days have passed.
  - Mailing an address that marked us as spam also costs `knowlu.com` reputation (I6).
- *Smallest fix.*
  - Keep a suppression row: a keyed HMAC of `email_key`, the reason and the date. Write it on every
    leave, complaint and hard bounce, including a pending leave.
  - A join for a suppressed key answers `sent` and mails nothing.
  - Lifting a suppression is a person's own act: a re-join through the leave page's token, or a request
    to support.
  - Cap confirm mails per key, for example 3 in 30 days, so the "one a day" bound is finite.
  - Add a privacy-page sentence for the suppression record. The policy choice goes to Quinn and the
    lawyer (see the decisions for Quinn below).

**I6. §7.6 and §4.6 (lines 467-470): the shared Resend account couples the list to sign-in.**
- *How it fails.*
  - Resend's suppression list (scope **unverified**: account- or domain-wide) adds anyone who marks a
    waitlist confirm mail as spam. That person's *sign-in codes* are then withheld too, and they cannot
    sign in to a product they pay for. §4.6's "that list can only withhold mail, so nothing needs to flow
    back" is exactly the harm.
  - Confirm mails, the class most likely to draw complaints (sent to addresses strangers type), go from
    `EMAIL_FROM`, the sign-in sender. Q6 moves only *updates* to a subdomain.
  - Both projects share one account (§7.6). The staging and production defaults of 40 each add up to 80
    of the believed 100 a day, leaving 20 for both projects' sign-in and billing mail.
  - §4.6 lets joins open before the bounce and complaint webhook exists, so complaints on confirm mails
    go unrecorded.
- *Smallest fix.*
  - Settle the suppression scope (add it to U5).
  - Send all list mail, confirms included, from its own subdomain, or a separate Resend account if
    suppression is account-wide, *before production joins open*.
  - Set staging's default budget to single digits.
  - Make `WAITLIST_WEBHOOK_SECRET` a gate on production joins as well as on sends.

**I7. §4.2 checks 3-4 (lines 337-341) and §4.7: the limits shut out real students and are easy to bypass.**
- *How it fails.*
  - `waitlist_hit('mail:day')` runs before the address is checked, so every request that passes
    `Origin` counts, including invalid addresses and in-window repeats that send nothing. Forty scripted
    requests from eight IPs close the form for everyone until UTC midnight, with no mail sent.
  - The `ip:` limits (5 per 10 minutes, 20 a day) treat a campus egress IP or carrier-grade NAT as one
    person. A dorm group chat passing an invite link, the very path §5 builds, hits `busy` after 20.
  - An IPv6 client rotates through its /64 and is never limited.
- *Smallest fix.*
  - Charge `mail:day` only where `waitlist_join` claims a mail, in the same statement, and keep the
    pre-address *read* so the answer stays independent of the address. The residual oracle needs an
    exact remaining count to probe; say so.
  - Bucket IPv6 by /64.
  - Make the per-IP daily limit a burst limit, or raise it well above 20.

**I8. §9 says more than the design does.**
- §9.2: "we send one email … and nothing more goes to it until someone clicks the link". The join table
  sends another confirm on every out-of-window re-join (I5), and "deleted after 30 days" runs from the
  *latest* request.
- §9.1: "Nothing else". The rows also hold `left_at` and `left_reason` (which records that you marked a
  mail as spam or that it bounced), `mail_claimed_at`, the `waitlist_sends` log, and the keyed per-IP
  count, which is pseudonymous personal data. §9.1 is the page's closed list (`privacy.html:33`), so
  each of these must be on it.
- §9.2 and §7.4: "at most twice a month". No gate in `waitlist-send` enforces it, and no test proves it.
- §9.2: "short logs". This is **unverified** (U14), and the line asserts it anyway.
- §9.3: "sees the address a message is going to". Resend also keeps the message, including the confirm
  and leave tokens, for its own retention period (**unverified**).
- *Smallest fix.* Reword each sentence to match the design after I5. Add a 409 gate in `waitlist-send`:
  refuse while two broadcasts have started in the last 30 days (from `waitlist_sends`), with a test.
  Drop "short" until U14 settles, and say that Resend holds a copy of each message.

**I9. §9.7's recommendation against `site/privacy.html:166`.** The page defines a smaller change as "a
description that names no new category, company or use". It names "a new category collected" as a
change that matters, with an email and an ask "before the new use begins". §9.1 adds a category under
that exact heading. The recommendation is therefore not a borderline literal reading: it breaks the
page's published rule, and that page is the one the app records acceptance of.
- *Smallest fix.* Recast Q2 so the options are the version bump (after I1's draft-first order, its cost
  is one release and one re-consent screen) and a separate, unversioned early-access notice page that
  the account policy points to without listing a category. Recommend one. My suggestion is the bump.

**I10. §6.1, line 618 ("The site offers $6.99 a month for the first year…"), and §14.3: no gate on
showing the offer.**
- *How it fails.*
  - No signed amendment holds this offer. §14.3 has no step that keeps it off production until Q4's
    amendment, the billing spec and `terms.html` exist.
  - If the port's copy carries it (the prototype does, per line 618), production makes a price promise
    to consumers that nothing in billing honours.
  - Consent entry `c1` (§10.2) names "the founding price" and is append-only, so the words outlive a Q4
    that declines the offer.
  - Q4 also conflates this offer with Amendment 2026-10-16b's quiet-release "discounted price"
    (cloud design :808, :830). If every quiet-release buyer gets the discount, "Join before ⟨date⟩ and
    pay $6.99" implies an exclusivity that is not true.
- *Smallest fix.*
  - Add a §14.3 gate: no `offer_version` in the registry, and no offer text on any page, until Q4's
    amendment is signed.
  - Leave the founding price out of `c1`, or let `c1` wait on Q4.
  - Make Q4 state whether a non-member who buys during the quiet release pays the discount.

**I11. §13 and §15: execution order and unlisted pins.**
- *How it fails.*
  - `waitlist_terms_test.ts` (W1) pins the site's consent and offer text against `site/`. It goes red
    on `main` until the port lane (W7, out of scope) lands.
  - §13.6's DOM test lives outside `site/` and has no `ci.yml` step. Like `app/assets/mail`, it needs
    one, and `ci.yml` is in no task, so CI never runs it.
  - If W7's index merges before W5 and P3, the next app tag ships it through today's publish path:
    without `waitlist-config.json` and with the `_headers` placeholder unfilled. Production then shows a
    form that cannot submit, and the privacy entry §9.1 requires may not be live yet.
  - The new migration also has to edit unlisted pins: `assertDefinerRevoked`'s
    `files.at(-1) == definer_trigger_execute`, the three `LATER_LANES` and "stamped last" lists, and
    `assertEquals(parsed, 51, …)`.
  - W2 (a migration with about eleven definer functions, five functions, three shared modules, the
    config and a sweep) is several tasks in one.
- *Smallest fix.*
  - Move the site-text pins into W7.
  - Add the `ci.yml` step to W7, run without `--allow-net`.
  - Order W5 before W7, and gate W7's merge on P3.
  - List the migration pins in W1.
  - Split W2 into the migration, then join/confirm/leave, then send/events.

## Minor

- **M1. Stale cites.**
  - HANDOFF `:40, :44, :383, :436, :461, :465` do not hold these facts at `530f5776`. "text/plain" and
    m148 are at `:46`, `monthly_spend` at `:42/:46`, and the Resend key items at `:563` and `:566`.
  - Cloud design `:756-764` and `:788-818` should be `:763-797` and `:798-837`.
  - `migrations_test.ts`: `:2103` should be `:2080-2182`, and `:2557-2563` should be `:2580-2640`.
  - `workflows.rs`: `:32-47` should be `:47-56`, `:152-175` should be `:163-192`, and `:327-372` should
    be `:322-443`.
  - §2 line 111: "SHA-pinned (in `ci.yml` and `release.yml` only)" is no longer true.
- **M2. HANDOFF:563 records a ruling** that "a site-only deploy workflow is not taken" and that
  `release.yml` is the only deploy path for `site/`. §11 and Q7 should cite it and say that W-D3
  supersedes it.
- **M3. Rule 1, header line 24:** an absolute path on the author's machine (redacted before commit) names a person's machine. Use the
  branch name only.
- **M4. Q4's arithmetic (§6.2, line 648; Q4, line 1390).** With the June-August pause (monthly only), a
  founding monthly year is 9 × $6.99 = $62.91, against $79.98 for two terms. "A founding member who
  wants the cheapest year picks the term" is therefore wrong for anyone who pauses.
- **M5. §7.3, lines 717-720, preview.** Friends are real recipients. The preview sends them a mail §7.3
  calls commercial, with no postal address. Either set P1's PO box before the preview, or state that the
  preview's confirm mail is treated as transactional, as a lawyer item.
- **M6. The consent record (§3.1, §3.5).**
  - It does not record which confirm mail's words were sent.
  - `consent_version` and `form_id` are what the client asserts.
  - The proof is deleted 30 days after a leave, while CASL and GDPR put the burden of showing consent
    on the sender for mail already sent.

  Add a confirm-template id. Put "how long consent proof outlives a leave" to the lawyer (§8 item 8).
- **M7. §5.3, line 554.** A third party's out-of-window re-join with their own code replaces
  `referred_by` on a pending victim and takes the credit. Also, a form without the offer (`invite`)
  denies founding to someone who joined the same day as a `home` joiner. Freeze `referred_by` at first
  set, and show the offer on every form while it runs.
- **M8. §4.5.** Resend's API rate limit (**unverified**) and the function's wall-clock limit are not
  addressed. Updates draw on `mail:day`, so a broadcast closes the join form for the day. Pace the
  sends, cap each call, and reserve part of `mail:day` for joins.
- **M9. §13 gaps.**
  - No test that the answer returns while the mailer never resolves (the timing claim).
  - No test that a leave between two pages withholds the second.
  - No test of the code-collision redraw.
  - No pin on the CSP's contents.
- **M10. §4.5 and §3.6 admin objects.** The grants for `admin.waitlist_*` are unspecified. Mirror
  `admin.comp_*`: revoke from `public, anon, authenticated, service_role`. Note that the SQL editor's
  history keeps any address typed into `admin.waitlist_entry(address)`.
- **M11. §7.3 subject.** "Confirm your early access to Knowlu" reads as access already granted. "Confirm
  your address for Knowlu's early-access list" says what the mail is (CAN-SPAM, non-deceptive subject).
- **M12. §11.3 steps 4-5.**
  - Say how the `v` prefix is stripped when the version is compared with the tag.
  - Step 5 must handle a live 404 (a release with no manifest).
  - Fetch with `-OutFile` (or `curl -o`), because `Invoke-WebRequest`'s `.Content` re-encodes the body.
  - `_headers`' `/*` rule adds headers to `/releases/*`. That is harmless for the updater, but the test
    "no rule names `/releases`" does not see it.
- **M13. §14.3.** Name HANDOFF:566's open "roll the exposed Resend key; which key is live" as a
  precondition of P3. Commercial mail must not start on a key that was pasted into a chat.
- **M14. §15 W5.** "The main session *may* add a `contract-reviewer` read" should say *must*. The updater
  of every installed copy depends on this diff.

## §8, the legal reading

- The reading matches the FTC's guide on these points: a postal address that may be a street address, a
  USPS PO box or a CMRA private mailbox; an opt-out within 10 business days that works at least 30 days
  after a send and asks for nothing but the address and one page; no false headers; non-deceptive
  subjects; no transfer of opted-out addresses; responsibility for whoever sends on our behalf.
- Primary-purpose classification goes to the lawyer as item 2.
- "Not legal advice" is stated (lines 26 and 781).
- Gaps are I5 (durable opt-out), M5 and M6. Also add to lawyer item 3: whether CASL needs the mailing
  address and contact details shown in the form's consent request itself, not only in the mails.

## Checks that passed

- **Anon access.** Anon and authenticated cannot read, count, write or execute the list: RLS with no
  policy, revokes including `service_role`, definer transitions granted to `service_role` only, no
  `public` view. The new definer guard is satisfied.
- **Enumeration.** No endpoint's status, body or headers depend on whether an address is listed. The
  only residual is I7's budget oracle.
- **Link scanners.** No `GET` changes anything, and the confirm token travels in the fragment.
- **IP handling** is minimal as described: a keyed, dated, truncated HMAC, swept within 48 hours.
- **Tests stay on the machine.** Every listener is `127.0.0.1:0` and the resolver is injected.
- **Personal data in the spec.** No personal email address, real student, real school, business entity
  or co-founder is named; only M3's path.
- **Frozen references and the contract list** are untouched unless Q2 moves `PRIVACY_VERSION`.
- **Releases.** `deploy-site` builds and signs nothing, and the signing key stays in `build`.

## For Quinn (what §16 should keep)

1. **Q2.** Recast per I9: a version bump on the next release, or a separate unversioned notice page.
   *Recommend:* the bump.
2. **Q3.** One first-party script on the index. This reverses `site.rs`'s "decision 11". The download
   button comes off the index. *Recommend:* as the spec.
3. **Q4.** The offer, plus I10's question: does a quiet-release buyer outside the list also get the
   discount? Also M4's corrected arithmetic. *Recommend:* show no offer until it is signed.
4. **Q5.** What "move up" buys. *Recommend:* as the spec.
5. **Q6, widened.** A paid plan, and list mail (confirms included) on its own subdomain, or its own
   Resend account if suppression is account-wide (I6), before production joins. *Recommend:* before
   joins, not before the first broadcast.
6. **Q7.** The shared deploy with draft-first ordering (I1), superseding HANDOFF:563's ruling.
   *Recommend:* yes.
7. **Q8.** Preview against staging. *Recommend:* as the spec, with P1's PO box first (M5).
8. **Q9.** When the lawyer reads. *Recommend:* as the spec.
9. **New: opt-out durability (I5).** Keep a keyed suppression record of addresses that left, complained
   or bounced, and send nothing to them on a stranger's re-join. *Recommend:* yes, lifted only by the
   person's own act; the lawyer confirms.
10. **New: how long consent proof outlives a leave (M6).** *Recommend:* the lawyer sets it. Until then,
    keep the minimal consent fields under the suppression record.

Q1 (Gmail folding) can stay as the spec's default. It is low-stakes, and W-D2 already reserved it.
W-D5 (double opt-in) should be ruled before the plan is written.
