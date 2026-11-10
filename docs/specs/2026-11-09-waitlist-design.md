# Knowlu's waitlist: early access, invites and the founding price — design

**Status:** **Signed by Quinn 2026-11-09 (rulings); r2 fixes folded in un-re-reviewed; the plan's
review covers them.** Revision 2. Spec only: no plan and no code. **Size: Contract** (CLAUDE.md
"Workflow").
- It holds personal data of people who are not account holders, sends commercial email, changes the
  privacy page, and changes how `knowlu.com` is deployed, which today is part of the release path.
- Reviews:
  - round 1 (`docs/reports/2026-11-09-waitlist-spec-review-r1.md`: FAIL, 0 Critical, 11 Important,
    14 Minor), answered in Revision 1;
  - round 2, the final round (`docs/reports/2026-11-09-waitlist-spec-review-r2.md`: FAIL, 1 Critical,
    5 Important, 11 Minor, all "accept the fix"), folded into Revision 2 on Quinn's acceptance.
- Quinn's rulings are recorded in §0 and §16. One thing stays open: the founding offer's amount, term
  and cutoff, which belong to its own amendment (§16).
- Next: the plan, with a fidelity ledger.

**Asked by:** the main session's dispatch of 2026-11-09, which carries Quinn's decisions (§0), its
revision directions, and Quinn's rulings of the same day.
**Governs:** the email list behind the marketing site's one call to action, "Get early access":
- how an address joins, is confirmed and earns invite credit;
- how the founding price is recorded;
- how the list receives updates and how a person leaves it for good;
- how the site ships without an app release.

**Authority, newest first:** the cloud design (`docs/specs/2026-09-09-knowlu-cloud-design.md`):
- Amendment 2026-10-19 (privacy bump #1);
- Amendment 2026-10-16b (`:798-837`): the pilot in spring 2027; a quiet release at a discounted price
  whose amount is open; the official launch in fall 2027; a recommended cap of about 50 outside
  sign-ups;
- Amendment 2026-10-16 (`:763-797`): prices;
- D1 (CI first), D2, D7 (Cloudflare and Supabase), §5.6 and §9.

Then `VISION.md`, `HANDOFF.md` and the privacy page as published.

**Code read:**
- The draft read `origin/main` at `bfea03be`.
- Revision 1 re-checked every cite the review named, and every file this revision newly relies on, on
  the branch `security/rls-pgtap`. Its lines match the review's `530f5776` cites wherever both were
  checked. Cites are from that tree unless they say otherwise.
- The round-1 review settled U1, U12 and U16 (§17). Round 2 checked every revised cite at `530f5776`;
  only `ci.yml`'s had moved, and Revision 2 fixes them (m8).
- Anything not confirmed is marked **unverified** and listed in §17, where the plan's reviewer should
  start.

**Not legal advice.** §8 is an engineering reading of the rules, written for a lawyer to check. A
lawyer reviews the consent wording, the privacy section and the first update email before the first
update email is sent.
**Contract list: touched.** Q2, as ruled, bumps the privacy version. That moves `PRIVACY_VERSION`
(`app/src/account.rs:25`) through `contract-engineer` and then `contract-reviewer`. No frozen reference
moves, and no engine or app source changes.

### Revision 1: each finding, and where it is answered

| ID | Finding, in short | Answer | Where |
|---|---|---|---|
| I1 | Release-first ordering made the app public before its page; "latest release" was ambiguous; the checkout ref was undefined | **Fixed.** Draft-release-first: `publish` creates the release as a draft. `deploy-site` assembles that tag's draft assets and that tag's `site/` (the assets now come from the run's artifact: Revision 2, N1). The draft is published only after the production checks pass. Site-only mode alone uses the latest *published* release and `main` | §11.3, §11.4 |
| I2 | The reviewer gate could be bypassed; the secrets might not reach the called job | **Fixed.** Production deploys through a `pages` environment (branch policy `main` and `v*` tags, required reviewer). The preview is a separate Pages project, `knowlu-preview`, with its own environment. A pin forbids the preview job from naming the production project | §11.5 |
| I3 | CI contracts the spec did not list | **Fixed.** Both `workflows.rs` tests are named and changed; zizmor and actionlint must stay clean; job-level concurrency uses separate groups, so a manual run never cancels a release's deploy; the release re-checks the live manifest before it publishes; negative pins added | §11.3, §11.6, §13.5, W5 |
| I4 | `db.ts` logs upstream bodies, which can carry an address | **Fixed.** The waitlist uses its own RPC wrapper and its own catch, and both log a fixed label and a status only. A test uses a stub that echoes the address | §4.1, §13.2 |
| I5 | Opt-outs were not durable | **Fixed.** A keyed suppression record is written on every leave, complaint, hard bounce and deletion, and a suppressed address is never mailed again on anyone's join. Join-path mail is capped at 3 per address per 30-day window. Only the person can lift a suppression. Policy is Q10 | §3.1, §3.5, §3.8, §4.4, §9, §16 |
| I6 | The shared Resend account couples the list to sign-in | **Fixed.** List mail never uses the sign-in key or sender; it has its own key, sender and subdomain, in a separate Resend account (recommended; required if suppression is account-wide). Joins refuse until the webhook exists. The budgets are resized to fit one quota | §4.6, §4.8, §7.6, Q6 |
| I7 | The limits shut out real students and were easy to bypass | **Fixed.** The join budget is read before the address and charged only when a mail is claimed; the residual oracle is stated. IPv6 is bucketed by /64. The IP limit is a burst limit with no daily cap. The global budget is the backstop | §3.7, §4.2, §4.7 |
| I8 | §9 promised more than the design does | **Fixed.** §9 is rewritten to the design: every stored item is named, Resend's copy is disclosed, there is no "short", and the frequency promise is enforced by a 409 gate with a test | §4.5, §9, §13.2 |
| I9 | Q2's recommendation broke the page's own change rule | **Taken.** Q2 is reframed; the version bump is recommended, the separate notice page is the alternative | §9.7, Q2 |
| I10 | Nothing kept the offer off production | **Fixed.** No offer is in the registry and none is on any page until Q4's amendment is signed, and a test pins it. `c1` does not name the price. Q4 is widened and its arithmetic fixed | §6, §10.2, §14.2, Q4 |
| I11 | Execution order and unlisted pins | **Fixed.** Site-text pins move to W7 and the privacy-date pin to W6. W7 adds the DOM test's CI step. W5 comes before W7, and W7's merge waits for P3. The form stays hidden without its config. The migration pins are listed in W1, and W2 is split into three | §10.1, §13, §15 |
| M1 | Stale cites | **Fixed** throughout, against the revision's tree | §2 and every section |
| M2 | HANDOFF:563's ruling against a site-only deploy | **Named**, and put to Quinn as superseded by W-D3 | §11.1, Q7 |
| M3 | A machine-specific path in the header | **Removed**; the branch is named instead | header |
| M4 | Q4's arithmetic | **Fixed** (9 paid months with the pause) | §6.3, Q4 |
| M5 | The preview mailed friends with no postal address | **Fixed.** The PO box comes before the preview, so every list mail carries it | §7.3, §14.1, Q8 |
| M6 | The consent record | **Fixed.** A confirm-template id is recorded; the confirm mail restates the consent, so the proof does not rest on what the client asserts; the consent facts survive a leave in the suppression record. Retention is lawyer item 8 and Q11 | §3.1, §3.8, §7.3, §8 |
| M7 | `referred_by` could be hijacked; founding depended on the form | **Fixed.** `referred_by` is frozen at first set. `offer_version` is set by the server from the registry for every form | §5.3, §6.2 |
| M8 | Send pacing, the wall clock and the shared budget | **Fixed.** Sends are paced, each call is capped, and joins and sends have separate budgets | §4.5 |
| M9 | Test gaps | **Fixed.** Answer-before-mail, a leave between pages, the code redraw and the CSP contents are all tested | §13.2, §13.4, §13.6 |
| M10 | Admin grants; the SQL editor's history | **Fixed.** Grants mirror `admin.comp_*`; the history is named | §3.4, §3.6 |
| M11 | The confirm subject | **Fixed** | §7.3 |
| M12 | Assembly details | **Fixed.** `v` prefix, live 404, `-OutFile`, and a pin on the headers `/*` adds to downloads | §11.3, §13.5, §13.6 |
| M13 | The exposed-key roll | **Fixed.** A precondition, P0 | §14.2 |
| M14 | A `contract-reviewer` read of the deploy | **Fixed.** Now "must" | §15 W5 |
| r1 §8 | CASL: contact details in the form's request itself | Lawyer item 3, widened | §8 |
| U1, U12, U16 | Settled by the review | Recorded | §17 |

### Revision 2: round 2's fixes, folded in on Quinn's acceptance (2026-11-09)

Round 2 was the final review, so these are not re-reviewed here. The plan's review covers them.

| ID | Finding, in short | Fix | Where |
|---|---|---|---|
| C1 | A re-join that mailed nothing still replaced the token, voiding the person's link (a stranger could keep anyone from confirming) | **A new token, and refreshed consent fields, are written only in the statement that claims a mail.** With no claim, nothing changes and the earlier link keeps working. `rejoin` skips the 24-hour window, because it is the owner's own act, but keeps the budget and the cap; with no claim it changes nothing and answers `later`. Tests added | §3.5, §4.4, §10.5, §13.2, §13.4 |
| N1 | A read-only token sees no drafts, so the draft guard was decorative and release mode could not read its assets | **Release mode takes its bytes from the run's own `release` artifact** (`release.yml:255-262`). **Site-only mode lists drafts in a separate `drafts` job with `contents: write` and no Cloudflare token**, which passes a yes or no to the deploy job. `go-public` compares the live manifest with the artifact's, the bytes the draft was made from, so it needs no read of a draft either (the review suggested the draft's own copy; the artifact's is the same bytes with one less unverified step). S12 makes a draft and sees the refusal, without pushing a tag | §11.3, §11.4, §11.6, §14.1 |
| N2 | A stranded draft blocked site-only deploys forever; a rerun of `publish` made a second draft | **Site-only mode refuses only on a draft whose `v*` tag is newer than the latest published release.** `publish` reuses its tag's existing draft (`gh release upload … --clobber`) and refuses if the tag is already published. **The recovery runbook** is written down (§11.7), and "harmless" is gone. That `gh` finds a draft by tag is U23 | §11.3, §11.4, §11.6, §11.7, §13.5, W8 |
| N3 | The tag-equals-`main` check failed signed releases after a mid-build merge, and refused hotfixes | **Every production deployment carries `site-source.txt`** (its source commit). Release mode refuses only if the live source's `site/` differs from the tag's *and* the live source is not an ancestor of the tag. Site-only mode applies the same rule to `main`'s head, so it waits for a hotfix branch to be merged back. A bootstrap rule covers the first deploy, which finds no file. **The hotfix runbook** is written down | §11.3, §11.4, §11.7, §13.5 |
| N4 | The frequency gate counted broadcasts, not people | **`waitlist_batch` skips any entry that already has two send rows in the last 30 days**, so the limit holds per person on a rolling window; the 409 gate stays. A staggered-send test | §3.5, §4.5, §13.2, §13.4 |
| N5 | Events without a tag were dropped, with no fallback | **Each project sends from its own addresses**, and an event is attributed by its tag, else by the payload's `from`. One that matches neither is dropped with a fixed log label and counted. **S14 is unconditional**: a real test inbox marks a confirm mail as spam if Resend has no test address. If the payload carries neither tags nor `from`, the plan returns to Quinn before any list mail | §3.1, §4.6, §4.8, §7.3, §13.2, §14.1 |
| m1 | The `pages` approval stands in front of every app release | Said in §11.3, §11.5, §12 and W8. "Prevent self-review" stays off. An approval left 30 days is in the runbook | §11.3, §11.5, §11.7, §12, W8 |
| m2 | Any pushed `v*` tag reaches the `pages` environment | **A tag ruleset limits who can create `v*` tags** (Q7, ruled); its availability is U24 | §11.5, P1 |
| m3 | The `deploy-check` alias serves the production site without noindex | **Accepted**, with reasons | §11.3 |
| m4 | `waitlist_forget_key` on zero rows | Writes no suppression record | §3.5 |
| m5 | "Never rotated" contradicted §3.8's rotation | Aligned: a rotation keeps `WAITLIST_ADDR_KEY_PREV` for lookups only, until its records expire (3 years at most) | §3.8, §4.8 |
| m6 | The events path must hash the lower-cased address | Stated | §4.6 |
| m7 | The rejoin state assumed a mail was sent | The `later` state | §10.5 |
| m8 | `ci.yml` cites were `bfea03be`'s | Now `:158-171` and `:172-177` (the review's `:174-177` plus the step's comment), re-read for this revision | §2, §10.6, §13 |
| m9 | W1's migration-pin edits are red until W2a's migration exists | The pin edits ride with W2a | §15 |
| m10 | U19 is documented by GitHub | Recorded; W5's dry run stays the proof | §17 |
| m11 | Pages API tokens are account-wide | Planned as if the preview token can deploy production | §11.5, §17 |

**Also in Revision 2, from Quinn's rulings rather than the review:**
- the status line;
- §0's rulings paragraph;
- §16, rewritten as a "Decided" record;
- each section's "recommended" turned to "decided" where a ruling settled it (§3.6, §3.8, §5, §6, §7.6,
  §8, §9, §10, §11.1-§11.2, §12, §14, §15).

The fixes also raised three new unverified items, U23 to U25 (§17).

---

## 0. Quinn's decisions, recorded as decided

| # | Decision | Where this spec carries it |
|---|---|---|
| W-D1 | **Storage is our own Supabase**: `knowlu-staging` first, production only on Quinn's word. **No anonymous reads of the list, ever.** | §3.4, §13, §14 |
| W-D2 | **No duplicate emails**: unique on the normalized address (trimmed, lower-cased). Gmail dots and plus tags are a default this spec sets (§3.2); this was Q1, and it is no longer asked | §3.2 |
| W-D3 | **Going live**: first a private Cloudflare Pages preview link (`*.pages.dev`) shared with friends, then a site-only deploy so the site can ship without an app release. **Releases stay CI-only.** This supersedes the 2026-10-20 ruling that a site-only deploy workflow "is not taken" (`HANDOFF.md:563`), as Quinn ruled under Q7 (Quinn, 2026-11-09) | §11, §14, §16 |
| W-D4 | **The postal address is a USPS PO box that Quinn will supply.** **Update emails stay blocked until it is configured.** This spec goes further: no list mail at all is sent without it, so it comes before the preview (Quinn, 2026-11-09) | §4.1, §4.5, §7.3, §14 |
| W-D5 | **Double opt-in (Quinn, 2026-11-09).** A signup counts only after the confirm link. The form answers "check your inbox" the same way whatever the address's state. A repeat signup re-sends at most once a day and three times in a 30-day window | §3.5, §4.2, §4.3 |
| W-D6 | **Sending is Resend**, from a separate Resend account (Q6, Quinn, 2026-11-09). API or SMTP, and Resend's broadcast tooling or our own loop, are decided in §7.1-§7.2. Our table is the source of truth either way | §7 |

**Quinn's rulings of 2026-11-09 on this spec's questions** accepted every recommendation, and §16 records
them as decided. In brief:
- Q2: a privacy version bump.
- Q3: one first-party script on the index, and the download button off it.
- Q4: no offer anywhere until an amendment signs it; one discount shared with quiet-release buyers;
  Pell students pay the lower price, never both; earlier joiners qualify; the claim expires when the
  list closes.
- Q5: move-up as specified.
- Q6: a separate Resend account on a paid plan, with its own sending subdomain, before any list mail,
  staging's included.
- Q7: the shared draft-first deploy, with Quinn's approval on every release and a `v*` tag ruleset.
- Q8: the preview opens only after the PO box exists and S1-S14 pass.
- Q9: when the lawyer reads, as specified.
- Q10 and Q11, merged: the suppression fingerprint and its consent facts are kept 3 years by default,
  and the lawyer sets the final period.

**Still open:** only the founding offer's amount, term and cutoff, which are left to its own amendment
(a pricing analysis is running).

## 1. What this is, and what it is not

**In scope:**
- One list in Supabase: four tables, their transitions as database functions, and owner-only admin
  functions.
- Five edge functions: join, confirm, leave (unsubscribe), send (update emails) and events (bounces and
  complaints).
- A suppression record that keeps an opt-out in force.
- The invite route `knowlu.com/r/<code>`, invite credit and the "move up" order.
- The record the founding price will need, and the gate that keeps the offer off production until it is
  signed.
- The confirm email and the update emails, sent apart from sign-in mail.
- The privacy page's additions.
- The deploy change that lets the site ship alone without dropping `releases/`.
- Tests, rollout, and the lab disclaimer's replacement.

**Out of scope:**
- **Porting the prototype into `site/`.** That is its own lane (W7). §10 is its contract.
- **Billing the founding price**, and the offer's words. §6 names every gate.
- **Cloudflare Turnstile.** Not now; §4.7 gives the trigger.
- **A status page or a position number.** Not in v1 (Q5, Quinn, 2026-11-09).
- **Recruiting the pilot from the list.**

**Checked against `VISION.md` and the two overriding rules:**
- The waitlist adds nothing to the app; what it must not do is bend a commitment:
  - **No free tier, one edition (D2).** The founding price is a discount on the one edition, and it
    needs Quinn's amendment before any page shows it (§6.1).
  - **"One daily email at most"** (VISION, Standing rules). List mail is far rarer, and §4.5 enforces
    it.
  - **Data minimization.** An address, the facts of consent and an opt-out fingerprint. No IP address
    on any row, no name, no school, no age (§3.1, §3.7, §3.8).
  - **The account is the source of truth** (commitment 2). The list is not an account; the two meet
    only where §3.6 lets Delete my data reach a list entry.
- **Rule 1, no single-user assumptions.** Nothing in the repo names a person's address, inbox, postal
  address, key or machine. The PO box and every key are function secrets (§4.8). The admin functions
  are run by "the project owner". Test addresses are `@example.invalid`.
- **Rule 2, frozen references.** Nothing under `engine/tests/fixtures/` is touched.

## 2. What exists today

| Fact | Where |
|---|---|
| Resend relays sign-in mail over SMTP (`smtp.resend.com`, sender name `Knowlu`, `admin_email = "hello@knowlu.com"`) | `cloud/supabase/config.toml:85-92` |
| **One Resend account and key serve both projects**: `EMAIL_FROM = Knowlu <hello@knowlu.com>` on each. Resend's DNS records sit on `send.` and `resend._domainkey.` under `knowlu.com` | `HANDOFF.md:52`, `:566` |
| **The exposed-key roll is open**: the Resend key was pasted in chat, and which key is live is still Quinn's to answer | `HANDOFF.md:563`, `:566` |
| Functions send through Resend's **HTTP API** (`EMAIL_API_URL`, `EMAIL_API_KEY`, `EMAIL_FROM`; a bearer key and a JSON body) | `functions/billing-jobs/index.ts:26-36` |
| The canary sender is the house pattern for a Resend call: custom headers, a 10 s timeout, the body cancelled unread, nothing logged but the provider's status | `functions/_shared/canary_send.ts:1-11, 57-79` |
| **Supabase's relay serves an HTML response as `text/plain` on `*.supabase.co`**; pages on `knowlu.com` or a paid custom functions domain is open item m148 | `HANDOFF.md:46`, `:510` |
| Every function is `verify_jwt = false` and answers in our own shape; a test requires each deployed function to carry the entry | `config.toml:101-150`; `functions/_shared/config_toml_test.ts:6-36` |
| No function answers CORS or `OPTIONS` today | no `access-control` header under `functions/` |
| `readJson` caps a body in characters; **`asResponse` logs a stray error's `message`** before answering a bare 500 | `functions/_shared/http.ts:48-58`, `:65-69` |
| **`db.ts`'s `ok()` writes the upstream body to the function log verbatim, on purpose** | `functions/_shared/db.ts:29-38` |
| An RPC call that cancels the body unread and throws only the status: the precedent for a quiet wrapper | `functions/_shared/entitlement.ts:111-126` (`countFrom`) |
| The subscription consent reads the caller's IP as the first hop of `x-forwarded-for` | `functions/billing-checkout/handler.ts:80-88`; `migrations/20260910000100_accounts.sql:36-49` |
| Work after the answer runs through `EdgeRuntime.waitUntil` where the runtime has it | `functions/mail-inbound/index.ts:31-42` |
| Account deletion's purge takes the account id; the handler holds the address; the tombstone is an **unkeyed** SHA-256 of it | `functions/account/index.ts:99-152`, `:191`; `functions/account/handler.ts:96` |
| Migration guards: RLS on every table with no client write policy; views `security_invoker` or revoked; definer functions revoked from `anon` and `authenticated` (`assertDefinerRevoked`); no address-shaped token; each cron minute unique | `cloud/supabase/migrations_test.ts:56-81`; `migrations/migrations_test.ts:286, 350, 533, 2080-2182, 2580-2640` |
| Pins a new migration must edit: the latest file named by suffix (`files.at(-1)`), the "stamped after every other file but the named later ones" lists, `LATER_LANES`, and the corpus counts (51 function creations, 10 view creations) | `migrations/migrations_test.ts:584`, `:1988`, `:3336-3348`, `:3509-3520`, `:3642-3649`, `:347`, `:368` |
| The closed `admin` schema, revoked even from `service_role`; owner-only `admin.comp_*` functions; a table whose writes are revoked from `service_role` | `config.toml:9`; `migrations/20261020000500_testers.sql:10`, `:62-63`, `:176-179` |
| Staging proofs are SQL in a rolled-back transaction with `set local role`, run by the controller | `cloud/supabase/proofs/testers_staging.sql` |
| The RLS behaviour suite (pgTAP) is a plan on its own branch, not merged | `security/rls-pgtap`: `docs/plans/2026-11-09-rls-pgtap-plan.md` §D3-§D4 |
| Deno tests may bind `127.0.0.1` and read anything; the mail scripts' DOM tests run with linkedom in their own CI step | `.github/workflows/ci.yml:158-171` (the `--allow-net=127.0.0.1` comment and the `deno test` line), `:172-177` (the mail scripts' step); re-checked for Revision 2 |
| `site/`: `index.html`, `privacy.html`, `terms.html`, `signed-in.html`, `subscribed.html`, `site.css`, `releases/.gitkeep`; no `_redirects`, no `_headers` | `site/` |
| Site pins: no `<script` on the index or the privacy page ("decision 11"); the privacy sentence on both; the download link; `releases/.gitkeep`; four banned words | `engine/tests/site.rs:17-36` |
| Pages already serves `privacy.html` at `/privacy` and answers the `.html` form with a 308 | `HANDOFF.md:50` |
| The privacy page is version **2026-10-20** (equal to `PRIVACY_VERSION`) and Last changed 2026-10-23. It promises a closed list of categories; "a new category collected" is a change that matters, with an email and an ask; Resend is unnamed; the site has "no analytics scripts" | `site/privacy.html:12-13, 33, 86, 163, 166`; `app/src/account.rs:25`; `app/tests/static_assets.rs:1676-1686` |
| **The release publish job** (`environment: release`) downloads the artifact into `site/releases/`, deploys Pages with `pages deploy site --project-name knowlu --branch main --commit-dirty=true`, then runs `gh release create` | `.github/workflows/release.yml:273-312` (`:281`, `:294-298`, `:312`) |
| **A 2026-10-20 ruling: `release.yml` is the only deploy path for `site/`, and "a site-only deploy workflow is not taken"** | `HANDOFF.md:563` |
| `site/releases/` holds the versioned installer, a byte copy `Knowlu-setup.exe`, and `latest.json` (UTF-8 without a BOM) whose `url` names the versioned installer | `scripts/release.ps1:293-327` |
| The updater endpoint in every installed build is `https://knowlu.com/releases/latest.json` | `app/tauri.conf.json:26` |
| Workflow pins: **every** workflow's actions SHA-pinned via `all_workflows()`; `release.yml` must name `secrets.CLOUDFLARE_API_TOKEN` and `secrets.CLOUDFLARE_ACCOUNT_ID`; `publish` carries `cloudflare/wrangler-action@` and `gh release create`; `environment: release` appears exactly twice; `privacy-guard.ps1` is run against synthetic trees | `engine/tests/workflows.rs:13-23, 47-56, 66-77, 163-192, 322-443` |
| `security.yml` runs zizmor and actionlint on every workflow | `.github/workflows/security.yml:19-49` |
| The latest tag, `v0.1.5`, carries `PRIVACY_VERSION = "2026-10-20"` and `TOS_VERSION = "2026-09-10"`, matching the pages | review r1, U12 |
| Pages other systems depend on: the auth redirect `signed-in.html`; `/privacy`, which Google's consent screen names; the privacy and terms URLs set in Stripe | `config.toml:21-22`; `HANDOFF.md:593`, `:596` |
| Prices and stages: $9.99 a month; a 6-month term at $39.99; the Pell price; the June-August pause on monthly plans; the pilot and the quiet release from spring 2027; the official launch in fall 2027 | cloud design `:763-797`, `:798-837`, R3 (`:342`) |

