# Review r2: pilot privacy amendment (r2 note) and the built pages

**Verdict: FAIL** (reviewer agent, 2026-11-21; filed by the controller, since the agent's hook refuses writes outside the main checkout). Of r1's 20 findings, 15 were closed, 4 partly open (K3, I1, I2, I8) and 1 left for Quinn (I7). It found 4 new issues. The last round under CLAUDE.md: open items went to Quinn.

## Must-fix (all applied before the PR; see the note's §1a)
- **F1** Kept sign-ins still counted ("the two", "both") in four sentences; now worded by kind.
- **F2** Four places still said the vault is in the account (vault bullet, Supabase, retention, the terms' first section); qualified.
- **F3** The This-computer-only list missed the zyBooks/VHL replies; added (pin updated).
- **F4** The statement said "open-weights" (absent from the policy), and the consent promise did not cover whole forwarded messages; dropped, and 3.5 now covers "your email or anything found in it".
- **F5** This-computer-only text published without the §1 signatures. Resolved by Quinn's Q-P9 ("just publish even if the app doesn't reflect it"); the signatures are still owed before C30.
- **F6** Merging decides publication; C25 is not built. Resolved by Q-P9: publish now by hand deploy from the merged commit.

## Decisions for Quinn (resolved per recommendation, note §1a)
- **D-1** Publish timing: Quinn ruled publish now (Q-P9).
- **D-2** Q-P4: applied as D6 says.
- **D-3** Pilot form: "deleted when the pilot ends, or sooner if you ask".
- **D-4** Mailbox disconnect sentence scoped: fingerprints and untaken items keep their own times.

## Minor
- **F7** The sale paragraph points at *Who else touches it* (applied).
- **F8** "keeps only what is" vs the decision records: left as is.
- **F9** A site.rs comment about the footer: left as is.
- **F10** Line endings: `git ls-files --eol` shows i/lf for every changed file.
- Platform words ("Windows Credential Manager", the cache sentence) are left to the Mac lane.

## Passed
§3.0–3.8 and §4 match the note sentence for sentence; the launch-bump text matches the rewrite; the legal-demand exception, decision-record retention and forwarded-mail timing each read the same everywhere; no script on any legal page; the pages use tokens/base/legal.css; the screenshots render cleanly in light and dark, on desktop and phone.
