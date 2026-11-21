# Review r1: legal pages amendment, pilot privacy (spec mode)

**Verdict: FAIL.** 3 critical, 9 important, 8 minor. Reviewer agent, 2026-11-21. The agent's hook refused writes outside the main checkout, so the controller filed this from its returned text (condensed, every finding kept).

## Critical
- **K1** Per-profile setting vs per-account purge: a second computer on "Your account" re-uploads the vault after the purge. Make it an account setting the server enforces; test that a second profile's push is refused.
- **K2** 3.5's legal/attack reading cases contradict privacy.html's "No human at Knowlu reads it without your explicit consent" (tasks) and the Gmail paragraph's "No human at Knowlu reads your mail without your explicit consent" (Google's verification reads it). Amend both or narrow 3.5; material change for PRIVACY_CHANGES.
- **K3** Text assumes two kept sign-ins and Windows ("Both", "either one", "Windows Credential Manager"); the site says Windows and Mac, the rewrite folds a third kept sign-in (O1) into B, and roadmap PR #86 renumbers bumps (ruling 21: storage described without naming Windows; sites by kind). Word by kind; rebase onto the roadmap numbering.

## Important
- **I1** Sentences still saying the vault is in the account: What stays first paragraph, the vault bullet, the forwarded-mail table row "which your account holds", the export's "same text your account holds"; site.rs:405 pins one (add a sentence after). Backups ([BACKUP WINDOW]) make "deletes any copy" false for a while. 3.2's prefix splices into a broken sentence.
- **I2** 3.2's "still leaves" list misses judgment records (first three words of titles/organizers/senders), learned rules, calendar links, email in transit, issue reports, setup answers; §4's "nowhere else" implies offline (against D3).
- **I3** Q-P4 vs the judgments/rules retention ("until you delete your account"); mail decision records are judgments rows (origin gmail_api/forward); judgments has no address_id, claims prune at 7/30 days, so per-mailbox purge of old records needs a design choice; rules keyed on sender unaddressed; C33 is a migration, so Contract.
- **I4** Unnamed pins: site.rs:272 ("Settings → Email"), site.rs:428-435 (both 3.6 sentences; test name asserts the opposite), site.rs:16 (3.1's literal), ui_wizard.rs:270-275 needs a bare one-line <p>.
- **I5** Statement says what the policy does not: campus passwords in Credential Manager; names models; "school-related email" (personal mail is read); leaves out calendar; drops the kept-sign-in risk sentence; company list misses Link and Resend and conditions Google.
- **I6** 3.5 overpromises: "everything ... in your files or export" (site.rs:389 bans it); "never on a student's computer" (false today); "separate accounts" uncheckable; C34 misses GitHub, the mailbox host, registrar/DNS, tools connected to production.
- **I7** Statement linked from the live pilot band, which collects a Google Form the statement does not mention; C25 does not guard pilot-privacy.html; roadmap decision 14 publishes terms without a release.
- **I8** Signature gate misses the cloud design's Ruling 2 (which removed the sync switch); no fallback text if unsigned; C30 must wait.
- **I9** D7's parenthetical names a private matter in a public repo; strike.

## Minor
- **M1** 3.3 splice leaves a dangling clause; give the whole bullet. **M2** 3.4 vs D5 on failed reads ("never reached"). **M3** 3.1 "our servers keep them" → "that copy". **M4** "Your school, never" absolute; reuse the page's own school sentence. **M5** "Nobody else" omits a buyer in a sale. **M6** Tests for C35's links, banned words on pilot-privacy, the PRIVACY_CHANGES lines, the lawyer list. **M7** Legal-page header: theme toggle and phone menu need script; say both are dropped. **M8** Where C30's setting lives; `sync` run directly under it exits 0 with a named line.