## 3. The data

### 3.1 Four tables, all in `public`

**`public.waitlist`**, one row per address:

| Column | Type | Why it exists |
|---|---|---|
| `id` | `uuid` primary key | the entry; leave tokens are derived from it (§4.4) |
| `email` | `text not null` | the address as typed, trimmed: where mail goes |
| `email_key` | `text not null`, unique | `lower(email)`: the duplicate rule (W-D2) |
| `key_hash` | `text not null`, unique | HMAC-SHA256(`WAITLIST_ADDR_KEY`, `email_key`), in hex. The suppression key and the per-address mail cap's bucket, computed by the function at join (§3.8) |
| `fold_key` | `text not null` | the Gmail-folded form, compared only when crediting a referral (§3.2) |
| `status` | `text not null`, check `pending`, `confirmed`, `unsubscribed` | |
| `created_at` | `timestamptz not null` | the first request; the pending sweep counts from it (§3.6) |
| `requested_at` | `timestamptz not null` | the join that issued the current token; frozen at confirmation as the consent's request time |
| `confirm_hash` | `text`, unique where not null | SHA-256 of the current confirm token, kept after confirmation so a second click answers "already confirmed" |
| `confirm_expires_at` | `timestamptz` | `requested_at` plus 7 days |
| `confirm_template` | `text` | the registry id of the confirm mail sent with the current token: which words the person confirmed in answer to (§7.3) |
| `mail_claimed_at` | `timestamptz` | when the join path last sent, or began sending, a mail here: the 24-hour window |
| `confirmed_at`, `first_confirmed_at` | `timestamptz` | the latest confirmation; the first, which is the place in line and the founding test |
| `left_at`, `left_reason` | `timestamptz`, `text` check `link`, `one_click`, `complaint`, `bounce` | set while `unsubscribed` |
| `consent_version` | `text not null` | the consent words the form showed, by registry id. The form asserts it and the function checks it against the registry; the server-sent confirm mail restates the consent, so the proof does not rest on it alone (§7.3) |
| `form_id` | `text not null` | which form, from an allow-list (supporting evidence only) |
| `offer_version`, `offer_cutoff` | `text`, `timestamptz` | **set by the server** from the offer live in the registry at the request, whatever the form; both null while no offer is signed (§6) |
| `founding` | `boolean not null default false` | frozen at the first confirmation (§6.2) |
| `invite_code` | `text`, unique where not null | issued at the first confirmation (§5) |
| `referred_by` | `uuid references public.waitlist(id) on delete set null` | set by the first join that carries a valid code, **then never changed** (§5.3) |
| `credited_at` | `timestamptz` | when this entry's confirmation credited its referrer; once only |
| `credits` | `integer not null default 0`, check `>= 0` | referrals credited to this entry |

There is no IP address, user agent, name, school or age column. `migrations_test.ts:46-54` already
forbids a birthdate.

**`public.waitlist_suppressed`**, one row per address that must not be mailed (§3.8):
- `key_hash text primary key`;
- `reason text not null`, check `link`, `one_click`, `complaint`, `bounce`, `deleted`;
- `suppressed_at timestamptz not null`;
- the consent facts copied from the entry as it goes: `consent_version`, `confirm_template`,
  `requested_at`, `confirmed_at`, and `last_mailed_at` (the latest list mail). All are null for an
  address that never confirmed.

No address is stored.

**`public.waitlist_throttle(bucket text, window_start timestamptz, hits integer, primary key (bucket,
window_start))`**. The buckets:
- `ip:<keyed hash>` (§3.7);
- `addr:<key_hash>`, the per-address cap on join-path mail (§3.8);
- `mail:join:day` and `mail:send:day`, the two daily budgets (§4.2, §4.5);
- `events:unattributed:day`, a count of webhook events no project claimed (§4.6). It holds a number,
  never an address.

**`public.waitlist_sends(broadcast text, entry_id uuid references public.waitlist(id) on delete cascade,
sent_at timestamptz, provider_id text, primary key (broadcast, entry_id))`**. One row per update email
sent. It makes a send resumable and idempotent, and it feeds the frequency gate (§4.5).

**Why one row per address and not an event log.** The consent proof is the request time, the words
shown, the confirm mail sent and the confirmation time, and all four sit on the row. They move to the
suppression record when the row goes. A log would hold the same address many times for no named use.

### 3.2 Normalization, and the Gmail default

- **`email`**: leading and trailing ASCII whitespace removed. Anything else wrong is refused, never
  repaired (§4.2).
- **`email_key`**: `lower(email)`. No mainstream provider treats the local part as case-sensitive, and a
  case-sensitive key would let one person sit on the list twice.
- **Gmail dots and plus tags: folded only to judge referral credit, never for identity.** This is a
  default, not a question (it was Q1):
  - The identity is the mailbox that clicked the confirm link. Dots mean nothing only at `gmail.com`
    and `googlemail.com`, and plus tags are a convention some systems do not honour, so a folded unique
    key would merge mailboxes that are distinct somewhere.
  - The cost of not folding is that one person can hold two entries and must confirm each.
  - `fold_key` is computed at join. For `gmail.com` and `googlemail.com`, dots are removed from the
    local part, the `+tag` is stripped and the domain is written `gmail.com`. For every other domain,
    only the `+tag` is stripped.
  - `fold_key` is compared only when a referral is credited (§5.3).

### 3.3 Indexes

- `waitlist`:
  - unique on `email_key`, on `key_hash`, and on `invite_code` and `confirm_hash` where not null;
  - `(status, first_confirmed_at)` for the invitation order;
  - `referred_by` and `fold_key`;
  - two partial indexes for the sweep: `created_at where status = 'pending'` and `left_at where status
    = 'unsubscribed'`.
- `waitlist_suppressed`: its primary key, and `suppressed_at`.

### 3.4 Row-level security and rights: no anonymous reads, ever

- **RLS on all four tables, with no policy** (`migrations_test.ts:56-81`).
- **`revoke all … from public, anon, authenticated, service_role`** on each table, as `comp_grants`
  does (`20261020000500_testers.sql:62-63`). Every function in this project holds the service key, so a
  mis-aimed `restUpsert` anywhere is refused by the database, and the list is reachable only through
  its own transitions (§3.5).
- **Every transition is a `security definer` function** with `set search_path = public, extensions`.
  Each is owned by the migration owner, with execute revoked from `public`, `anon` and `authenticated`
  and granted to `service_role` alone. That is `assertDefinerRevoked`'s shape
  (`migrations/migrations_test.ts:533`) and the RLS plan's F1 and F2.
- **The owner's objects live in `admin`**:
  - `admin.waitlist_counts` (a view), and the functions `admin.waitlist_order(n)`,
    `admin.waitlist_entry(address)`, `admin.waitlist_forget(address)` and `admin.waitlist_lift(address)`;
  - `admin.waitlist_forget` takes `suppress => true` by default; `false` is only for a child's entry and
    for the controller's own test inboxes (§3.6, §14.2);
  - each is revoked from `public`, `anon`, `authenticated` and `service_role`, as `admin.comp_*` are
    (`20261020000500_testers.sql:176-179`), on top of the schema revoke (`:10`);
  - no view in `public` reads the list, not even a count. A count is a business fact, and a view is one
    grant away from the anon key (the `monthly_spend` lesson, `HANDOFF.md:42`).
- **On the wire:** with the anon key, `GET /rest/v1/waitlist`, a `HEAD` asking for an exact count, and
  `POST /rest/v1/rpc/waitlist_join` each answer a permission error (§13.4, §14.1 S1).

### 3.5 The transitions, and the registry they check against

The functions never write a table. They call these:

| Function (`public`, definer) | Called by | Effect | Returns |
|---|---|---|---|
| `waitlist_join(email, email_key, key_hash, key_hash_prev, fold_key, token_hash, expires, consent_version, form_id, confirm_template, offer_version, offer_cutoff, invite, join_budget)` | join | the rule below, in one transaction. `key_hash_prev` is null except after a key rotation (§3.8) | the action (`confirm_mail`, `listed_mail` or `none`), the entry id, and the invite code for `listed_mail` |
| `waitlist_mail_result(entry, ok)` | join, after the send | on a failed send, releases the row's claim, the `addr:` hit and the budget unit, so a retry can mail | nothing |
| `waitlist_budget_left(bucket, limit)` | join, send | reads a daily budget without charging it | integer |
| `waitlist_confirm(token_hash)` | confirm | §4.3's outcomes. On a first confirmation it also issues the code (redrawn on a collision, up to three times), credits the referrer (§5.3) and freezes `founding` (§6.2) | outcome, invite code and `founding` |
| `waitlist_leave(entry, action, reason, token_hash, expires, join_budget)` | leave, events | §4.4, writing the suppression record in the same transaction. For `rejoin`, the token arguments and the budget are used only if the claim succeeds (r2 C1) | outcome (`left`, `deleted`, `sent` or `later`) |
| `waitlist_suppress_key(key_hash, reason)` | events | suppresses an address that has no entry (a complaint or bounce after deletion) | nothing |
| `waitlist_forget_key(email_key)` | `account`'s purge | deletes the entry with that key and writes its suppression record. **Zero rows is success and writes no suppression record**, so a deleted account with no list entry leaves no fingerprint (r2 m4) | nothing |
| `waitlist_hit(bucket, window, limit)` | join, confirm, leave, events (the unattributed count) | adds one hit and answers whether the bucket is over its limit | boolean |
| `waitlist_batch(broadcast, n)`, `waitlist_sent(broadcast, entry, provider_id)`, `waitlist_broadcasts_started(days)` | send | the next page of `confirmed`, unsuppressed entries with no send row for this broadcast **and fewer than two send rows of any broadcast in the last 30 days** (the per-person rolling limit, r2 N4), with status re-read in the same statement; the send row; how many broadcasts began in the last `days` | rows, and how many were held back by the per-person limit; nothing; integer |
| `waitlist_sweep()` | `pg_cron`, daily | §3.6's deletions | nothing |
| `waitlist_new_code()` | `waitlist_confirm` only; executable by no API role | 8 random Crockford base32 characters; a separate function so a test can replace it (§13.4) | text |

**The join rule.** Every branch makes the same one call and returns quickly. Whether any mail is
claimed is decided in this order:
1. **`key_hash` is suppressed**: nothing is written and no mail is sent (§3.8).
2. **The state of `email_key`:**

| State | Effect on the record, **only when step 3 claims a mail** | Mail |
|---|---|---|
| no entry | insert `pending` with the token hash, the consent fields, and `referred_by` when the code is valid and not the address's own (§5.3) | confirm |
| `pending` | a new token (the old one is superseded); the consent fields refreshed; `referred_by` set only if still null | confirm |
| `confirmed` | nothing | "already on the list", with the invite link |
| `unsubscribed` | cannot happen, because leaving suppresses (§4.4); treated as suppressed | none |

3. **The mail is claimed only if all three hold:**
   - `mail_claimed_at` is more than 24 hours old (or null);
   - the `addr:<key_hash>` bucket has had fewer than 3 mails in its 30-day window;
   - `mail:join:day` has a unit left. **The unit is charged here, in this statement, and only here.**

   **The new token, the refreshed consent fields and the claim are written in this one statement, or not
   at all** (r2 C1). When no mail is claimed, nothing changes, the action is `none`, and the link
   already mailed keeps working.

   The draft refreshed the token on every re-join. A second press inside 24 hours then voided the
   first link and mailed nothing, and a stranger could keep anyone from confirming.

Claiming stamps `mail_claimed_at`. That keeps two concurrent joins from both mailing, and
`waitlist_mail_result` releases the claim, the `addr:` hit and the budget unit when Resend refuses.
- *An accepted edge:* the undelivered token stays in place, so an earlier link stops working. This
  happens only when our own send fails, never by anyone's choice.
- Because the claim is released, the person's next press mails a fresh link at once.

**The registry** is a TypeScript module, `functions/_shared/waitlist_terms.ts`. It holds five lists:
- **consent versions**: id, the exact words the form shows, and the privacy page version they link to;
- **confirm templates**: id and a hash of the confirm mail's text (§7.3);
- **offers**: id, the exact words and the cutoff. **Empty until the offer's amendment is signed**
  (§6.1);
- **move-up rules**: id and the sentence (§5.4);
- **form ids**.

It is **append-only**: a published entry's words never change, and a test pins a hash of each. The
function accepts the newest consent version and the one before it (the site and the functions deploy
separately), and refuses any other with `bad_request`. The live offer is the newest offer whose cutoff
is in the future; the server, not the form, records it.

### 3.6 Retention and deletion

| What | Kept for | Then |
|---|---|---|
| A `pending` entry never confirmed | until 30 days after `created_at`, and its latest link has expired (at most 37 days) | deleted by the sweep. No suppression record: nobody asked to stop |
| A `confirmed` entry | until the person leaves, asks for deletion, or the list closes | **the list closes 12 months after Knowlu opens to everyone**; a migration sets that date once it is known, and the founding facts billing needs move to billing first (§6.3) |
| An `unsubscribed` entry | 30 days after `left_at`, never mailed in between | deleted. Inside the 30 days the person can join again from the leave page and keep their place, founding flag and credits (§4.4) |
| *Delete my address now* on the leave page, a request to `support@knowlu.com`, or **Settings → Delete my data** on an account with the same address | — | the entry is deleted at once and a suppression record is written |
| `waitlist_suppressed` rows | **3 years after `suppressed_at`**: the default Quinn ruled (Q10, 2026-11-09), until the lawyer sets the final period (§3.8) | deleted by the sweep, unless the person lifts the record first |
| `waitlist_sends` rows | 90 days after `sent_at` | deleted, or with the entry |
| `waitlist_throttle` rows | 48 hours (`ip:`, `mail:*`); 31 days (`addr:`, `events:*`) | deleted |
| The hosts' own request logs and Resend's copy of each message | their own retention (**unverified**, U14) | named on the privacy page without a length (§9) |

- **The sweep** is the `pg_cron` job `knowlu-sweep-waitlist`, daily, at a minute no other fixed-minute
  job uses (`migrations/migrations_test.ts:2580-2640`).
- **A request by email** to see, delete or lift an entry is answered by the project owner in the SQL
  editor, through the `admin` functions (§3.4).
  - The editor's saved history keeps any address typed into it. The owner deletes the query from the
    history after answering; whether the platform keeps a copy beyond that is **unverified**.
- **Delete my data reaches the list.** The account handler already holds the address
  (`account/handler.ts:96`). W3 passes `lower(address)` to `waitlist_forget_key`, a definer call,
  because the table is closed even to `service_role`. It runs before the `accounts` row goes.
- **Children under 13.** If we learn that an entry is a child's, the owner deletes it with
  `admin.waitlist_forget(address, suppress => false)`. No suppression record is kept: nothing about the
  child stays.

