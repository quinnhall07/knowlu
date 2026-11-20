PASS with changes

# HANDOFF.md, the 2026-11-20 v0.4.0-released block: pre-push review

Diff: `git diff` on `docs/handoff-v040-released` (base `ef87fcc9`; `origin/main` is 5 docs commits ahead, none touch HANDOFF.md, so it rebases clean). Docs only, no contract-list file.

## Verified against evidence
- Tag: `v0.4.0^{commit}` = `ef87fcc9`; tag object `2795aa34` on origin. The run was created 2026-11-20T21:54:25Z on push, so "pushed 2026-11-20 (21:54Z)" holds. The tag object itself is dated 2026-11-19 22:54Z (created locally then), which is why the Release's `createdAt` reads 11-19.
- PRs: #80 merged 2026-11-19T19:19Z as `04c5f85f`, and `96e927d1` (brand icons) is in main's history. #81 as `0b5111c7`, #82 as `ef87fcc9` (both 2026-11-19T22:41Z), #83 on 2026-11-20T22:39Z as `135ebe94`. All match.
- `gh run view 44824405471`: release, push of `v0.4.0`, success. `gh release view v0.4.0`: setup.exe at 6,880,456 bytes, its `.sig`, `latest.json`. `knowlu.com/releases/latest.json` reads 0.4.0 (pub_date 22:04:44Z). The ledger's last line backs "homepage unchanged".
- Proof and hand check passed 2026-11-19, and the ledger's run notes back the follow-up items listed.
- Price: both amendments are on `main` with the signing dates stated (spec:993, :1234). The one-line summary agrees with Amendment 2026-11-17's "What still stands" and rulings (1), (2), (3), (5) and (6), and it says "every 4 weeks".
- Hygiene: no emails, account ids, grant ids or personal paths in the diff. LF only (0 CR), and `git diff --check` is clean.

## Findings

**I1 (Important): HANDOFF.md:34-35, "(r5) are on branch `docs/v041-pricing`".** This is not true of any ref:
- `origin/docs/v041-pricing` is at `a95ca620`, which is r2.
- Local `docs/v041-pricing` is at `a9f494af`: r4 plus the r4 delta review.
- r5 exists only as an uncommitted edit to the note in the `v041-pricing` worktree.
- The plan's status line still reads r4.

Consequence: the next session (planning v0.4.1–v0.4.9) checks out the branch and gets r2 from origin, which is missing Quinn's 2026-11-20 zero-customers ruling. If that worktree is cleaned up, r5 is lost. Fix: commit and push r5 before this push, or say "note r5 (uncommitted, worktree `v041-pricing`), plan r4; origin has r2".

**I2 (Important, privacy, rule 7): the block's point-release list drops the ledger's two "For Quinn" items about the site that v0.4.0 just deployed.**
- `site/index.html:39,73,306` post pilot applications to Google Forms, and `site/privacy.html` names neither the form nor Google as a collector (its Google mentions are about sign-in and Gmail only).
- `site/index.html:7,74,397` say "Windows and Mac", but the app is Windows-only.

Consequence: HANDOFF is the playbook, and the ledger is gitignored. The next session plans the point releases without these items, and knowlu.com keeps collecting pilot applications through a third party its privacy page does not disclose. Fix: add both to "Small items" (or to a "For Quinn" line), marking the first as a privacy-page change (Contract).

**M1 (Minor): HANDOFF.md:468 (not in the diff) still gates the quiet release on "`site/terms.html` carrying the 2026-10-16 prices".** Amendment 2026-11-17 (spec:1304) re-points that gate to "the Amendment 2026-11-10 prices, as Amendment 2026-11-17 amends them", so the line contradicts the new price block. Fix: replace the phrase.

**M2 (Minor): HANDOFF.md:28-32 reads as if the new price is already live.** Per spec:1382, (6) takes effect at signing, but `billing-checkout` still sells the old plans until the v0.4.1 switch, under 10-16b's gate. Link applies "once Stripe Managed Payments is in effect for checkout" (spec:135). Consequence: a reader concludes that checkout already charges $12.99. Fix: add "the code follows in v0.4.1; until then checkout still sells the old plans, gated to the pilot". The P15 refund of an unintended 4-week charge is also missing from the summary.

**M3 (Minor): HANDOFF.md:38 cites `router.js:92`, but the `t.finished` handler is on :93 (`startViewTransition` is on :92).**

**M4 (Minor): HANDOFF.md:24, "The workspace gate was 3385 passed", names no commit.** The ledger records 3385 for the icon worktree. The pre-tag gate on `ef87fcc9` records CI green but no count. Fix: name the commit or say "on the icon branch".

## Verdict
PASS with changes. Fix I1 and I2 before pushing. M1–M4 are one-line edits.