### 3.7 IP addresses: not stored

- **Not on any row, and not in the consent record.** The subscription consent keeps an IP because
  California's subscription law wants proof of what a buyer agreed to (`20260910000100_accounts.sql:36-49`).
  Nothing requires one here; the double opt-in record is the proof.
- **Rate limiting uses a short-lived keyed hash.** The bucket is `ip:` followed by the first 32 hex
  digits of HMAC-SHA256(`WAITLIST_IP_KEY`, UTC date + `|` + the prefix).
  - **The prefix is the full IPv4 address, or the IPv6 /64**, because one IPv6 client holds a whole /64
    and could otherwise rotate past any limit.
  - The key never leaves the function secrets, the date stops one address being linked across days,
    and the sweep deletes the row within 48 hours.
- **Which header carries the client address on Supabase's edge is unverified** (U2).
  `billing-checkout` reads the first hop of `x-forwarded-for` (`handler.ts:83-88`), and if the gateway
  appends to a caller's own header, that hop is forgeable. The handler takes the reader as an injected
  function, and S5 decides which header to trust. The join budget holds whatever S5 finds.

### 3.8 Suppression, the per-address cap, and the consent proof

**Why.** Without a durable opt-out, a stranger can type someone's address again after that person
pressed "remove", or after they complained, and the confirm mail goes out again. By §7.3's own reading,
that confirm mail is commercial mail to an address that opted out, which CAN-SPAM forbids once 10
business days have passed. Mailing someone who marked us as spam also costs the sending domain its
reputation.

**What is kept:** `waitlist_suppressed` (§3.1). It is written in the same transaction as each of these:
- every leave, including leaving a pending entry from its confirm mail;
- every complaint and hard bounce;
- every "delete now", support deletion and Delete my data.

The pending sweep writes none, and nor does the under-13 deletion.

**Its effect.** A join for a suppressed `key_hash` answers `sent`, like every join, and writes and sends
nothing. The batch that picks update recipients skips suppressed keys too, as a second guard.

**Lifting it is the person's own act, and nothing else lifts it:**
- while the unsubscribed entry still exists (30 days), the leave page's "Join again" (§4.4), which only
  the mailbox's owner can reach;
- afterwards, a request to `support@knowlu.com` from that address. The owner runs
  `admin.waitlist_lift(address)`, and the person then joins as anyone does.

**Keyed hash, plain hash, or the address: the keyed HMAC** (`WAITLIST_ADDR_KEY`; Q10, Quinn,
2026-11-09).
- The address itself would let us do nothing the record needs. A fingerprint answers the one question
  we ask, "is this typed address suppressed?", and cannot be mailed or listed.
- A plain SHA-256, as the account tombstone uses (`account/index.ts:191`), can be checked against a
  dictionary of addresses by anyone who holds the table. A keyed one needs the key as well, and the key
  lives only in the function secrets.
- *The cost:* the records cannot be re-hashed under a new key, because there are no addresses to hash
  again. **A rotation, if one is ever needed** (a suspected leak of the key), works like this (r2 m5):
  - new records and new `key_hash` values use the new key;
  - the old key moves to `WAITLIST_ADDR_KEY_PREV` and is used **for lookups only**: while it is set,
    the join passes a second hash and `waitlist_join`'s step 1 checks suppression under both;
  - the old key is removed once every record made under it has expired, 3 years at most.

  Rotation is not routine.
- The privacy page names the record (§9), and the lawyer reads whether keeping it squares with a request
  to be erased (§8 item 9).

**The per-address cap.** The `addr:<key_hash>` bucket's window opens at the first join-path mail and
lasts 30 days, and it allows 3 mails. A stranger can therefore reach an address that never confirms at
most three times in each 30-day window, and never after its owner presses "remove this address now".
The bucket outlives the pending row (31 days), so the row's deletion does not reset it.

**The consent proof** (Q10 and Q11, merged; Quinn, 2026-11-09). A row's consent facts move into its
suppression record when the row goes, with no address, so that the sender can still show consent for
mail already sent: CASL and GDPR put that burden on the sender. **The record is kept 3 years by
default**, which is believed to be CASL's limitation period (**unverified**). The lawyer sets the final
period (§8 items 8 and 9).

## 4. The endpoints

### 4.1 Rules every waitlist function keeps

- **Five functions**: `waitlist-join`, `waitlist-confirm`, `waitlist-leave`, `waitlist-send` and
  `waitlist-events`.
  - Each is `verify_jwt = false` with its own `config.toml` entry, because no caller holds a session.
  - Each has a thin `index.ts` and a pure `handler.ts` (`cloud/supabase/README.md:151-152`).
- **JSON, a 303 redirect, or an empty body: never HTML.** The relay serves HTML as `text/plain`
  (`HANDOFF.md:46`), so every page a person sees lives on the site.
- **CORS, limited to the site's origins.**
  - `WAITLIST_ORIGINS` lists exact origins.
  - `OPTIONS` from a listed origin answers 204 with that origin allowed, `Vary: Origin`,
    `Access-Control-Allow-Methods: POST`, `Access-Control-Allow-Headers: content-type` and
    `Access-Control-Max-Age: 600`, and no credentials. Any other origin gets 403 and no allow header.
  - A `POST` to join, confirm or the leave page's buttons without a listed `Origin` answers 403. The
    RFC 8058 one-click is the exception (§4.4).
  - CORS stops another site's page from driving a visitor's browser. It does nothing against a script;
    §4.7's layers do that.
- **No address, token, IP address or hash ever reaches a log.** Three things hold this:
  - **The waitlist's own RPC wrapper**, `_shared/waitlist_db.ts`. It never goes through `db.ts`, whose
    `ok()` logs the upstream body verbatim (`db.ts:29-38`). A Postgres error body carries "Failing row
    contains (…)" or "Key (…)=(…)", which is the address. The wrapper cancels the body unread and logs
    `waitlist <rpc>: <status>`, as `countFrom` does (`entitlement.ts:111-126`). It then throws the house
    502.
  - **Each waitlist `index.ts` has its own catch**, which logs a fixed label and the error's class name.
    It does not use `asResponse`, which logs `e.message` (`http.ts:65-69`), and a resolver's or a
    parser's message can carry the domain or the body.
  - **The mail sender logs only Resend's status** (`canary_send.ts:8-10`). Resolver errors are
    swallowed with no message.

  §13.2 proves all three with stubs that echo the address.
- **No answer depends on whether an address is listed.** Every join path makes the same one database
  call. Mail is sent after the answer, through `EdgeRuntime.waitUntil` (`mail-inbound/index.ts:36-42`),
  so the answer's timing does not depend on the send. §13.2 proves the answer returns while the mailer
  never resolves.
- **The list's mail never touches sign-in's.** The waitlist modules read `WAITLIST_EMAIL_API_URL`,
  `WAITLIST_EMAIL_API_KEY` and `WAITLIST_EMAIL_FROM`, never `EMAIL_*` (§7.6), and a test pins that.
- **The configuration gate.** Join answers `error` and sends nothing unless all of these are set:
  - `WAITLIST_POSTAL_ADDRESS` (W-D4);
  - `WAITLIST_WEBHOOK_SECRET`, so complaints are recorded before anyone can be mailed (§4.6);
  - the list's own mail key and sender;
  - `WAITLIST_ADDR_KEY`, `WAITLIST_IP_KEY` and `WAITLIST_UNSUB_KEY`.
- **Bodies are capped at 4,096 characters** (`readJson`'s `limit`, `http.ts:48-58`).

### 4.2 `POST /waitlist-join`

**Request:** JSON `{email, consent_version, form_id, invite?, website?}`. `website` is the honeypot, a
field hidden from people and from assistive technology. The form no longer sends `offer_version`; the
server sets it (§6.2).

**Checks, in this order:**
1. method, `Origin`, the body, then the configuration gate (§4.1);
2. **honeypot**: a non-empty `website` answers `sent` and does nothing else;
3. **the IP burst limit**: 30 requests in 10 minutes per `ip:` bucket (§3.7), and no daily cap. A
   campus or carrier NAT puts hundreds of students behind one address, and a group chat passing an
   invite link is exactly the traffic §5 wants. This limit only slows a script; the join budget is the
   backstop;
4. **the join budget is read, not charged**: `waitlist_budget_left('mail:join:day', WAITLIST_JOIN_PER_DAY)`.
   With none left, every join answers `busy` **before the address is looked at**, so the answer cannot
   depend on the address;
5. **the address**:
   - ASCII only; at most 254 characters; exactly one `@`;
   - a local part of 1 to 64 characters from letters, digits and ``.!#$%&'*+/=?^_`{|}~-``, with no
     leading, trailing or doubled dot;
   - a domain of at least two labels, each 1 to 63 letters, digits or hyphens with no hyphen at either
     edge, ending in a label of at least two letters (punycode `xn--` labels allowed);
   - then an MX-or-A lookup through an injected resolver, with a 1.5 s timeout. No record means
     `invalid`. A resolver error or a timeout counts as valid, so our failure never refuses a real
     address;
6. `consent_version` and `form_id` against the registry; the server picks the confirm template and the
   live offer (none until the offer's amendment is signed, §6.1);
7. `waitlist_join`, then the answer; after the answer, the mail and `waitlist_mail_result`.

**Only a well-formed request that is not suppressed, not already mailed in the last 24 hours, and under
its address's cap spends a unit of the budget**, because the unit is charged inside `waitlist_join`'s
claim (§3.5). Invalid addresses, honeypot hits, in-window repeats and suppressed addresses spend none.

**The residual oracle, stated.** Because a mail spends a unit and a repeat does not, someone who drives
the budget to exactly its last unit could probe one address and then send a fresh one: `busy` would mean
the probe was mailed. To get there they must know the exact remaining count, which nothing exposes. They
would have to spend nearly the whole day's budget on real confirm mails to addresses that are not
suppressed, which closes the form for everyone and shows in `admin.waitlist_counts`. That is accepted.

**Responses:**

| Outcome | Status | Body | When |
|---|---|---|---|
| `sent` | 202 | exactly `{"outcome":"sent"}` | **every valid request, whatever the address's state**, and every honeypot hit |
| `invalid` | 400 | `{"outcome":"invalid","error":"<one sentence>"}` | the address fails the syntax or DNS check |
| `bad_request` | 400 | same shape | a missing field, not JSON, or an unknown consent or form id (a site bug, named) |
| `forbidden` | 403 | same shape | `Origin` not listed |
| `too_large` | 413 | `http.ts`'s answer | over 4,096 characters |
| `busy` | 429, with `Retry-After` in seconds | same shape | step 3 or step 4, decided before the address is read |
| `error` | 5xx | `{"error": …}` | the configuration gate, the database, or anything unexpected |

**The mail after the answer.** `confirm_mail` sends the confirm email and `listed_mail` the "already on
the list" email (§7.3); `none` sends nothing. If Resend refuses or times out, `waitlist_mail_result`
releases the claim, the `addr:` hit and the budget unit. The cost of answering first is that a Resend
outage still shows `sent`. The alternative makes the answer and its timing depend on the address's
state, which is the leak W-D5 rules out.

### 4.3 `POST /waitlist-confirm`

**Request:** JSON `{token}`, 43 characters of base64url (32 random bytes). The function hashes it and
calls `waitlist_confirm`.

| Outcome | Status | Body | When |
|---|---|---|---|
| `confirmed` | 200 | `{"outcome":"confirmed","invite_url":"<site>/r/<code>","founding":false}` (`true` only under a signed offer, §6.2) | the entry's current token, `pending`, unexpired |
| `already_confirmed` | 200 | the same shape | the current token of a `confirmed` entry |
| `expired` | 410 | `{"outcome":"expired","error":…}` | the current token of a `pending` entry, past `confirm_expires_at` |
| `invalid` | 404 | same shape | unknown; superseded by a newer token; or the entry left or was deleted. An old link never re-subscribes anyone |
| `bad_request`, `forbidden`, `busy`, `error` | 400, 403, 429, 5xx | as §4.2 | the `ip:` limit here is 30 in 10 minutes |

- **No `GET` confirms anything.** Link scanners open links before people do, and a confirm on `GET`
  would confirm addresses nobody confirmed. The mail links to `https://knowlu.com/w/confirm#<token>`,
  and that page's button `POST`s the token.
- **The token travels in the fragment**, so it is never in a request line, a host's log or a `Referer`.
- **`already_confirmed` leaks nothing**: only the mailbox the link was sent to holds the token.
- **One transaction confirms:**
  - status `confirmed` and `confirmed_at`, with `first_confirmed_at` set if null;
  - `founding` frozen (§6.2);
  - the invite code issued if there is none, redrawn on a collision up to three times, then `error`
    (§5.1);
  - the referrer credited (§5.3).

### 4.4 `waitlist-leave`: unsubscribe, with RFC 8058 one-click

**The token** is base64url of the entry's 16 id bytes followed by the first 16 bytes of
HMAC-SHA256(`WAITLIST_UNSUB_KEY`, `leave|` + id).
- It is derived, never stored, and valid as long as the key is: well past CAN-SPAM's 30 days and CASL's
  60.
- After a key rotation the old key (`WAITLIST_UNSUB_KEY_PREV`) is still accepted for 60 days.
- It can act on one entry only.

**Routes:**
- **`POST /waitlist-leave?t=<token>` with the form body `List-Unsubscribe=One-Click`** is RFC 8058's
  one-click. It needs no `Origin`, cookie or session, answers 200 with an empty body, and is not
  IP-limited, because a mail provider sends these from a few addresses of its own. Its reason is
  `one_click`.
- **`POST /waitlist-leave` with JSON `{token, action}`**, from the site's leave page (listed `Origin`;
  `ip:` limit 30 in 10 minutes):
  - `action: "leave"` answers `{"outcome":"left"}`;
  - `"delete"` answers `{"outcome":"deleted"}`;
  - `"rejoin"` answers `{"outcome":"sent"}` when a confirm mail was claimed, and `{"outcome":"later"}`
    when none could be (below; r2 C1).
  A token whose HMAC checks out but whose entry is gone answers `left`, because it is. A token that
  fails the HMAC answers 404 `invalid`.
- **`GET /waitlist-leave?t=<token>`** answers 303 to `<site>/w/leave#<token>` and changes nothing.

**Effects, each in one transaction with its suppression record (§3.8):**
- **leave** a `pending` entry: the entry is deleted and suppressed (the "remove this address now" path);
- **leave** a `confirmed` entry: it becomes `unsubscribed`, with `left_at` and `left_reason`, and is
  suppressed;
- **delete**: the entry is deleted and suppressed (`deleted`);
- **rejoin**, only from an `unsubscribed` entry that still exists. It is the person's own act, from a
  link only their mailbox holds, so **it skips the 24-hour window but keeps the rest of §3.5's claim**:
  the `addr:` cap and a `mail:join:day` unit (r2 C1).
  - **When a mail is claimed**, in that one statement: the suppression record is removed, and the entry
    goes back to `pending` with a new token and `mail_claimed_at` stamped, keeping `first_confirmed_at`,
    `founding` and `credits`. The confirm mail is sent after the answer, and `waitlist_mail_result`
    releases the claim on a failed send, as for a join.
  - **When none can be claimed** (the cap is spent or the day's budget is gone): **nothing changes**. The
    entry stays `unsubscribed` and suppressed, so no unmailed token is left behind, and the answer is
    `later`. The page says to try again tomorrow (§10.5). The person is never left `pending` with a link
    they were not sent.

Every leave takes hold before the next page of any send, because `waitlist_batch` re-reads status and
suppression in the statement that picks the page (§13.2 tests a leave between two pages).

### 4.5 `POST /waitlist-send`: update emails, and no export

- **Who calls it.** The controller, from a shell, with `x-knowlu-job-token` equal to
  `WAITLIST_JOBS_TOKEN`, the house pattern (`cloud/supabase/README.md:92`, `:128`). Never cron, never the site, never a subagent.
- **Request:** `{broadcast, dry_run?, limit?}`. `broadcast` names a file compiled into the function,
  `functions/waitlist-send/broadcasts/<id>.ts`, holding the subject, the text and the HTML. **No route
  sends words that were not reviewed in a pull request.**
- **It refuses with 409 and a named sentence, sending nothing**, when:
  - `WAITLIST_POSTAL_ADDRESS` is unset (W-D4);
  - `WAITLIST_UPDATES` is not exactly `on`;
  - `WAITLIST_WEBHOOK_SECRET` is unset;
  - the broadcast lacks a footer slot (§7.4);
  - **this broadcast would be the third to start in 30 days**: `waitlist_broadcasts_started(30)` counts
    the distinct broadcasts whose first send row is newer than 30 days, and a new broadcast is refused
    at 2. §13.2 tests it.
- **The per-person limit is the batch's, not the gate's** (r2 N4). A broadcast can take days to send
  under the daily budget, so counting broadcasts by their first send row lets one person receive three
  in 30 days when sends are staggered. `waitlist_batch` therefore skips any entry that already has two
  send rows of any broadcast in the last 30 days, on a rolling window, re-read in the statement that
  picks the page. A skipped entry is not lost: it has no send row for this broadcast, so a later call
  picks it up once its window allows. **This is what keeps §9's "never more than two in any 30 days"**
  true for each person. The 409 gate stays as the cheap first check. §13.2 and §13.4 test a staggered
  send.
- **It sends** to `confirmed`, unsuppressed entries with no send row for this broadcast that are under
  the per-person limit, in pages from `waitlist_batch`:
  - each recipient is one Resend `POST /emails` with `Idempotency-Key: <broadcast>:<entry>`, followed by
    its send row;
  - sends are one at a time, at least 600 ms apart, because Resend's API rate limit is believed to be 2
    requests a second (**unverified**, U5);
  - **each call sends at most `limit`** (default 50, at most 100), so it ends well inside the function's
    wall-clock limit (**unverified**, U20). The controller calls again.
- **It stops** when `mail:send:day` (`WAITLIST_SEND_PER_DAY`) is spent, and answers
  `{"sent":n,"skipped":n,"held":n,"remaining":n}`, where `held` counts entries the per-person limit
  kept back. **Sends never draw on the join budget**, so a broadcast never closes the form.
- **`dry_run`** answers the counts and the rendered subject and text, with placeholders for the address
  and the token.
- **No export.** No endpoint returns or transfers the list (CAN-SPAM forbids transferring opted-out
  addresses). The owner reads counts with `admin.waitlist_counts` and the invitation order with
  `admin.waitlist_order(n)` (§3.4).

### 4.6 `POST /waitlist-events`: bounces and complaints

- **Authenticated by Resend's webhook signature** (`WAITLIST_WEBHOOK_SECRET`). The scheme is
  **unverified** (U5).
- **It exists before any join can mail**, because the join refuses without its secret (§4.1). A
  complaint about the very first confirm mail is recorded.
- **It belongs to the list's own Resend account** (§7.6), so it sees only list mail. Staging and
  production share that account, so **each event is attributed to a project before it is read further**,
  and a staging complaint never suppresses an address on production (r2 N5):
  1. **by tag**: every list send carries a tag naming its project (`list-staging` or
     `list-production`). Whether webhook payloads return tags is **unverified** (U5);
  2. **else by sender**: each project sends from its own addresses (§4.8, §7.3), so the payload's `from`
     is matched against this project's `WAITLIST_EMAIL_FROM` and `WAITLIST_UPDATES_FROM`;
  3. **neither matches**: the event is dropped with the fixed log label `waitlist-events: unattributed`
     and counted in `waitlist_throttle` under `events:unattributed:day`, which `admin.waitlist_counts`
     shows. A rising count means attribution is broken, and it is visible without reading any address.

  The fallback is not optional: S14 (§14.1) proves attribution live on staging, by whichever route the
  payload supports, before any list mail goes out on production.
- `email.complained` and hard `email.bounced`:
  - **the address is lower-cased before it is hashed** (r2 m6), because `key_hash` is the HMAC of
    `email_key`, never of the address as typed;
  - for an address with an entry, `waitlist_leave` with reason `complaint` or `bounce`, which writes the
    suppression record;
  - for an address with none, `waitlist_suppress_key(HMAC(lower(address)))`.

  Either way it answers 200 and logs one fixed label, `waitlist-events: tag` or `waitlist-events: from`,
  which names only how the event was attributed (S14 reads it), and nothing that names anyone.

### 4.7 Rate limits and bot defence, without a tracker

**The layers, cheapest first:**
1. **Double opt-in.** Nobody gets a second kind of mail they did not confirm.
2. **Suppression.** An address that left, complained or bounced is never mailed again on anyone's join
   (§3.8).
3. **The per-address limits.** One join-path mail a day, and three in a 30-day window.
4. **The join budget**, charged only by mails actually claimed. It is the real backstop for the sending
   reputation (§7.6).
5. **The IP burst limit**, IPv6 grouped by /64. It slows one script and cannot shut out a campus.
6. **The honeypot, the `Origin` check and the DNS check.**

**Turnstile is not recommended now.**
- *What it buys:* it stops a scripted flood that rotates addresses and networks.
- *What it costs:* a Cloudflare script that reads browser signals, which ends the page's "no third-party
  script" claim (`privacy.html:163`), plus a verify call with its own secret.
- *The trigger to add it:* the join budget runs out on two days in one week, or confirm-mail bounces
  pass 5% of a day's sends. It would then be its own small change, carrying its own page sentence.

### 4.8 Secrets, by name

Set by Quinn, or generated blind by the controller, on each project. None ever enters the repo.

| Name | Holds |
|---|---|
| `WAITLIST_ORIGINS`, `WAITLIST_SITE_ORIGIN` | the exact origins CORS allows; the origin the emails link to |
| `WAITLIST_IP_KEY`, `WAITLIST_ADDR_KEY` | the HMAC keys for `ip:` buckets and for `key_hash` (generated blind). `WAITLIST_ADDR_KEY` is not rotated as routine; a rotation follows §3.8 (r2 m5) |
| `WAITLIST_ADDR_KEY_PREV` | unset, except after a rotation: the previous address key, **used for suppression lookups only**, and removed once every record made under it has expired (3 years at most, §3.8) |
| `WAITLIST_UNSUB_KEY`, `WAITLIST_UNSUB_KEY_PREV` | the leave-token key, and the previous one during a rotation (generated blind) |
| `WAITLIST_JOBS_TOKEN` | the send route's token (generated blind) |
| `WAITLIST_WEBHOOK_SECRET` | the list account's webhook signing secret |
| `WAITLIST_EMAIL_API_URL`, `WAITLIST_EMAIL_API_KEY` | the list's Resend endpoint and key: **not** sign-in's `EMAIL_API_KEY` (§7.6) |
| `WAITLIST_EMAIL_FROM`, `WAITLIST_UPDATES_FROM` | the confirm sender and the update sender, both on the list's subdomain (Q6). **Each project's pair is distinct from the other project's** (for example a `staging-` local part on staging), because §4.6 attributes an untagged event by its `from` (r2 N5) |
| `WAITLIST_POSTAL_ADDRESS` | the PO box, one line, exactly as mailed (W-D4) |
| `WAITLIST_UPDATES` | on production, `on` only after the lawyer's read, on Quinn's word; on staging, only for the length of S13 |
| `WAITLIST_PREVIEW` | `on` on staging only: every mail carries the preview line (§7.3). It never excuses the postal address |
| `WAITLIST_JOIN_PER_DAY`, `WAITLIST_SEND_PER_DAY` | the daily budgets: staging 5 and 0; production 40 and 100, unless U5 shows the quota needs smaller numbers. **Their sum across both projects stays under the list account's daily quota**, which S10 checks |

## 5. Invites: `knowlu.com/r/<code>` and "move up"

### 5.1 The code

- **Eight characters of Crockford base32** (digits and capitals without `I`, `L`, `O` or `U`): 40
  random bits from `extensions.gen_random_bytes(5)`. pgcrypto is installed in `extensions`
  (`20260910000100_accounts.sql:9`).
- **Issued inside `waitlist_confirm` at the first confirmation**, and kept for the entry's life,
  including a rejoin from the leave page.
  - On a unique collision the function draws again, up to three times, then fails the confirmation as
    `error`. §13.4 forces a collision to prove it.
- **Why at confirmation.** A pending entry may be a stranger's typing. A code is shown only on the
  confirmed page and in the "already on the list" mail, which only the mailbox's owner reaches.
- **Why 40 bits.** A guessed code buys an attribution and nothing else: no read of an entry, and no
  credit without a distinct confirmed mailbox. Forty bits makes guessing pointless and keeps the link
  short.
- **Input is forgiving.** The join upper-cases the code and maps `O` to `0` and `I` or `L` to `1`. A
  code that matches nothing is ignored, and the join answers `sent` as always.

### 5.2 The route

- **One line in `site/_redirects`**: `/r/:code /?invite=:code 302`.
  - Whether Pages honours `_redirects` placeholders is **unverified** (U9). The fallback is
    `/r/* /?invite=:splat 302`, and S7 checks it on the preview.
- **Why not a Pages Function.** It would put code that runs on `knowlu.com` into a static site, and the
  check it could make is one the join already makes. A redirect cannot fail open.
- **On the page**, the script (§10):
  - reads `invite` from the query;
  - removes it from the address bar with `history.replaceState`;
  - keeps it in memory only, with no cookie and no storage;
  - sends it with the join.

  The page may say "A friend sent you this link." It never names the friend.
- `Referrer-Policy: no-referrer` (§10.4) keeps the code out of any `Referer`.

### 5.3 Credit: given at confirmation, once, and never taken back

**At join.** `referred_by` is set by the **first** join that carries a code which:
- matches a `confirmed` entry's code;
- belongs to an entry whose `fold_key` differs from this address's.

**After that it never changes.** The draft let a later join replace it, which would let a third party
re-join someone else's pending address with their own code and take the credit (r1 M7).

**At the first confirmation**, in the same transaction as §4.3's, the referrer gains one credit when all
of these hold:
- this entry's `first_confirmed_at` and `credited_at` were null until now;
- `referred_by` is set;
- the referrer is `confirmed` and not suppressed;
- the two `fold_key`s still differ.

It sets this entry's `credited_at` and adds one to the referrer's `credits`. A rejoin keeps
`credited_at`, so it never credits twice.

**Never revoked.** When the friend later leaves or is deleted, the referrer keeps the credit.
- Revoking would let a referrer watch their own place move and learn that a friend had left: a
  membership leak in reverse.
- *The cost:* someone can confirm through a friend's link, delete their entry and join again through the
  same link. Their address is then suppressed (§3.8), so they cannot rejoin without writing to support.
  The loop costs a support request each time, and the cap bounds it.

**Self-referral and farming.**
- The address's own code is ignored at join, and the confirmation repeats the `fold_key` test. That
  catches Gmail dots and plus tags anywhere.
- One person with several real mailboxes can credit themselves from each. Each needs a real confirm
  click, and **the cap is five credits' worth**.
- Disposable-address domains are not blocked: a blocklist is a third-party dependency that goes stale.

### 5.4 The order "move up" changes

- **The order key** is `first_confirmed_at − 7 days × least(credits, 5)`, ties broken by
  `first_confirmed_at`, then by `id`, over `confirmed` entries only.
- **It is computed on read**, by `admin.waitlist_order(n)`, and never stored.
- **What it orders: invitations to the spring 2027 release**, whose recommended cap of about 50 outside
  sign-ups (Amendment 2026-10-16b, still open there) makes a place matter (Q5, decided as specified:
  Quinn, 2026-11-09).
- **What it does not touch: the price.** Credits never buy an offer (§6).
- **Why a time shift.** It fits in one honest sentence, "each friend who joins with your link and
  confirms moves you a week earlier, up to five weeks", and it keeps the list mostly first come, first
  served. The sentence is a claim to consumers, so it lives in the registry, and W7's test pins the
  site's text to it.

### 5.5 A position number: not in v1 (Q5, Quinn, 2026-11-09)

- A number needs a per-person read surface: a token that reads, not only one that leaves or confirms.
- It reveals the size of the list.
- It invites gaming, and in a small list it moves too little to mean anything.
- Instead, the confirmed page and the "already on the list" mail carry the invite link and the move-up
  sentence.
- If Quinn wants a number later, it is its own change: a separate read token, a status page, and a
  privacy sentence.

## 6. The founding price: the gate, what the row records, and the money work it names

### 6.1 The gate: no offer anywhere until Quinn's amendment signs it

The prototype carries "$6.99 a month for the first year, then $9.99, for people who join before launch".
**No signed amendment holds that offer.** Amendment 2026-10-16b leaves the quiet release's "discounted
price" open (cloud design `:829-831`), and nothing in billing would honour it. **Quinn ruled (Q4, Quinn,
2026-11-09): no offer anywhere until an amendment signs it.** So, until that amendment is signed:
- **the registry's offers list is empty**, so `offer_version` is null on every row and `founding` is
  false;
- **no page on any deploy shows a price other than the signed ones, or the word "founding"**. W7's test
  pins this: while the registry holds no live offer, no file under `site/` contains "6.99" or "founding"
  (§13.6);
- **the consent line `c1` does not name the founding price** (§10.2), so the append-only words outlive
  whatever the amendment sets;
- **§14.2 has no step that publishes an offer.** Publishing one is its own change after the amendment:
  a registry entry, the site's words, the billing spec and `terms.html`.

### 6.2 What the row records once an offer is signed

| Field | Meaning |
|---|---|
| `offer_version`, `offer_cutoff` | **set by the server** from the live offer at the request, whatever form was used. The site shows the offer on every form while it runs, and W7's test pins that. The draft let the form assert the offer, so two people joining the same day through different forms could differ (r1 M7) |
| `requested_at` | the request that was confirmed, frozen at confirmation |
| `first_confirmed_at` | the first confirmation |
| `founding` | **frozen at the first confirmation**: true when `offer_version` is set and `requested_at < offer_cutoff` |
| `email` | the address an account will later be matched to (§6.3) |

- **A request before the cutoff and confirmed after it still counts**, within the 7-day link.
- **A rejoin from the leave page keeps `founding`.** A deleted entry keeps nothing.
- **People who joined before the offer existed qualify** (Q4, Quinn, 2026-11-09). When the offer goes
  live, a one-time migration sets `founding` for every confirmed entry requested before the cutoff, so
  joining early never costs anyone the price.
- **The claim expires when the list closes** (Q4, Quinn, 2026-11-09). Before then, billing takes the
  founding facts it needs (§3.6, §6.3 item 2).
- **The confirm answer carries `founding`** (§4.3), so the confirmed page states the price only to the
  people who hold it.

### 6.3 The money work this names, and does not design

Each item is a gate on charging anyone the founding price.
1. **Quinn's amendment to the cloud design (Q4).** It answers Amendment 2026-10-16b's open discounted
   price. **Already ruled (Quinn, 2026-11-09)**, so the amendment carries these as given:
   - **one discount, shared with quiet-release buyers**: a buyer during the quiet release who never
     joined the list pays it too, so no page may imply that joining is the only way to get it;
   - **a Pell student pays the lower of the two prices, never both discounts**;
   - earlier joiners qualify, and the claim expires when the list closes (§6.2).

   **Still open, and the amendment's alone:** the amount, the plan it applies to, the term (how long it
   lasts and how that is counted), and the cutoff. A pricing analysis is running; this spec does not
   anticipate its answer.

   **The arithmetic it should weigh**, using the prototype's $6.99 only as an example, not a proposal
   (cloud design `:772-780`; R3's pause on monthly plans only):

   | Plan, over one calendar year | Cost |
   |---|---|
   | Founding monthly, pausing June to August: 9 paid months × $6.99 | **$62.91** |
   | Founding monthly, never pausing: 12 × $6.99 | $83.88 |
   | Two 6-month terms at $39.99 | $79.98 |
   | Standard monthly, pausing: 9 × $9.99 | $89.91 |

   So the founding monthly price beats the term for anyone who pauses, and the term is cheaper only for
   someone who pays through the summer. "The first 12 paid months" spans about 15 calendar months for
   someone who pauses.
2. **The billing spec**:
   - how an account claims the price (the sign-in address equals a founding entry's `email_key`, or a
     claim link mailed to the list address);
   - the Stripe object;
   - `billing-checkout` and `billing-jobs`' `priceOf` and reminders;
   - the founding facts billing keeps when the list closes (§3.6).
3. **The terms.** `site/terms.html` states the price. That is a terms change, so it decides whether
   `TOS_VERSION` (`app/src/account.rs:24`, on the contract list) moves.
4. **Amendment 2026-10-16b's gate** before the first outside charge, among others the lawyer's reading
   of the terms and the privacy page.

## 7. Email

### 7.1 Resend's HTTP API, not SMTP

- **The house already sends this way** (`billing-jobs/index.ts:26-36`; `canary_send.ts:57-79`).
- **The API takes what the list needs per message**: custom headers (`List-Unsubscribe`,
  `List-Unsubscribe-Post`), a text part and an HTML part, a tag, and an idempotency key.
- **SMTP from an edge function** would need a raw TCP client and an outbound mail port that the edge
  may block (**unverified**). The SMTP block in `config.toml:85-92` is Supabase Auth's own relay.
- **It tests offline.** The endpoint is injected, and a `127.0.0.1:0` listener stands in for Resend
  (§13.3).

### 7.2 Our own send loop, not Resend's audiences and broadcasts

- **One list.** Resend's audiences would be a second copy of every address at a processor, kept in step
  by code that can drift. Our loop shows Resend one address at a time, as it sends.
- **One way to leave, and one suppression list that counts.** Resend's broadcasts bring their own
  unsubscribe page and list, which our table would have to mirror. Ours is the record (§3.8), and
  Resend's own suppression only ever withholds.
- **Reviewed words.** A broadcast is a file in a pull request (§7.5).
- **The cost.** We own pacing, idempotency, suppression and bounce handling (§3.8, §4.5, §4.6).
- **How unsubscribes stay in sync: there is nothing to sync.**
  - Every leave path writes our table and its suppression record: the page, one-click, a complaint, a
    bounce, Delete my data, and the owner's `admin` functions.
  - Every send page re-reads both.
- **Revisit** past about 10,000 addresses, or when a send must be scheduled across days.

### 7.3 The confirm email, and "already on the list"

Under CASL, a message that asks for consent is itself believed to be a commercial message (lawyer item
2). So the confirm email carries the same identity, postal address and way out as an update, and
**every list mail carries the postal address. The PO box comes before the preview** (§14.1), so no
friend is mailed without it (r1 M5).

| Part | The confirm email |
|---|---|
| From | `WAITLIST_EMAIL_FROM`: display name `Knowlu`, an address on the list's own subdomain (Q6), **not** sign-in's `Knowlu <hello@knowlu.com>` (`HANDOFF.md:52`). Staging and production use different addresses, so an untagged webhook event can be attributed by its `from` (§4.6, r2 N5). Reply-To `support@knowlu.com` |
| Subject | **"Confirm your address for Knowlu's early-access list"**. The draft's "Confirm your early access to Knowlu" read as access already granted (r1 M11) |
| Why | "Someone asked to add this address to Knowlu's early-access list. If it was you, confirm below." |
| **What you are agreeing to** | Restated from the server, not taken from the form: "Once you confirm, we'll send you updates about Knowlu's opening and early access, at most two in any 30 days. Every email has a link to leave." This is the text the confirm click answers, so the consent proof rests on words we sent and recorded (`confirm_template`), not only on what the form asserted (r1 M6) |
| The link | `https://knowlu.com/w/confirm#<token>`, as a button and a plain URL. "This link works for 7 days." |
| The way out | "If this wasn't you, ignore this email. Or press here to remove this address and stop anyone adding it again: <leave link>." |
| Footer | the postal address; "Knowlu, knowlu.com"; on staging only, the preview line |
| Headers | `List-Unsubscribe: <https://<functions>/waitlist-leave?t=<token>>`, `List-Unsubscribe-Post: List-Unsubscribe=One-Click`, and the list tag |

- **Ignoring it is enough.** The pending entry is deleted in at most 37 days (§3.6), and a stranger's
  repeated joins reach it at most three times in a 30-day window (§3.8).
- **The "already on the list" email** answers an out-of-window repeat join from a confirmed address. Its
  subject is "You're already on Knowlu's early-access list". It carries the invite link, the move-up
  sentence, and the same footer and headers.
- **The preview line** (staging only, `WAITLIST_PREVIEW=on`): "This is a preview list for testing. It
  will be deleted before Knowlu opens, and you'll be asked to join again."
- **The confirm template's words are a registry entry** (id and hash, §3.5). Changing them means a new
  id, recorded on each row it is sent with.

### 7.4 Update emails

Each rule is tested where a test can see it (§13):
- **Sender identity.** The display name is `Knowlu`; the address is `WAITLIST_UPDATES_FROM` on the
  list's subdomain (Q6); Reply-To is `support@knowlu.com`, which a person reads. The sender is
  described as Knowlu.
- **Accurate headers.** From, Reply-To and the return path are ours and aligned for DMARC.
- **A subject that says what is inside**: no `Re:` or `Fwd:`, and no false urgency. A test refuses the
  prefixes.
- **The footer, in both parts:**
  - "You're getting this because you asked for early access at knowlu.com.";
  - an unsubscribe link to `https://knowlu.com/w/leave#<token>`: one page, one button, no sign-in, no
    fee;
  - the postal address.
- **One-click headers** as on the confirm email. The DKIM signature must cover both
  `List-Unsubscribe` headers (RFC 8058). Whether Resend signs custom headers is **unverified**; S11
  checks the `h=` tag.
- **Honoured at once.** A leave takes effect before the next send page (§4.4).
- **At most two update emails to any one person in any 30 days, enforced** by `waitlist_batch`'s
  per-person rolling check, with `waitlist-send`'s 409 gate in front of it (§4.5, r2 N4).
- **No tracking.** Open and click tracking are off for the list's sending domain (**unverified** that
  this is a per-domain setting). There are no images and no remote content.

### 7.5 Where the words live

- **`functions/_shared/waitlist_mail.ts`** holds the confirm and "already on the list" mails, the
  footer and the header builder. Each is a pure function from the entry's facts and the configuration to
  `{subject, text, html, headers, tags}`. The text part is the source; the HTML carries the same words.
- **`functions/waitlist-send/broadcasts/<id>.ts`** holds one update each. A test renders every
  broadcast and checks:
  - both parts are present and the footer slots are filled;
  - the subject has no `Re:` or `Fwd:`;
  - there is no `<img`;
  - links go only to `knowlu.com` or the functions origin.
- **Not Resend's template editor.** Words in code are reviewed, versioned and testable offline.

### 7.6 The list's mail is kept apart from sign-in's

**Why.** Today one Resend account and key send every project's sign-in codes (`HANDOFF.md:52`, `:566`).
Mixing list mail into it couples the list to sign-in in four ways (r1 I6):
- **Suppression.** If a person marks a list mail as spam and Resend's suppression list is account-wide
  (**unverified**, U5), their sign-in codes are withheld too, and they cannot sign in to a product they
  pay for.
- **Reputation.** Confirm mails go to addresses strangers type, so they draw the most complaints. Sent
  from `hello@knowlu.com`, they would spend sign-in's reputation.
- **Quota.** The draft's budgets were 40 on each project against a believed 100 a day for the whole
  account. That left 20 for both projects' sign-in and billing mail.
- **The key.** The current key is in the open roll (`HANDOFF.md:566`).

**The rule:**
- **List mail never uses sign-in's key or sender**, and the waitlist modules cannot read `EMAIL_*`
  (§4.1).
- **Decided: a separate Resend account for the list, on a paid plan** (Q6, Quinn, 2026-11-09), with
  its own key, quota, suppression list and webhook. List mail is sent from its own subdomain of
  `knowlu.com` (for example `list.knowlu.com`; the name is Quinn's). The subdomain's new DNS records
  leave the existing `send.` and `resend._domainkey.` records alone.
  - This holds whatever U5 finds about the scope of Resend's suppression, because only a separate
    account also separates the quota and the key.
  - The one account serves both projects' list mail, so the two projects are told apart by tag and by
    sender (§4.6, §4.8).
- **When:** before any list mail, staging's included (Q6, Quinn, 2026-11-09), because friends who test
  the preview also hold accounts.
- **The budgets fit one quota.** Staging joins 5 a day; production joins 40 and sends 100. That totals
  145 a day, so the list account's plan must allow it. If the plan's daily limit is lower, the
  production numbers come down (S10 checks the sum).

## 8. The law it has to keep

**This is not legal advice.** It is an engineering reading of each rule, written so a lawyer can check
it quickly. **A lawyer reviews the consent wording (the form's line and the confirm mail's restatement),
the privacy section (§9) and the first update email before the first update email is sent** (§14.2, P6).
Every figure and rule not quoted from the dispatch is **unverified** until that read (U15).

| Rule | What it asks, as read here | How this design meets it |
|---|---|---|
| **CAN-SPAM** (US), per the FTC's compliance guide for business | No false or misleading header information. No deceptive subject. The message identified as an advertisement (lawyer item 1). A valid physical postal address: a street address, a USPS PO box, or a private mailbox registered with a commercial mail receiving agency. A clear opt-out asking for nothing but the address and a reply or a visit to one web page, with no fee. Opt-outs honoured within 10 business days; the mechanism working at least 30 days after each message; opted-out addresses never sold, transferred or mailed again. The sender stays responsible for whoever sends on its behalf. Penalties up to **$53,088 per email** | §7.3-§7.4's headers, subjects and footers. The PO box on every list mail, and no list mail without it (§4.1). Leaving is one page and one button, or one-click (§4.4), in force before the next send page. **An opted-out address is never mailed again, not even a confirm mail on a stranger's join** (§3.8). Tokens live as long as their key. No export or transfer (§4.5). Resend sends on our instruction, and our record decides |
| **CASL** (Canada), if a Canadian joins | Consent before a commercial electronic message, express consent recorded with what was asked. Identification of the sender, a mailing address, and a phone number, email or web address. An unsubscribe in force within 10 business days and working for 60 days after the message. A request for consent is itself a commercial message | Double opt-in is express consent. The row keeps the form's words, the confirm mail's words (`confirm_template`), and the request and confirmation times, and the suppression record keeps them after a leave (§3.8). Every list mail's footer names Knowlu, the postal address, `support@knowlu.com` and `knowlu.com`. Leave tokens outlive 60 days |
| **GDPR and UK PECR**, if someone in Europe or the UK joins | Consent that is freely given, specific, informed, unambiguous and demonstrable (Art. 7(1)), and as easy to withdraw as to give (Art. 7(3)). The information duties of Art. 13. Access and erasure. Processors bound by contract | Knowlu is sold to US undergraduates (cloud design `:810`), but the form cannot stop anyone else, so the list meets the bar anyway: double opt-in with the record; the consent line linking §9's section; one-press leaving; and the owner's `admin` functions for access, erasure and lifting (§3.4). Keeping a keyed suppression record after an erasure request is lawyer item 9 |
| **CalOPPA** (California) | A conspicuous privacy policy naming the categories collected, who they are shared with, how to review or change them, how changes are announced, the effective date, how Do Not Track is handled, and whether third parties collect information about online activity | §9 names every stored item, the processors, the retention and how to leave or be deleted, and keeps the Do Not Track paragraph true |
| **COPPA** (US) | Rules for sites directed to children under 13, or that knowingly collect from them | The site is written for college students, and an account needs 18 or older (`privacy.html:155`). A child's entry is deleted with no suppression record (§3.6) |
| **The FTC Act** on deceptive claims | An offer to consumers must be true as stated | No offer is shown until it is signed and billed (§6.1). Its words, its cutoff and the move-up sentence are registry entries that the site is pinned to |
| **Referral mail** | A sender who induces someone to forward its message can be treated as its sender (**unverified**; lawyer item 5) | **Knowlu never emails a friend.** There is no "invite by email" form; a person shares their link however they like |
| **Mailbox providers' sender rules** | Authenticated mail (SPF, DKIM, DMARC), one-click unsubscribe, a low complaint rate | §7.4 and §7.6. List mail has its own sender and subdomain, so its complaints cannot reach sign-in |

**What the lawyer is asked to rule on:**
1. Whether double opt-in is the "prior affirmative consent" that excuses update emails from naming
   themselves as advertisements.
2. Whether the confirm email is commercial or transactional under CAN-SPAM, and the CASL
   consent-request rule. The design treats it as commercial (§7.3).
3. CASL's identification fields and its 10-day and 60-day figures, and **whether the form's consent
   request must itself show the mailing address and contact details**, not only the mails. The form's
   line links §9's section, which names Knowlu and `support@knowlu.com`, but not the PO box.
4. The consent line (§10.2), the confirm mail's restatement (§7.3), §9's section and the first
   broadcast.
5. The referral rule, and the move-up sentence as a claim to consumers.
6. How the sender is named in the footer. This spec describes the sender as Knowlu.
7. The processor terms to accept with Supabase, Resend and Cloudflare, and the current CAN-SPAM
   penalty figure.
8. **How long the consent proof must outlive a leave.** Quinn ruled 3 years in the suppression record
   as the default, and **the lawyer sets the final period** (Q10 and Q11, merged; Quinn, 2026-11-09;
   §3.8).
9. **Whether keeping a keyed suppression record squares with an erasure request**, under the same
   ruling: the lawyer's period replaces the 3-year default for the fingerprint too.

## 9. The privacy page: the exact additions

**Read against:**
- `site/privacy.html` as published (version 2026-10-20, Last changed 2026-10-23);
- the L4 draft (`docs/notes/2026-10-19-l4-privacy-page-draft.md`), including its Q2 and Q3 on what counts
  as a smaller change;
- the L4 plan.

**The wording goes to `reviewer`, to Quinn, and to the lawyer before the first update email** (§8).
Places are named by section heading.

**Constraints on the wording:**
- **every sentence promises only what §3-§7 enforce** (r1 I8);
- none of it uses `site.rs:32-34`'s banned words;
- **none uses "founding" or a price** while §6.1's gate holds.

### 9.1 What we collect, and why: a new last entry, after "Gmail, if you connect it"

> **The early-access list, if you join it**
> If you ask for early access on this website, we keep:
> - the email address you typed;
> - when you first asked, when you last asked, and when you confirmed;
> - which version of the form's words you saw, which form it was, and which confirmation email we sent
>   you;
> - when the form last sent you an email;
> - if a price offer is running when you ask, which offer it was and whether you qualify for it;
> - your invite link's code, how many people joined through it and, if a friend's link brought you,
>   which entry's link that was;
> - if you leave, when and how: the link in an email, your mail program's unsubscribe button, marking
>   our email as spam, or our email bouncing;
> - for 90 days, which of our update emails we sent you;
> - after you leave or delete your address, the fingerprint described under *The early-access list*
>   below.
>
> We keep no name, no school, and no record of your network address. The list exists so we can tell
> you when Knowlu opens to you and send the few updates you asked for. Joining it does not create an
> account, and an account still needs you to be 18 or older.

The page promises a closed list ("If a category is not on this list, we are not collecting it",
`privacy.html:33`), so this entry is published before production takes its first address (§14.2, P2).

### 9.2 A new section, "The early-access list", after "Email you forward to Knowlu"

Its heading carries `id="early-access"`, which the form's consent line links to.

> When you join, we email the address you typed and ask you to confirm it. Nothing else is sent to it
> until someone confirms. If the form is used again for an address that has not confirmed, we send the
> request again, but never more than once a day or three times in a 30-day period. An address nobody
> confirms is deleted 30 days after it was first entered, once its last link has expired.
>
> Once you confirm, we send you updates only about Knowlu's opening and early access, and never more
> than two to you in any 30 days. If the form is used again for your address after you have confirmed, we
> email you your invite link instead, at most once a day and three times in a 30-day period. The list
> is never sold, rented or shared, and it is used for nothing but those emails.
>
> Every email has a link to leave. One press on the page it opens takes you off at once, and the same
> page can delete your address outright. Your mail program's own unsubscribe button works too. To see
> what the list holds about you, or to have it deleted, write to support@knowlu.com from that address.
> If you have a Knowlu account under the same address, Settings → Delete my data removes your entry
> along with everything else.
>
> **When you leave, or your address is deleted, we keep a fingerprint of it** so that nobody can add it
> to the list again and nothing is sent to it. The fingerprint is made with a secret key only our
> server holds, and it cannot be turned back into your address. With it we keep the reason, the date,
> and the facts of the consent you gave: which words you saw and when you asked and confirmed, but not
> your address. We keep this for three years. To be added again, write to support@knowlu.com from that
> address.
>
> Your invite link carries a code, not your address. Nobody who uses it learns who you are, and you do
> not learn who used it: you only move earlier in line. Knowlu never emails the people you share it
> with.
>
> To keep the form from being abused, our server counts the requests that come from each network
> address in a day, and how often each address has been emailed in a 30-day period. It keeps those
> counts under fingerprints made the same way, never the addresses themselves, and deletes them within
> two days and 31 days respectively. Cloudflare, which serves this website, and Supabase, which receives
> the form, keep their own logs of requests, with the network address each came from, for periods they
> set. We do not copy them.
>
> The list is not for children under 13. If we learn that an entry belongs to a child under 13, we
> delete it and keep nothing.

- "You only move earlier in line" is Q5 as ruled (Quinn, 2026-11-09).
- "Three years" is the default Quinn ruled (Q10 and Q11, merged; Quinn, 2026-11-09). If the lawyer sets
  another period, this sentence and §9.4 change with it, before the first update email.
- "Never more than two to you in any 30 days" is what `waitlist_batch`'s per-person check enforces
  (§4.5, r2 N4); "to you" makes the promise exactly as wide as the check.

### 9.3 Who else touches it

- **Supabase**: the bullet gains, at its end, "The early-access list lives there too, and it is what
  receives the early-access form."
- **Cloudflare**: unchanged ("this website and the downloads"). The preview project (§11.5) is part of
  "this website".
- **The closing "One more" paragraph names the provider and adds the list:**

> One more, doing nothing with your data but delivering it: Resend, which sends our account and billing
> email, the delivery checks Knowlu sends to a mailbox you forward from, and the early-access list's
> email. It sees the address each message goes to and keeps a copy of each message it sends, links
> included, for a period it sets, and it tells us when a message to the list bounces or is marked as
> spam.

- The company count ("Seven companies", `privacy.html:75`) does not change: Resend is the "one more"
  already counted outside the seven.
- The separate Resend account (§7.6) is the same company and needs no separate sentence.

### 9.4 How long we keep it: a new bullet, after "Email you forward to Knowlu"

> **The early-access list** — an address nobody confirms, 30 days after it was first entered, once its
> last link has expired; a confirmed one, until you leave the list or ask us to delete it, and no longer
> than 12 months after Knowlu opens to everyone; after you leave, 30 days, in which nothing is sent to it
> and you can join again from the leave page and keep your place, and then it is deleted. The
> fingerprint kept after you leave or delete, with the facts of your consent, three years. The record of
> which update emails went to you, 90 days. The counts that keep the form from being abused, two days
> for network addresses and 31 days for email addresses.

### 9.5 Your rights: the Delete bullet

After "…and the account and everything under it on our servers.", insert:

> It also removes an early-access entry under the same email address, keeping only the fingerprint
> described under *The early-access list*; an entry under a different address stays until you leave the
> list from one of its emails or write to us.

### 9.6 Do Not Track, now that Q3 allows one first-party script (Quinn, 2026-11-09)

Replace "and this is a static website" with "and this website is plain pages with one small script of
our own, which sends nothing except the early-access form when you submit it". The rest stays as it is.

### 9.7 A new version, not a "Last changed" edit (Q2: decided, Quinn, 2026-11-09)

**The page's own rule decides this.** "A new category collected" is a change that matters, with an
email and an ask "before the new use begins", and a smaller change is "a description that names no new
category, company or use" (`privacy.html:166`). §9.1 adds a category under exactly that heading. The
draft's "Last changed" recommendation broke the page's published rule, on the page whose acceptance the
app records (r1 I9).

- **Decided: a version bump** (Q2, Quinn, 2026-11-09).
  - The page's Effective and version lines move to the new date. So does "Last changed".
  - `PRIVACY_VERSION` (`app/src/account.rs:25`) moves with them: `contract-engineer`, then
    `contract-reviewer`, and `app/tests/static_assets.rs:1676-1686` keeps the pair equal.
  - Every account gets the email and L4's re-consent panel.
  - The page ships in a release. Under draft-first ordering (§11.3), the page is live on `knowlu.com` in
    the same deployment as the app's files, and before the GitHub Release is public. So the cost is one
    release and one re-consent screen.
  - **Production joins open only after that release** (§14.2, P2).
- **Not taken: a separate, unversioned early-access notice page.** It would have avoided the
  re-consent, at the cost of two privacy documents a reader must find and a CalOPPA question of whether
  the site's "policy" is one page or two.
- **The registry follows the page.** The newest consent version names the page version its words link
  to. W6's test pins it against the page, so it lands with the page and is never red before.

## 10. What the site must provide

Building the site is the port lane's job (W7). This section is its contract with the backend: the port
is done when these hold and its tests (§13.6) pass.

### 10.1 The form

- **One form**, with:
  - an email input (`type="email"`, `autocomplete="email"`, `required`);
  - the honeypot `website`: visually hidden, `tabindex="-1"`, `autocomplete="off"`, and hidden from
    assistive technology;
  - `form_id` and `consent_version` as hidden values.
- **The consent line sits beside the button**, in the registry's exact words (§10.2). When an offer is
  live, the offer text is shown on every form, in the registry's words.
- **One first-party script**, `site/waitlist.js` (Q3, Quinn, 2026-11-09). **The download button leaves
  the index** under the same ruling: until Knowlu opens, the index's one call to action is the form.
  - On submit it `POST`s JSON to `<functions base>/waitlist-join` with the fields above and the
    in-memory `invite` (§5.2), then shows the state from §10.5.
  - It sets no cookie, uses no `localStorage` or `sessionStorage`, and requests nothing but the
    functions origin and the site itself.
- **The functions base comes from `waitlist-config.json`**, which only `deploy-site` writes, per target
  (§11.3).
- **The form stays hidden unless that file loads and names an origin.** Without it the page shows one
  line, "Early access isn't open yet", and no input. So an index that reaches production any other way
  can never show a form that cannot submit (r1 I11).
- **Without JavaScript**, a `<noscript>` line says the form needs JavaScript. The confirm step needs a
  script anyway (the token is in the fragment).

### 10.2 The words the registry holds (drafts, for review)

| Registry entry | Draft words |
|---|---|
| consent `c1` | "We'll email you to confirm. After that, only updates about Knowlu's opening and early access, at most two in any 30 days. Leave any time. Privacy: knowlu.com/privacy#early-access". It does not name a price (§6.1) |
| confirm template `t1` | §7.3's text, hashed |
| move-up `m1` | "Each friend who joins with your link and confirms moves you a week earlier, up to five weeks." Q5 as ruled (Quinn, 2026-11-09); the words still go to review |
| offer | **none until the offer's amendment is signed** (§6.1; Q4, Quinn, 2026-11-09) |

### 10.3 Pages and routes

| Path | What it is |
|---|---|
| `/w/confirm` | Reads the token from the fragment and removes it from the address bar. One button, "Confirm my address", `POST`s it, then the page shows the result (§10.5). On `confirmed` it shows the invite link with a copy button and the move-up sentence, and the offer only when `founding` is true |
| `/w/leave` | Reads the token the same way. Two buttons: "Take me off the list" and "Delete my address now". After `left`, a third appears: "Join again", which `POST`s `rejoin` |
| `/r/<code>` | The `_redirects` line (§5.2) |
| `/releases/*` | Kept. **Supplied by `deploy-site`, never committed** (§11). `releases/.gitkeep` stays in git (`site.rs:35`) |
| `/privacy`, `/terms.html`, `/signed-in.html`, `/subscribed.html` | Kept at these paths: Google's consent screen and Stripe name `/privacy` and the terms (`HANDOFF.md:593`, `:596`), and the auth redirect lands on `signed-in.html` (`config.toml:21-22`) |

`/w/*` pages carry `<meta name="robots" content="noindex">`. Pages already serves `privacy.html` at
`/privacy` (`HANDOFF.md:50`). Whether a nested `w/confirm.html` is served at `/w/confirm` the same way is
**unverified** (U9); if not, the links name the `.html` file.

### 10.4 `_headers`

- **`/*`** carries exactly three headers:
  - `Referrer-Policy: no-referrer`;
  - `X-Content-Type-Options: nosniff`;
  - a CSP of `default-src 'self'; script-src 'self'; connect-src 'self' <functions origin>;
    frame-ancestors 'none'; base-uri 'none'; form-action 'self'`.

  `deploy-site` fills in the functions origin. Because `/*` also matches `/releases/*`, the test pins
  that rule's header names exactly (r1 M12), so nothing that could change how a download is served
  slips in. If the port needs `style-src` or `img-src` beyond `'self'`, it says why and the pin moves
  with it.
- **`/w/*`**: `X-Robots-Tag: noindex`. On the preview, `deploy-site` adds it to `/*`.
- **No rule names `/releases`.** `deploy-site` compares `latest.json`'s bytes and content type before and
  after (§11.3).
- **Web fonts are served from the site itself.** A third-party font host would see every visitor,
  against "no third party collects anything about you" (`privacy.html:163`).

### 10.5 The states the backend supports

The wording is the port's; the meaning is fixed here.

| Where | State | From | The page says, in substance |
|---|---|---|---|
| form | **not open** | no `waitlist-config.json` | early access isn't open yet; no input |
| form | **sent** | 202 `sent` | check your inbox for a link to confirm. Identical for every address. If nothing arrives: check spam, try again tomorrow, and **if you left the list before, write to support@knowlu.com to be added again** |
| form | **invalid** | 400 `invalid` | the address doesn't look right; the typed text stays |
| form | **rate-limited** | 429 `busy` | too many tries; try again later (over an hour reads as "tomorrow") |
| form | **offline** | the request never completed | you seem to be offline; nothing was sent |
| form | **error** | 5xx, 403, 413, `bad_request`, or an answer that does not parse | something went wrong on our side; try again later |
| confirm | **confirmed** | 200 `confirmed` | you're on the list; your invite link; the offer only if `founding` |
| confirm | **already confirmed** | 200 `already_confirmed` | you're already confirmed; the same link |
| confirm | **expired** | 410 | the link has expired; join again from the home page |
| confirm | **invalid link** | 404, or no fragment | the link doesn't work, perhaps replaced by a newer email or the address left; join again |
| leave | **unsubscribed** | 200 `left` | you're off the list and nothing more will be sent; "Delete my address now" and "Join again" offered |
| leave | **deleted** | 200 `deleted` | the address is deleted, and only its fingerprint is kept so nobody can add it again |
| leave | **rejoined** | 200 `sent` | a confirm mail is on its way; check your inbox to confirm again |
| leave | **rejoin later** | 200 `later` | nothing was sent and you are still off the list; try "Join again" tomorrow. The page keeps the token, so the button works again on a later visit to the same link (r2 C1, m7) |
| leave | **leave before confirming** | 200 `left` on a pending entry | the same as unsubscribed, without "Join again" (nothing is left to rejoin) |
| leave | **invalid link** | 404 | the link doesn't work; write to support@knowlu.com |
| confirm, leave | **rate-limited, offline, error** | as on the form | as on the form |
| any | **no JavaScript** | `<noscript>` | the form and these pages need JavaScript |

### 10.6 Pins the port moves, and the tests it owns

- `engine/tests/site.rs:22` forbids `<script` on the index and the privacy page. Under Q3 (Quinn,
  2026-11-09) the index may carry exactly one first-party script, `waitlist.js`, and the pin is narrowed
  to say so. The privacy page stays script-free.
- `site.rs:29` pins the index's download link. The download button leaves the index (Q3), so the pin
  becomes a check that `releases/Knowlu-setup.exe` is still served (§11.3, §13.5).
- `site.rs:26` pins the privacy sentence on the index. Keeping it, or moving the pin, is the port lane's
  question for Quinn.
- `site.rs:32-34`'s banned words stay banned.
- **W7 owns the tests that read the site's text**, so nothing is red before the port lands (r1 I11):
  - the consent line and the move-up sentence equal the registry's;
  - while the registry has no live offer, no file under `site/` contains "6.99" or "founding";
  - when one is live, every form shows it;
  - the `_headers` pins;
  - the DOM test, with **its own `ci.yml` step**, modelled on the mail scripts' step (`ci.yml:172-177`)
    and run with `--allow-read` only, never `--allow-net`.

## 11. Shipping the site without an app release

### 11.1 The trap, and the ruling it supersedes

- **Today one job publishes both halves.** `release.yml`'s `publish` job downloads the artifact into
  `site/releases/`, deploys Pages, then creates the GitHub Release (`release.yml:273-312`). A Pages
  direct upload replaces the whole deployment.
- **`releases/` is never in git**; only `.gitkeep` is (`site.rs:35`). It holds the versioned installer,
  the byte copy `Knowlu-setup.exe`, and `latest.json` (`scripts/release.ps1:293-327`).
- **So a site-only deploy from a checkout uploads an empty `releases/`.**
  - The updater, baked in as `https://knowlu.com/releases/latest.json` (`app/tauri.conf.json:26`),
    breaks for every installed copy.
  - The download 404s.
  - Nothing looks wrong.
- **The ruling this supersedes.** On 2026-10-20 the controller ruled, under Quinn's delegated go, that
  `release.yml` is the only deploy path for `site/` and that "a site-only deploy workflow is not taken"
  (`HANDOFF.md:563`). W-D3 asks for one. **Quinn superseded that ruling** (Q7, Quinn, 2026-11-09), and
  W8 records the supersession in `HANDOFF.md` beside the old line, so the record does not hold two
  answers.

### 11.2 The ways out

| Option | How | Against |
|---|---|---|
| **(a) Assemble `releases/` from a GitHub Release** | Download its installer and manifest; copy the installer to `Knowlu-setup.exe` | Needs the order fixed and the result checked against what is live |
| (b) Move `releases/` to R2 | A bucket on a custom hostname, and a Worker or route for `knowlu.com/releases/*` | A new service, a larger release-path change; R2 custom domains are per hostname (**unverified**) |
| **(c) One shared deploy workflow** | The only path to Pages, called by `release.yml` and by hand | Needs a source for `releases/` anyway, (a) or (b) |
| (d) Redirect `/releases/*` to GitHub | A `_redirects` rule | Whether Tauri's updater follows a redirect is **unverified**; a wrong guess breaks updates for everyone |
| (e) Commit the installer | Binary in git | History bloat, and it still loses the manifest's pairing with its tag |

**Decided: (c) on (a), with the release created as a draft first (§11.3)** (Q7, Quinn, 2026-11-09). In
release mode the bytes come from the run's own artifact rather than the draft (r2 N1); site-only mode
takes them from the latest published release. R2 stays the fallback if
the installer outgrows Pages' per-file limit (believed 25 MiB, **unverified**; today's installer
deploys, so it is under it).

### 11.3 Release mode: draft first, then the site, then public

`release.yml` becomes four jobs:

| Job | Environment | What it does |
|---|---|---|
| `build` | `release` (unchanged) | builds and signs; uploads the artifact |
| `publish` | `release` | checks the assets are present (as today, `release.yml:307-311`), then makes the draft, **idempotently** (r2 N2): if the tag already has a **published** release it refuses red; if it has a draft, it re-uploads the assets to that draft with `gh release upload <tag> … --clobber`; otherwise **`gh release create <tag> … --draft --verify-tag`**. So a rerun never makes a second draft for one tag. A draft is not public. No Pages deploy here any more. That `gh` finds a draft by its tag name is **unverified** (U23) |
| `site` | none (a job that calls a reusable workflow cannot declare one) | `uses: ./.github/workflows/site.yml` with `mode: release`, `tag: <the tag>`, `target: production`. No `secrets: inherit`; the called deploy job reads its secrets from its own `pages` environment (§11.5). **Its deploy waits for Quinn's approval on every release** (r2 m1) |
| `go-public` | none; `permissions: { contents: write }` | downloads the run's `release` artifact and `https://knowlu.com/releases/latest.json` with `-OutFile`, and **fails red unless the live file is byte-equal to the artifact's `latest.json`**, the bytes the draft was made from (or the live URL answers 404 and the release has no manifest). Only then `gh release edit <tag> --draft=false` |

**Why this order.** The page and the app go live on `knowlu.com` together, in one Pages deployment, and
the GitHub copy of the installer is public only after that deployment is verified. So a release that
moves `PRIVACY_VERSION` cannot be installed before its page is published (r1 I1). If anything fails
before `go-public`, the release stays a draft: nothing public is inconsistent, but the draft is not
harmless, because it is a release in flight until someone finishes it or deletes it. §11.7 says which,
for each failure (r2 N2).

**Quinn's approval stands in front of every app release** (r2 m1). The `pages` environment's required
reviewer gates the `site` job's production deploy, so every tag waits there for Quinn's approval. That
fits "tags on Quinn's word". "Prevent self-review" stays **off**, or the person who pushed the tag could
not approve it. An approval left waiting 30 days lapses and strands the draft (§11.7).

**The `deploy-check` alias, accepted without noindex** (r2 m3). Step 7's first deploy publicly serves
the production-configured site, installers included, at the `deploy-check` branch alias. That is
accepted:
- it serves bytes that are already public, or will be seconds later, on `knowlu.com` and GitHub;
- adding a header only for that branch would make the checked deployment differ from the production one,
  and the check exists to prove they are the same;
- Pages is believed to add `X-Robots-Tag: noindex` to branch aliases by default (**unverified**, U9),
  and nothing links to the alias.

**`site.yml` in release mode, step by step.** The assembly is `scripts/ci/assemble-site.ps1`, so
`workflows.rs` can run it against synthetic trees:
1. **The pages come from the tag, and never roll back what is live** (r2 N3). `actions/checkout` runs
   with `ref: <tag>`, `fetch-depth: 0` (the ancestry test needs history) and `persist-credentials: false`.
   - **Every production deployment carries `site-source.txt`**, one line: the full commit SHA its
     `site/` came from. `deploy-site` writes it (step 6). The repository is public, so the SHA tells
     nobody anything new.
   - The run downloads the live `https://knowlu.com/site-source.txt` with `-OutFile`; call its commit S.
   - **It refuses only if both hold:** `git rev-parse S:site` differs from `git rev-parse <tag>:site`,
     **and** S is not an ancestor of the tag (`git merge-base --is-ancestor S <tag>` fails). Deploying
     the tag then could only roll pages back.
   - So a merge to `main` during a release's 40-minute build no longer fails a signed release: the live
     source is still the older commit, an ancestor of the tag. A hotfix tag passes when its branch
     carries the live pages (§11.7).
   - **The bootstrap.** Until the first deployment under this workflow, the live site has no
     `site-source.txt` (it answers 404). S is then the commit of the latest published release's tag,
     because the old `publish` job deployed that commit's `site/` (`release.yml:286-298`).
   - S naming a commit the checkout does not hold is treated as "not an ancestor", so it refuses.
2. `site/releases/` must hold only `.gitkeep`.
3. **The releases come from this run's own `release` artifact** (r2 N1), never from the draft and never
   from "latest". `actions/download-artifact` fetches the artifact `build` uploaded (`release.yml:255-262`:
   `site/releases/**` and `publish/**`) into a fresh folder. A called workflow runs inside its caller's
   run, so the artifact is there, and it holds the very bytes the draft was made from. This needs no
   write token and no read of a draft. That a called workflow's job can download its caller's artifact
   is documented by GitHub but **unverified here** (U17); the first release after W5 proves it, and a
   failure there leaves only a draft (§11.7). The artifact is kept 7 days (`release.yml:262`), which
   bounds how late a rerun can come.
4. **Checks:**
   - `latest.json` parses;
   - its `version` equals the tag with one leading `v` removed (`v0.1.6` gives `0.1.6`);
   - its `platforms.windows-x86_64.url` is `https://knowlu.com/releases/` plus a file in the folder;
   - `Knowlu-setup.exe` is byte-equal to that file.
5. **The policy guard.** The privacy page's version must equal `PRIVACY_VERSION`, and the terms page's
   version must equal `TOS_VERSION`, in the tag's `app/src/account.rs`, read with `privacy-guard.ps1`'s
   pattern (`scripts/ci/privacy-guard.ps1:34`). It fails closed.
6. It writes `waitlist-config.json` and `_headers`' functions origin for the target, and, for
   production, `site-source.txt` holding the commit the pages came from (step 1). The values come from
   workflow variables and the checkout's own `git rev-parse HEAD`, passed through `env:`, never
   interpolated into a `run:` line.
7. **Deploy and verify, in the `pages` environment's job:**
   1. `pages deploy <out> --project-name knowlu --branch deploy-check`, then that deployment's
      `latest.json` bytes, both installers' status and length, and `/privacy`, `/terms.html`,
      `/signed-in.html` and `/subscribed.html` are checked;
   2. then `--branch main`, and the same checks against `knowlu.com`.

   Every fetch uses `-OutFile`, because `Invoke-WebRequest`'s `.Content` re-encodes the body (r1 M12).

### 11.4 Site-only mode (`workflow_dispatch` on `main`)

1. **The pages come from `main`.** The job runs only when `github.ref` is `refs/heads/main`, and the
   environments' branch policies enforce the same (§11.5). The same never-roll-back rule as release
   mode, step 1, holds with `main`'s head in place of the tag: after a hotfix released from a branch,
   site-only mode refuses until that branch is merged into `main` (§11.7).
2. **A release in flight stops it, and only a newer one** (r2 N1, N2).
   - **A separate job, `drafts`, looks**, because GitHub lists draft releases only to a token with
     push access. It holds `permissions: { contents: write }` and **no Cloudflare token and no
     environment**. It runs `gh api` on the releases list and outputs one value, `in_flight`: `yes` or
     `no`.
   - `in_flight` is `yes` when **a draft exists whose `v*` tag is newer** (by version order) than the
     latest published release's tag. A stranded draft for an older or superseded tag does not block the
     site; §11.7 says to delete it.
   - The deploy job `needs: drafts` and refuses on `yes`. It keeps `contents: read`.
   - The draft list is not checked a second time just before the production deploy (Revision 1 said it
     was; a second write-scoped job there buys nothing). A release that reaches Pages inside that window
     is caught by step 5 if it deploys first. If it deploys after, its own `go-public` re-check fails red
     and §11.7's rerun finishes it.
3. **The releases come from the latest *published* release** (GitHub's "latest"; its exact semantics are
   **unverified**, U10). This is the only mode that uses "latest", and a read token can download a
   published release's assets.
4. The same checks as release mode, step 4.
5. **The live byte check.** Download the live `latest.json` with `-OutFile`; it must be byte-equal to the
   assembled one. If the release has no manifest, the live URL must answer 404. Anything else means a
   release is in flight or the site is not what the latest release says, and the run refuses.
6. The policy guard, against the latest published tag's `account.rs`. The review found it passes today:
   `v0.1.5` carries 2026-10-20 and 2026-09-10, the pages' versions (U12). Under Q2's version bump, a
   site-only deploy therefore cannot publish the new privacy version; only its release can.
7. The config, then the same deploy and checks as release mode, step 7.

**`dry_run`** stops after step 7's check-branch deploy and its checks. S12 uses it.

### 11.5 Environments, tokens and the preview (r1 I2)

- **`pages`** holds `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`. Its deployment-branch policy
  allows `main` and tags `v*` only, and it has Quinn as a required reviewer, with "Prevent self-review"
  off (r2 m1). **Both modes' production deploy jobs declare it**, so Quinn approves every release's site
  deploy and every site-only deploy.
  - The secrets move out of the `release` environment or the repository (where they live today is
    U11); Quinn moves them.
  - A job inside a called workflow that declares an environment reads that environment's secrets.
    GitHub documents this (r2 m10); W5's dry run is still the proof (U19).
- **A tag ruleset limits who can create `v*` tags** (r2 m2; Q7, Quinn, 2026-11-09). The branch policy
  admits any `v*` tag, and the approver sees the deployment, not the workflow diff, so anyone able to push
  a tag on an arbitrary commit could otherwise run that commit's `site.yml` with the token once it is
  approved. The ruleset restricts creating, updating and deleting `v*` tags to the repository's admins,
  which today means Quinn; it is a GitHub setting, not repository content. It closes the same exposure
  for `release`'s signing secrets. P1 sets it. That rulesets are available to this repository on its
  plan is believed (it is public) but **unverified** (U24).
- **The preview is a separate Pages project, `knowlu-preview`**, deployed only by the preview job.
  - That job's own environment, `pages-preview`, has a branch policy of `main` and holds its own token.
  - **Plan as if that token can deploy production** (r2 m11): Cloudflare's Pages token permission is
    believed to be account-wide, not per project (U21). The design holds anyway: the token is reachable
    only from reviewed code on `main`, and a `workflows.rs` pin forbids the preview job from naming
    `knowlu` (§11.6). If Cloudflare later offers a per-project scope, Quinn narrows the token.
  - The preview is served at `knowlu-preview.pages.dev`. It carries no `releases/`, its config names
    staging, and every page carries `X-Robots-Tag: noindex`.
- **What the reviewer gate covers, said plainly:**
  - **It covers** every production deploy that the workflows on `main` or a `v*` tag can make.
  - **A branch cannot get round it.** `workflow_dispatch` runs the workflow as it exists on the chosen
    branch, but a branch's copy cannot reach either environment, so it gets no token.
  - **It does not cover two things.** A reviewed change on `main` that points the preview job at
    `knowlu`: a `workflows.rs` pin forbids that (§11.6). And a person using the Cloudflare dashboard
    directly.
- **Why not one environment for both targets.** It would put Quinn's approval in front of every preview
  deploy, and it would still leave a single token able to deploy either project.

### 11.6 CI contracts the change must keep (r1 I3)

- **`engine/tests/workflows.rs`:**
  - `release_runs_only_on_version_tags_and_names_every_secret_the_script_needs` (`:66-77`): the two
    `CLOUDFLARE_*` names move to a new assertion on `site.yml`. `release.yml` keeps the rest.
  - `release_builds_and_publishes_in_two_jobs_joined_by_an_artifact` (`:163-192`): `publish` carries
    `gh release create` with `--draft`, and no `cloudflare/wrangler-action@`. `environment: release`
    still appears exactly twice (`build`, `publish`). Its comment changes: `publish` now creates a draft
    nobody can see, the `pages` environment gates what goes public on `knowlu.com`, and `go-public`
    flips the draft only after the live check.
  - `every_action_is_pinned_to_a_full_commit_sha` already covers `site.yml` through `all_workflows()`
    (`:13-23`, `:47-56`).
  - **New pins:**
    - no workflow but `site.yml` contains `pages deploy`;
    - `site.yml` names no `TAURI_SIGNING_`, no `AZURE_` and no `id-token`;
    - no workflow uses `secrets: inherit`;
    - the preview job names `--project-name knowlu-preview` and never `knowlu` alone; each deploy job
      declares its environment;
    - every checkout in `site.yml` has `persist-credentials: false`, and its top-level `permissions` is
      `{}`;
    - `release.yml`'s order is draft, then `site`, then `go-public` with `--draft=false`;
    - **`publish` is idempotent** (r2 N2): it names `--clobber` on the reuse path and refuses on a
      published tag, and `gh release create` appears once;
    - **the `drafts` job** (r2 N1) is the only job in `site.yml` with `contents: write`; it declares no
      environment and names no `CLOUDFLARE_`; no job that names `CLOUDFLARE_` holds more than
      `contents: read`;
    - **release mode never downloads from a release**: its deploy job uses `actions/download-artifact`
      and does not run `gh release download` (r2 N1);
    - `site.yml`'s release-mode checkout has `fetch-depth: 0`, and the assembly writes
      `site-source.txt` for production (r2 N3).
- **`security.yml` stays clean** (zizmor and actionlint, `security.yml:19-49`): inputs and variables
  reach `run:` only through `env:`; per-job `permissions`; `persist-credentials: false`; no
  `pull_request_target`.
- **Concurrency is set at job level, in separate groups**: `pages-release` for release mode,
  `pages-site` for site-only mode, `pages-preview` for the preview, each with `cancel-in-progress:
  false`.
  - GitHub keeps one pending run per group and cancels the older pending one. With separate groups, **a
    manual run can never replace a release's pending deploy**.
  - The two production groups cannot interleave unseen: site-only mode refuses while a newer draft
    exists (§11.4 step 2) or the live manifest is not the latest release's (step 5), and `go-public`
    fails red if the live manifest is not its own (§11.3).
  - A release whose deploy is superseded by a newer tag's stays a draft. It is not harmless: it is
    deleted by hand, per §11.7, and until then it does not block site-only mode, because it is not
    newer than the latest published release once the newer tag goes public (r2 N2).
- **Releases stay CI-only.** Only `release.yml` builds and signs, on a `v*` tag. `deploy-site` builds and
  signs nothing, holds no signing secret, and never runs self-hosted. A hand-run `pages deploy` stays
  unsupported.

### 11.7 Recovery and hotfix runbooks (r2 N2, N3, m1)

W8 copies these into `docs/reference/app.md`'s release steps. Each starts by asking one question: **did
this release's production Pages deploy happen?** The `site` job's log answers it, and so does the live
`https://knowlu.com/site-source.txt`, which names the tag's commit once it has.

| What happened | State | What to do |
|---|---|---|
| `publish` failed, or was rerun | no draft, or one draft for the tag | `gh run rerun <id> --failed`. `publish` reuses the tag's draft, so a rerun never makes a second one |
| `site` failed **before** the production deploy: the never-roll-back check, the policy guard, an asset check, a rejected approval | a draft; the live site is still the previous release | Fix the cause. If the tag's commit is fine, rerun the failed jobs within the artifact's 7 days (`release.yml:262`). If not, **delete the draft and keep the tag** (`gh release delete <tag>`, without `--cleanup-tag`), then cut a new tag |
| The `pages` approval was left 30 days and lapsed (r2 m1) | as above, and the artifact has expired | Rerun the whole run (`gh run rerun <id>`): it rebuilds and re-signs from the same tag, and `publish` reuses the draft. Or delete the draft and keep the tag |
| `site` deployed production, then `go-public` failed (a flaky fetch, a token) | **`knowlu.com` serves the new release while it is still a draft** | **`gh run rerun <id> --failed`**, which reruns only `go-public`. Never delete this draft: site-only mode would then compare the latest *published* release with the newer live one and refuse until the next release |
| `go-public` failed because the live manifest is not its own (another deploy landed after this one) | the live site is not this release's | Rerun the whole `site` job, then `go-public`. Site-only mode cannot have caused it unless the two raced inside §11.4's window |
| A newer tag superseded this run's pending deploy (`pages-release` keeps one pending run) | an older tag's draft that will never deploy | Delete the older draft, keep its tag. Once the newer release is public, the older draft no longer blocks site-only mode, but it is still deleted so the list stays clean |
| A draft made by hand in GitHub's interface | a draft nobody's run owns | Delete it. If its tag is newer than the latest published release, it blocks site-only mode until then |

**The hotfix runbook** (r2 N3). A fix to an installed app that cannot wait for `main`:
1. Branch from the tag being fixed.
2. **Bring the live pages onto the branch**, as one commit: `git checkout <S> -- site`, where S is the
   commit in the live `site-source.txt` (usually on `main`). The hotfix then publishes exactly the pages
   already live and nothing new. Without this the never-roll-back check refuses it, because S is not an
   ancestor of the branch and its `site/` differs. Pulling the rest of `main` in is never the fix,
   because it would ship unreleased app work.
3. Make the fix, tag the branch, and let `release.yml` run as usual, Quinn's approval included.
4. **Merge the hotfix branch into `main`** soon after. Until then site-only mode refuses (§11.4 step 1),
   because the live source, the hotfix commit, is not an ancestor of `main`.

## 12. What must not break

| What | Why it matters | What holds it |
|---|---|---|
| `https://knowlu.com/releases/latest.json`, the versioned installer and `Knowlu-setup.exe`, byte for byte as their release published them | Every installed copy updates from that URL (`tauri.conf.json:26`) | Assembly from the run's own artifact (release mode) or the latest published release (site-only mode); the live byte check; post-deploy checks; `go-public`'s re-check (§11.3-§11.4); §13.5's synthetic-tree tests |
| **No app is public before its page** | A release that moves `PRIVACY_VERSION` must not be installable before the page is live | Draft-first: the GitHub Release is published only after `knowlu.com` serves the page and the files together (§11.3) |
| **Releases are CI-only** | CLAUDE.md; the updater key lives only in `build` | `deploy-site` builds and signs nothing and holds no signing secret (pinned, §11.6) |
| **The production reviewer gate** | W-D3: production on Quinn's word | The `pages` environment, with branch policy and a required reviewer, on both modes, so **Quinn approves every app release's site deploy** (r2 m1); the `v*` tag ruleset (r2 m2); the preview's separate project and environment; pins (§11.5-§11.6) |
| **Live pages never roll back** | A release must not undo a page a site-only deploy published, and a hotfix must not ship unreleased pages | `site-source.txt` and the ancestor rule, in both modes (§11.3 step 1, §11.4 step 1); the hotfix runbook (§11.7) |
| **A stranded draft is recoverable, and only a newer one blocks the site** | A failed or superseded release must not freeze site-only deploys | The scoped `drafts` job, idempotent `publish`, and §11.7's runbook |
| The pages others land on: `/privacy`, `/terms.html`, `/signed-in.html`, `/subscribed.html` | Google's consent screen, Stripe and the auth redirect (`HANDOFF.md:593`, `:596`; `config.toml:21-22`) | §10.3 keeps the paths; every deploy checks all four (§11.3) |
| **Sign-in codes keep arriving** | Students must be able to sign in | List mail uses its own Resend account (Q6, Quinn, 2026-11-09), key, sender, subdomain and webhook (§7.6); the waitlist modules cannot read `EMAIL_*` (pinned, §13.2). The existing DNS records under `knowlu.com` are untouched |
| **No anonymous read of the list, ever** (W-D1) | The list is personal data of people who are not account holders | §3.4; §13.4's tests and S1 |
| **An opt-out is permanent until its owner lifts it** | CAN-SPAM; reputation; decency | The suppression record, written in the same transaction as every leave (§3.8); tested (§13.4) |
| **No address in any log** | The function log is readable by whoever runs the project | §4.1's wrapper and catch; §13.2's echoing-stub tests |
| The migration guards and the pins a new migration edits | They hold the schema's posture | §3.4 passes `migrations_test.ts:56-81` and `migrations/migrations_test.ts:286, 350, 533, 2080-2182, 2580-2640`. W1 updates the pins listed in §2 |
| **Delete my data** | The page promises the account "and everything under it" goes (`privacy.html:151`) | One definer call, `waitlist_forget_key`, before the `accounts` row goes. It cannot be a `restDelete` (`account/index.ts:139-142`); zero rows is success; a failure fails the purge as every step's does |
| The privacy page's pins, and its version paired with `PRIVACY_VERSION` | `site.rs:96-299`; `static_assets.rs:1676-1686` | §9 adds sentences and removes none that a pin names. Q2's bump moves both together, and the deploy guard keeps the live page paired with the app that ships it |
| "No analytics scripts", "no cookies dropped by anyone else" | `privacy.html:163`; the closed list (`:33`) | §10's single first-party script, no storage, no third-party request, self-hosted fonts; no Turnstile |
| Tests never leave the machine | CLAUDE.md | Injected resolver, clock, random source and mail endpoint; the one listener on `127.0.0.1:0`; the DOM test runs without `--allow-net` |
| No engine or app source change; no frozen reference touched | CLAUDE.md's two rules | §1. The contract-list change is `PRIVACY_VERSION`, under Q2, through `contract-engineer` and `contract-reviewer` |

## 13. The tests that prove it

TDD: each test is written first and seen to fail. **No test leaves the machine.** Every resolver, clock,
sleep, random source, environment read, mail endpoint and database call is injected, and the one
listener binds `127.0.0.1:0` (`ci.yml:158-171` allows exactly that). **Each test is listed under the task
that owns it, so nothing is red before its code lands** (r1 I11).

### 13.1 Pure modules (W1)

- **The address**: trimming and lower-casing; `fold_key` for Gmail and for other domains; a table of
  syntax cases (each length limit, dots, hyphens at label edges, a one-letter final label, non-ASCII
  refused, punycode accepted); the DNS check (no record is `invalid`; an error or a 1.5 s timeout is
  valid).
- **The keys**:
  - `key_hash` is the HMAC of `email_key`, not of the typed address; `Mixed.Case@Example.invalid` and
    `mixed.case@example.invalid` give the same hash on the join path and on the events path (r2 m6);
  - with `WAITLIST_ADDR_KEY_PREV` set, the join yields both hashes and new records use only the new key
    (r2 m5);
  - the `ip:` prefix is the full IPv4 address, and the /64 for IPv6, so two addresses in one /64 share a
    bucket and two in different /64s do not.
- **The registry**:
  - a hash pin for every published consent, confirm template, offer and move-up entry;
  - the newest and the previous consent are accepted and any other is refused;
  - **the offers list is empty**; the test that pins it changes in the same pull request as the signed
    offer's registry entry.
  - **No test here reads `site/`.** Those pins are W6's and W7's (§13.6).
- **The tokens**: a leave token round-trips; one flipped byte is refused; `_PREV` is accepted and a
  third key is not; a confirm token's stored hash is its SHA-256, and no URL holds the hash.

### 13.2 Handlers (W2b, W2c), pure with injected dependencies

- **join:**
  - each check in §4.2's order;
  - **byte-identical `sent` answers** (status, headers, body) for a new address, a pending one in and out
    of its window, a confirmed one in and out of it, a suppressed one, one at its 30-day cap, and a
    honeypot hit, each making exactly one call to the database;
  - **with the budget at zero**, a new and a listed address both get `busy`, and the address is never
    read;
  - **the budget is charged only by a claimed mail**: an invalid address, a honeypot hit, an in-window
    repeat and a suppressed address charge nothing;
  - **each missing secret in the configuration gate** answers `error`, and nothing is sent;
  - **the answer returns while the mailer never resolves** (a mail stub whose promise never settles),
    and the pending send is handed to `waitUntil`;
  - a failed send calls `waitlist_mail_result(entry, false)`;
  - CORS (a listed origin, an unlisted one, and a `POST` without `Origin`), and a body over 4,096
    characters.
- **No address in any log (r1 I4).** Captured output must not contain the address, the token, the IP
  address or any hash when:
  - the database stub answers 400 with a body that echoes the address, as Postgres's "Failing row
    contains" does;
  - the resolver throws an error that carries the domain;
  - the mail stub answers 422 with the address in its body;
  - the handler throws an unexpected error whose message contains the address.
- **The list never reads sign-in's mail settings**: an environment stub that throws on any `EMAIL_*`
  read is never touched.
- **confirm:** each outcome in §4.3; `GET` refused; `founding` in the answer.
- **leave:**
  - the RFC 8058 form `POST` without `Origin` answers 200 with an empty body;
  - `leave`, `delete` and `rejoin` each call `waitlist_leave` with their action;
  - **`rejoin` answers `later` and sends nothing** when the transition reports no claim, and `sent` with
    the mail handed to `waitUntil` when it reports one (r2 C1);
  - a bad HMAC answers 404;
  - `GET` answers 303 to the site with the fragment and makes no database call.
- **send:**
  - each 409 gate, **including the frequency gate**: with two broadcasts begun in the last 30 days a
    third is refused, while a broadcast already begun may continue;
  - **the per-person limit** (r2 N4): when the batch stub reports entries held back, they are counted in
    `held`, not sent, and not marked sent;
  - sends are paced by the injected sleep, at least 600 ms apart;
  - one call sends at most `limit`;
  - it stops at `mail:send:day` and never touches `mail:join:day`;
  - the job token is required; `dry_run` sends nothing; `Idempotency-Key` is set.
- **events** (r2 N5, m6):
  - a good and a bad signature;
  - an event tagged with this project is handled; one tagged with the other project is dropped, even if
    its `from` would match;
  - **an untagged event whose `from` is this project's sender is handled**; an untagged one with the
    other project's sender is dropped;
  - an event that matches neither is dropped, logged only as `waitlist-events: unattributed`, and counts
    one hit on `events:unattributed:day`;
  - a complaint for `Mixed.Case@Example.invalid` is hashed as `mixed.case@example.invalid`;
  - a complaint for an address with an entry calls `waitlist_leave`, and one for an address without
    calls `waitlist_suppress_key`.

### 13.3 The mail, against a stand-in for Resend (W2b)

- **A `Deno.serve` listener on `127.0.0.1:0`** plays Resend through the injected
  `WAITLIST_EMAIL_API_URL`. Each test asserts the request:
  - the bearer header, which is the list key, never sign-in's;
  - both parts, the tag, and both `List-Unsubscribe` headers;
  - `Idempotency-Key` on sends;
  - the timeout path, and a non-2xx answer becoming a failed send.
- **The templates**: every mail and broadcast has both parts; the postal address in each footer; no
  `Re:` or `Fwd:`; no `<img`; links only to `knowlu.com` or the functions origin; the preview line
  exactly when preview is on. The confirm mail's text hashes to its registry entry.

### 13.4 The database (W1 pins, W2a migration, W4 behaviour)

- **Static pins** in `migrations/migrations_test.ts`'s style:
  - RLS on all four tables, with no policy;
  - the revokes, `service_role` included;
  - every waitlist function `security definer` with its `search_path`, execute granted to `service_role`
    only, or to no API role at all (`waitlist_new_code`, `waitlist_sweep`);
  - every `admin.waitlist_*` object revoked from `public`, `anon`, `authenticated` and `service_role`;
  - the sweep's minute field unique.
- **The pins a new migration must edit**, updated **in W2a's pull request, with the migration** (§2; r2
  m9: they pass only once the migration exists):
  - the latest file named by suffix (`files.at(-1)`, `:584`);
  - the "stamped after every other file but the named later ones" lists and `LATER_LANES` (`:1988`,
    `:3336-3348`, `:3509-3520`, `:3642-3649`);
  - the corpus counts of function creations (`:347`, 51 today, plus the waitlist's) and view creations
    (`:368`, 10 today, plus `admin.waitlist_counts`).

  W2a re-reads each at its own base before editing.
- **Behaviour.** These are pgTAP if the RLS suite has merged, otherwise
  `cloud/supabase/proofs/waitlist_staging.sql`, rolled back, run by the controller. They cover:
  - `anon` and `authenticated` can neither read, count, write nor execute anything of the list's;
  - every row of §3.5's join rule, the suppressed key included;
  - **a repeat inside the window keeps the first link working** (r2 C1): a join, a second join for the
    same address inside 24 hours (action `none`), then `waitlist_confirm` with the first token's hash
    answers `confirmed`. The row's `confirm_hash`, `requested_at`, consent fields and
    `confirm_template` are byte-equal before and after the second join;
  - the same for a repeat refused only by the `addr:` cap, and only by the day's budget;
  - the per-address cap of 3 in a 30-day window, surviving the pending row's deletion;
  - the budget charged only by a claimed mail, and `waitlist_mail_result` releasing all three claims;
  - **every leave writes its suppression record in the same transaction**, and a later join for that key
    sends nothing;
  - `waitlist_forget_key` on an address with no entry deletes nothing and writes no suppression record
    (r2 m4);
  - rejoin with a claim lifts the record, issues a new token and keeps the founding flag and credits;
    **rejoin inside 24 hours of the last mail still claims** (the window is skipped);
  - **rejoin with the cap spent, or the budget gone, returns `later` and changes nothing**: the entry is
    still `unsubscribed`, the suppression record still there, and no token written (r2 C1);
  - **a leave between two pages of a send withholds the second page's mail to that entry**;
  - **the staggered send** (r2 N4): an entry with send rows for broadcasts A (day 10) and B (day 12) is
    held back from broadcast C's batch on day 31, and picked up on day 41, when A's row is more than 30
    days old; an entry with one row in the window is not held back;
  - **the code-collision redraw**: inside the rolled-back transaction, the code generator
    `waitlist_new_code()` is replaced by one that returns a taken code twice, then a free one;
  - the credit rules; `referred_by` frozen at first set; the `founding` freeze;
  - the sweep's deletions and their times (§3.6).

### 13.5 The deploy (W5)

- **`engine/tests/workflows.rs`**: the changes and new pins in §11.6.
- **`assemble-site.ps1` against synthetic temp trees**, as `privacy-guard.ps1` is run (`:322-443`). The
  script takes the release's assets, the live manifest, the live `site-source.txt` and (site-only) the
  draft list as paths, and the workflow does the downloading, so no test touches the network. Cases:
  - **the never-roll-back rule** (r2 N3), on a synthetic git repository built in the temp tree:
    - the live source S equal to the tag passes;
    - S an ancestor of the tag, with a different `site/`, passes (a merge to `main` during the build);
    - S not an ancestor, with the same `site/`, passes (a hotfix that carried the live pages);
    - S not an ancestor, with a different `site/`, is refused;
    - S naming a commit the repository does not hold is refused;
    - no `site-source.txt` (404) falls back to the latest published tag's commit;
    - the same cases with `main`'s head in place of the tag, for site-only mode.

    The repository is made with local `git init` and commits in the temp tree; nothing is fetched;
  - **release mode**:
    - the artifact's folder yields exactly three files, with `Knowlu-setup.exe` byte-equal to the
      installer (r2 N1);
    - a release without a manifest yields two;
    - `v0.1.6` matches `0.1.6`, and `vv0.1.6` or `0.1.7` is refused;
    - production output carries `site-source.txt` with the tag's commit; the preview's does not need it;
  - **site-only mode**, with the draft list passed in as a file, as the `drafts` job would (r2 N1, N2):
    - a draft whose tag is newer than the latest published release is refused;
    - a draft for an older tag, or a superseded one, passes;
    - no drafts passes;
    - live bytes equal pass, and differing bytes are refused;
    - a live 404 passes only when the release has no manifest;
  - **both modes**:
    - a committed file in `site/releases/` is refused;
    - a manifest URL naming a missing file is refused;
    - a privacy or terms version ahead of the tag's is refused;
    - `waitlist-config.json` and the `_headers` origin are written for the target, and the preview adds
      noindex and omits `releases/`.

### 13.6 The site (W6, W7)

- **W6, with the page:**
  - §9's sentences in their sections (`page_text`, `missing`, `site.rs:96-120`);
  - the version and Effective lines equal to the new `PRIVACY_VERSION` (`static_assets.rs:1676-1686`);
  - the registry's newest consent names that page version. It lands with the page, never before.
- **W7, with the port:**
  - `_redirects` has the `/r/` line;
  - `_headers`' `/*` rule names exactly the three headers of §10.4, and its CSP is exactly §10.4's with
    the origin placeholder (r1 M9);
  - no rule names `/releases`; `site/releases/` holds only `.gitkeep`;
  - the privacy page carries no `<script`; under Q3, the index carries exactly one script, `waitlist.js`,
    from the site, and nothing inline or from another host;
  - the consent line and the move-up sentence equal the registry's;
  - **with no live offer, no file under `site/` contains "6.99" or "founding"**; with one, every form
    shows it;
  - **the DOM test of `waitlist.js`** (linkedom, outside `site/`, in its own `ci.yml` step with
    `--allow-read` only). It checks:
    - every state in §10.5, the not-open state included when `waitlist-config.json` is missing;
    - the invite is read, sent and removed from the address bar;
    - the confirm and leave pages read the fragment, remove it, and `POST` only on a button;
    - a rejected fetch shows offline;
    - stubs for `document.cookie`, `localStorage` and `sessionStorage` that throw on any use are never
      touched.

### 13.7 Over the wire, on staging and production

The smoke steps in §14 (S1-S14, P5). They need a deployed project and a mailbox, so they are the
controller's, not CI's.

## 14. Rollout

**Who does what.**
- The controller (the main session) runs every deploy, sets every secret and runs every smoke. **No
  subagent holds a session, a key or a job token.**
- Production happens only on Quinn's word.
- Test inboxes are mailboxes the controller can read; none is named in the repo.

### 14.1 Staging, and the preview behind it

**Before anything is mailed:**
- **the PO box**: Quinn supplies it, and it goes on every list mail, the preview's included (W-D4, r1
  M5);
- **the list's Resend account on a paid plan** (Q6, Quinn, 2026-11-09): its own subdomain verified, its
  key, staging's own sender addresses (distinct from production's, §4.8), and a webhook to staging's
  `waitlist-events`.

**The order, as Quinn ruled (Q8, Quinn, 2026-11-09):** the PO box and the list's account first; then
S1-S14, every one of them passing; only then is the preview link shared.

**Then:**
1. The migration on `knowlu-staging`, by the house procedure (`cloud/supabase/README.md`).
2. Secrets:
   - generated blind, never printed: `WAITLIST_IP_KEY`, `WAITLIST_ADDR_KEY`, `WAITLIST_UNSUB_KEY` and
     `WAITLIST_JOBS_TOKEN`;
   - `WAITLIST_ORIGINS` and `WAITLIST_SITE_ORIGIN` set to `https://knowlu-preview.pages.dev`;
   - `WAITLIST_PREVIEW=on`;
   - the list's mail key, URL and senders, and `WAITLIST_WEBHOOK_SECRET`;
   - `WAITLIST_POSTAL_ADDRESS`;
   - `WAITLIST_JOIN_PER_DAY=5` and `WAITLIST_SEND_PER_DAY=0`;
   - **not** `WAITLIST_UPDATES`.
3. The five functions deployed.
4. **The smoke, all before the preview link is shared:**

| Step | What is done | What must be seen |
|---|---|---|
| S1 | With the anon key: `GET /rest/v1/waitlist`; a `HEAD` asking for an exact count; `POST /rest/v1/rpc/waitlist_join` | three permission errors, and no count |
| S2 | `OPTIONS` from the preview origin, with no `apikey` (U3); from another origin; a `POST` with no `Origin` | 204 with the allow header; 403 with none; 403 |
| S3 | Join a test inbox from the preview | the `sent` state; one confirm mail from the list's sender with both parts, the restated consent, the postal address, the preview line and both `List-Unsubscribe` headers |
| S4 | Join again at once; then confirm, move the claim back 25 hours in the SQL editor, and join once more | byte-identical `sent` answers with no second mail, and **the first mail's link still confirms in S6** (r2 C1); then the "already on the list" mail |
| **S5** | **Which header carries the client address** (U2): 31 joins in 10 minutes from one machine, each with a different forged first hop in `x-forwarded-for`, using invalid addresses so no mail is spent | if the 31st is `busy`, the reader cannot be forged. If not, switch the reader to the header the gateway itself sets and repeat |
| S6 | Confirm from the mail; press again; a backdated token; a superseded token | `confirmed` with the invite link; `already_confirmed`; `expired`; `invalid` |
| S7 | Open `/r/<code>` on the preview; join and confirm a second inbox through it; then a plus-tag copy of the first inbox through its own link | lands on `/?invite=…`, which the page strips; the referrer's `credits` is 1 (`admin.waitlist_entry`); the plus-tag copy credits nothing |
| S8 | One-click `POST ?t=` with `List-Unsubscribe=One-Click` and no `Origin`; `GET` on the leave link; the leave page's buttons; **"remove this address now" from a fresh confirm mail, then a new join for that address** | 200 and `unsubscribed` with a suppression row; a 303 that changes nothing; `left`, `rejoined` and `deleted` as pressed; **the new join answers `sent` and no mail arrives**. Then, on a left entry whose `addr:` bucket is set to 3 in the SQL editor, "Join again" shows **rejoin later**, and the entry is still `unsubscribed` and suppressed (r2 C1) |
| S9 | Delete my data on a scratch account whose address has an entry, in a scratch profile on a scratch vault, driven through the DOM | the entry is gone with the account, and a suppression row exists (CLAUDE.md's live-run rule) |
| S10 | `WAITLIST_JOIN_PER_DAY=2`, then three joins: two new addresses, then one listed one; then read both projects' budget secrets against the list account's plan | the third answers `busy`, the same for the listed address as for a new one; **the budgets' sum is under the plan's daily quota**; the budget is reset |
| **S11** | The raw source of a received confirm mail | DKIM passes for the list's subdomain, and its `h=` covers both `List-Unsubscribe` headers; DMARC passes; no rewritten links and no tracking image |
| **S12** | `deploy-site` to the preview; production with `dry_run`; a dispatch from a scratch branch; then **a draft release made by hand for a version newer than the latest published one**, with no assets and naming a tag that does not exist yet, another production `dry_run`, and the draft deleted (r2 N1). **No `v*` tag is pushed**: a pushed tag would start `release.yml`. GitHub creates a draft's tag only when the draft is published (**unverified**, U25); if it creates one earlier, S12 deletes the draft at once and uses the synthetic-tree cases (§13.5) as the proof instead | the preview serves the site at `knowlu-preview.pages.dev` with noindex and no `releases/`; the dry run's check-branch deploy serves `latest.json` byte-equal to live, both installers and the four pages, and its policy guard passes; the scratch branch's dispatch cannot reach either environment; **with the newer draft present, the `drafts` job reports `in_flight: yes` and the dry run refuses before any deploy** |
| S13 | `waitlist-send`, with `WAITLIST_SEND_PER_DAY` and `WAITLIST_UPDATES=on` set for this step only: `dry_run`; two test broadcasts to the test inboxes; a third; then the sweep after backdating rows | the 409 gates refuse first; one mail per confirmed inbox with a send row each, and a second call sends nothing; **the third broadcast is refused by the frequency gate**; the sweep deletes exactly §3.6's rows. Both settings are then reverted |
| **S14** | **Unconditional** (r2 N5). A complaint and a hard bounce. The complaint through Resend's test address for complaints if it has one, **otherwise a real test inbox marks a confirm mail as spam**. The bounce through Resend's test address for bounces if it has one, otherwise a join for a nonexistent mailbox at a real mail domain. Then read the function log, where each handled event logs one fixed label naming only its attribution route (`tag`, `from` or `unattributed`) | each arrives and writes a suppression row; **the log shows which route attributed it**, and if it is `unattributed` S14 fails and the preview waits; an event for the other project's sender is dropped. If Resend's payload carries neither tags nor `from`, §4.6 has no route and the plan returns to Quinn before any list mail |

5. The test data is deleted. Then, with S1-S14 all passed (Q8), `deploy-site` publishes the preview and
   Quinn shares the link.

**The lab disclaimer.** "Nothing you type is stored" stops being true once the form posts to staging. On
the preview it becomes a notice above the form:
> Preview. This is a test version of Knowlu's site. If you join, your address goes to a test list that
> is deleted before Knowlu's real list opens, and you'll be asked to join again.

On production the notice is gone, and the consent line beside the button (§10.2) says what is kept and
links §9's section.

### 14.2 Production, on Quinn's word, in this order

| Step | What | Gate |
|---|---|---|
| **P0** | **Preconditions**: the exposed-key roll done (`HANDOFF.md:566`), so no commercial mail starts on a key pasted into a chat (r1 M13); the list's Resend account **on its paid plan** (Q6, Quinn, 2026-11-09), with production's own sender addresses and a webhook to production's `waitlist-events`; the PO box | Quinn's |
| P1 | W5 merged, and `deploy-site`'s production dry run (S12) passed. **The `v*` tag ruleset is set** (r2 m2) and the `pages` environment's reviewer has "Prevent self-review" off (r2 m1) | Quinn moves the Cloudflare secrets into `pages` (U11) and sets the ruleset |
| P2 | **The privacy release.** W6 (the §9 page and the new `PRIVACY_VERSION`) ships in a release, through draft-first, with Quinn's approval at the `site` job. Every account gets the email and the re-consent panel | Q2 (decided); the reviewer's and Quinn's read of §9 |
| P3 | The production migration, the functions and the secrets: `WAITLIST_ORIGINS=https://knowlu.com`, no preview, budgets 40 and 100 (or as S10 found), no updates | |
| P4 | W7 (the form) merged, which waits for P3, then a site-only `deploy-site` to production | Quinn's approval in the `pages` environment |
| P5 | S1, S2, S3, S8 and S10's sum on production with a test inbox, which `admin.waitlist_forget(address, suppress => false)` then removes. Staging's preview entries and suppression rows are deleted, and the friends are told to join again | |
| P6 | **The lawyer's read** of the consent line, the confirm mail's restatement, §9's section and the first broadcast; then `WAITLIST_UPDATES=on`; then the first broadcast, with a dry run first | Quinn's word, after the lawyer. No update email before this |

**Not in this rollout: the founding offer.** It needs its amendment (the amount, term and cutoff, still
open), the billing spec and the terms first (§6.1), and it is published by its own change.

## 15. The work, split

The plan turns these into tasks with a fidelity ledger. Both review rounds and Quinn's rulings are done;
the plan's own review also covers Revision 2's fixes, which no spec round re-read. Files are disjoint except where a row says otherwise. Everything a `test-writer`,
`implementer`, `console-ui`, `mechanical` or `docs-keeper` changes goes through `reviewer` before a
push.

| # | Task | Files | Agent | Why that agent |
|---|---|---|---|---|
| W1 | The failing tests: §13.1, §13.2, §13.3 and §13.4's static pins for the new objects (RLS, revokes, definers, the sweep's minute) | `cloud/supabase/functions/**/*_test.ts` (new), `cloud/supabase/migrations/migrations_test.ts` (new test functions only) | `test-writer` | TDD. No test it writes reads `site/`. It does **not** edit the existing corpus pins (r2 m9) |
| W2a | The migration: four tables, the definer functions, `admin` objects, the sweep job. **With it, in the same pull request, the pins a new migration must change** (`migrations_test.ts:584`, `:1988`, `:3336-3348`, `:3509-3520`, `:3642-3649`, `:347`, `:368`, re-read at W2a's base), because they pass only once the migration exists (r2 m9) | `cloud/supabase/migrations/<new>.sql`, `cloud/supabase/migrations/migrations_test.ts` (those pins only) | `cloud-engineer` | Personal data and the posture guards; Opus at high. It touches `migrations_test.ts` after W1 has merged, in different functions |
| W2b | Join, confirm and leave: the three functions, `_shared/waitlist_{db,terms,mail,address}.ts`, three `config.toml` entries | `functions/waitlist-{join,confirm,leave}/`, `functions/_shared/waitlist_*.ts`, `config.toml` | `cloud-engineer` | After W2a. The no-log wrapper and the answer-before-mail rule live here |
| W2c | Send and events: the two functions, the broadcasts folder, two `config.toml` entries | `functions/waitlist-{send,events}/`, `config.toml` | `cloud-engineer` | After W2b. **It shares `config.toml` with W2b, so the two run in sequence** |
| W3 | The purge's one call, test first | `functions/account/{index,handler}.ts` and their test | `test-writer`, then `cloud-engineer` | It touches Delete my data. The handler holds the address (`handler.ts:96`); W3 passes `lower(address)` into `purge` or a new dependency beside it |
| W4 | Behaviour: pgTAP rows if the RLS suite has merged, otherwise `proofs/waitlist_staging.sql` (§13.4) | the suite's files, or `cloud/supabase/proofs/` | `cloud-engineer` | The suite is that lane's design |
| W5 | **The deploy.** The failing `workflows.rs` changes and pins (§11.6) and the synthetic-tree tests first. Then `site.yml`, `scripts/ci/assemble-site.ps1`, and `release.yml`'s four jobs. zizmor and actionlint clean | `engine/tests/workflows.rs`, `.github/workflows/{site,release}.yml`, `scripts/ci/assemble-site.ps1` | `test-writer`, then `implementer` | Fully specified and checked by tests. **`reviewer` and `contract-reviewer` must both read it**, because the updater of every installed copy depends on this diff (r1 M14). Its first step proves U19 with a dry run; U17 is proved by the first release after it (§11.3 step 3). It includes Revision 2's `drafts` job, idempotent `publish`, `site-source.txt` and the ancestor rule |
| W6 | The privacy page (§9), its pins first, and `PRIVACY_VERSION` under Q2's bump (decided) | `site/privacy.html`, `engine/tests/site.rs`, `app/src/account.rs` | `test-writer`, then `console-ui` for the page; `contract-engineer` for `account.rs`, read by `contract-reviewer` | The page's words are reviewed text; `account.rs` is on the contract list |
| W7 | **The port lane, for its contract only**: the form, `waitlist.js`, `/w/*`, `_redirects`, `_headers`, the DOM test with its `ci.yml` step, and §13.6's W7 tests | `site/index.html`, `site/waitlist.js`, `site/w/`, `site/_redirects`, `site/_headers`, the DOM test's folder outside `site/`, `.github/workflows/ci.yml`, `engine/tests/site.rs` | `console-ui` | **It shares `site.rs` with W6**, as separate test functions merged by `integrator` |
| W8 | The reference docs and HANDOFF: the functions, secrets by name, the sweep, the `admin` functions, `deploy-site`, draft-first, and the environments. Also: that **a privacy-page version change now needs a new consent entry in the registry** (§9.7); **that Quinn's `pages` approval now stands in front of every app release** (r2 m1); **§11.7's recovery and hotfix runbooks** in `docs/reference/app.md`'s release steps (r2 N2, N3); the `v*` tag ruleset; and **the ruling at `HANDOFF.md:563` marked superseded** (Q7, Quinn, 2026-11-09) | `cloud/supabase/README.md`, `docs/reference/app.md`, `HANDOFF.md` | `docs-keeper` | Its lane |
| W9 | Staging, S1-S14, the preview, and production P0-P6 on Quinn's word | none in the repo | the main session | The one holder of sessions, keys and job tokens |

**Order:**
- W1 → W2a → W2b → W2c; W3 and W4 after W2a. W1's failing tests and the W2 tasks that turn them green
  share one branch, so `main` never goes red; the corpus pins W1 leaves alone stay green until W2a moves
  them with the migration.
- **W5 before W7.** W7's merge also waits for P3 (production functions live), and the form stays hidden
  without its config in any case (§10.1). So no release or deploy can ship a form that cannot submit
  (r1 I11).
- W6 after W5 (Q2 is ruled). It ships in P2's release.
- W8 alongside; W9 last.

## 16. Decided: Quinn's rulings of 2026-11-09

Quinn accepted every recommendation the spec put forward. Each ruling below is recorded as decided
(Quinn, 2026-11-09), with where the spec carries it. **One thing stays open**, at the end. Q1 (Gmail
folding) was not asked; §3.2 sets it as a default.

| # | Ruling (Quinn, 2026-11-09) | Carried in |
|---|---|---|
| **W-D5** | **Double opt-in.** A signup counts only after the confirm link | §3.5, §4.2-§4.3, §8 |
| **Q2** | **A privacy version bump**, not a separate notice page. `PRIVACY_VERSION` moves through `contract-engineer` and `contract-reviewer`; every account gets the email and the re-consent panel; production joins open after that release | §9.7, P2, W6 |
| **Q3** | **One first-party script on the index, `waitlist.js`, and the download button off the index.** The privacy page stays script-free; `site.rs:29` becomes a check that `releases/Knowlu-setup.exe` is still served | §9.6, §10.1, §10.6, §13.6 |
| **Q4** | **No offer anywhere until an amendment signs it.** Already settled for that amendment: **one discount, shared with quiet-release buyers**; **a Pell student pays the lower price, never both**; **earlier joiners qualify**; **the claim expires when the list closes** | §6.1-§6.3, §10.2, §13.6 |
| **Q5** | **"Move up" orders invitations to the spring 2027 release**: a week earlier per confirmed friend, up to five weeks. No position number or status page in v1 | §5.4-§5.5, §9.2 |
| **Q6** | **A separate Resend account for the list, on a paid plan, with its own sending subdomain, before any list mail, staging's included** | §4.8, §7.6, §14.1, P0 |
| **Q7** | **The shared draft-first deploy**, with Quinn's approval on every release (the `pages` environment's required reviewer) and a `v*` tag ruleset. **The 2026-10-20 ruling against a site-only deploy (`HANDOFF.md:563`) is superseded** | §11, §12, P1, W5, W8 |
| **Q8** | **The friends' preview opens only after the PO box exists and S1-S14 pass**: unlisted, noindex, at `knowlu-preview.pages.dev`, against staging. Its entries are deleted before production, and friends join again | §14.1 |
| **Q9** | **When the lawyer reads**: production joins open once the PO box, the list's own sending and P2's privacy release are live. The lawyer reads the consent line, the confirm mail's restatement, §9's section and the first broadcast before any update email (P6), and the offer's words before anyone is charged | §8, §14.2 |
| **Q10 and Q11, merged** | **The suppression fingerprint (a keyed HMAC) and its consent facts are kept 3 years by default, and the lawyer sets the final period.** Lifting stays the person's own act | §3.6, §3.8, §8 items 8-9, §9.2, §9.4 |
| **The PO box** | **A USPS PO box that Quinn will supply.** No list mail is sent without it | W-D4, §4.1, §7.3, §14 |

**Still open: the founding offer's amount, term and cutoff.** They belong to the offer's own amendment
to the cloud design, which answers Amendment 2026-10-16b's open discounted price. A pricing analysis is
running. Nothing in this spec waits on it: the registry's offers list stays empty, no page shows an offer,
and §14's rollout publishes none (§6.1). §6.3's arithmetic uses the prototype's $6.99 only as an example.

## 17. Unverified: where the reviewer should start

Each of these is something the design leans on that was not confirmed in code or a document. The plan
settles each before the task that depends on it.

| # | Claim | Leaned on in | How to settle it |
|---|---|---|---|
| U1 | *Settled by r1*: the five commits after `bfea03be` changed only what §2 now cites (`all_workflows()`, `security.yml`, `release.yml`'s lines, the definer guard and its pins) | — | — |
| U2 | Which request header carries the real client address on Supabase's edge, and whether `x-forwarded-for`'s first hop can be forged | §3.7, §4.2 | S5 |
| U3 | A preflight `OPTIONS` and a `POST` without `apikey` reach a `verify_jwt = false` function | §4.1, §10.1 | S2. If the gateway wants `apikey`, the site sends the anon key and CORS allows that header |
| U4 | Edge functions can resolve MX and A records | §4.2 | A staging call. If not, the DNS check is dropped and the syntax check stands alone |
| U5 | **Resend:** that DKIM covers custom headers; the idempotency window; tracking off per domain; **whether webhook payloads carry tags, and the sender's `from`** (§4.6's two attribution routes); the webhook signature scheme; the API rate limit (believed 2 a second); the paid plan's daily quota and price; test addresses for complaints and bounces. Whether suppression is account-wide no longer decides anything: Q6 ruled a separate account regardless | §4.5, §4.6, §7.3-§7.6 | Resend's documentation, then S11, S13 and S14. S14 runs whatever the documentation says (r2 N5) |
| U6 | *Settled*: `EMAIL_FROM` is `Knowlu <hello@knowlu.com>` on both projects (`HANDOFF.md:52`). The list no longer uses it | — | — |
| U7 | RFC 8058's exact wording on `GET`, and on what the one-click `POST` must answer | §4.4 | Read the RFC |
| U8 | School mail systems' link scanners open links | §4.3 | Settled by design either way: no `GET` changes anything |
| U9 | **Cloudflare Pages:** `_redirects` placeholders and splats; a nested `w/confirm.html` served at `/w/confirm` (the top-level `/privacy` is, `HANDOFF.md:50`); the check branch's alias URL, and whether it carries `X-Robots-Tag: noindex` by default (§11.3, r2 m3); the per-file size limit; the rollback route; creating the second project | §5.2, §10.3, §11 | Pages' documentation, then S7 and S12 |
| U10 | GitHub's "latest release" semantics. **Only site-only mode uses it** (§11.4); release mode reads the run's artifact | §11.4 | GitHub's documentation, or the script takes the highest `v*` tag with a published release |
| U11 | Where `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` live today (the repository, or the `release` environment) | §11.5 | The repository's settings. Quinn moves them into `pages` at P1 either way |
| U12 | *Settled by r1*: `v0.1.5` carries `PRIVACY_VERSION` 2026-10-20 and `TOS_VERSION` 2026-09-10, equal to the pages | — | S12 confirms `v0.1.5` is the latest published release |
| U13 | Whether Tauri's updater follows a redirect | §11.2 (d), only to reject it | Not needed |
| U14 | The hosts' own log retention (Supabase, Cloudflare) and how long Resend keeps a copy of each message | §3.6, §9.2-§9.3 | Their documentation. The page names no length until then |
| U15 | The legal points in §8 | §7.3, §8 | The lawyer's read (P6) |
| U16 | *Settled by r1*: the account handler holds the address (`account/handler.ts:96`) | — | — |
| U17 | **Revised (r2 N1).** Release mode no longer reads a draft. What it leans on now: that a job in a called workflow can download its caller run's artifact with `actions/download-artifact` (documented by GitHub). Separately, that the `drafts` job's `contents: write` token lists drafts, which r2 states GitHub requires | §11.3 step 3, §11.4 step 2 | The first release after W5 for the artifact; S12 for the drafts list |
| U18 | Whether the SQL editor keeps a copy of a query after it is deleted from the saved history | §3.6 | Supabase's documentation |
| U19 | That a job inside a called workflow can declare an environment and read its secrets. **GitHub documents it** (r2 m10) | §11.3, §11.5 | W5's dry run stays the proof, before `release.yml` changes |
| U20 | The edge function wall-clock limit | §4.5 | Supabase's documentation; `limit` comes down if needed |
| U21 | Whether a Cloudflare API token can be scoped to one Pages project. **Believed not: Pages permissions are account-wide** (r2 m11), and §11.5 is planned as if the preview token can deploy production | §11.5 | Nothing to settle before the plan; the design holds through the branch policy and the pin |
| U22 | CASL's limitation period, the basis of the 3-year default | §3.8, §16 (Q10 and Q11) | The lawyer, who sets the final period (§8 item 8) |
| U23 | **New (r2 N2).** That `gh release view <tag>` (or the equivalent `gh api` list) finds a **draft** by its tag name, and that `gh release upload <tag> … --clobber` replaces a draft's assets. `publish`'s idempotency rests on both | §11.3 (`publish`), §11.7 | `gh`'s documentation, then the first `publish` rerun; until then, the runbook's "delete the draft and keep the tag" is the fallback |
| U24 | **New (r2 m2).** That a tag ruleset restricting `v*` tags is available to this repository on its plan | §11.5, P1 | GitHub's settings at P1; if not, Quinn decides before P1 whether the branch policy alone is acceptable |
| U25 | **New (r2 N1).** That a draft release naming a tag that does not yet exist creates no git tag until it is published, so S12's test draft starts no `release.yml` run | §14.1 S12 | GitHub's documentation, before S12 |
