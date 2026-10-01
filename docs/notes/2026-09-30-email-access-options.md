# Email access beyond the Gmail API: research, experiments and a direction

**Date:** 2026-09-29 → 2026-09-30. **Status:** brainstorm and spikes finished; input to a spec, not a
decision of record. Quinn will write the spec in the main session. **Stage:** not yet ruled (the
recommendation is MVP, §7).

**Relation to signed work:** `docs/specs/2026-09-29-gmail-connect-design.md` (signed) stands. This note
adds paths it does not cover: school Microsoft 365 mail, mail from before setup, and Gmail at scale
before CASA. It develops the "forwarding" idea in `2026-09-29-funding-and-seed-grant.md` §6 and the
forwarding fallback in the cloud design's §13.

## 0. The problem

Email is one of Knowlu's strongest draws. The signed path is Gmail's API (`gmail.readonly`), which is a
Google *restricted* scope: restricted-scope verification plus an annual CASA assessment, and until then
Testing mode (at most 100 named testers, refresh tokens that expire every 7 days). Students also have
school accounts, Outlook.com, Yahoo and others, and a student who joins mid-semester needs the mail
that arrived before they signed up.

**The biggest finding:** all three pilot schools the repo names (UA, UK, UofL) run student mail on
**Microsoft 365**, not Google. UA moved Crimson from Google to M365 in May 2021
([UA news](https://news.ua.edu/2021/04/student-emails-moving-to-microsoft-get-the-lowdown/)), so the
legal-landscape note's "Crimson mail is Google Workspace" (`2026-09-09-knowlu-cloud-legal-landscape.md`,
§10) is stale. CASA, however it lands, reaches personal Gmail only, never pilot students' school mail.

## 1. Research summary (three read-only reports, 2026-09-29)

**Provider rules (primary sources):**
- **Google.** `gmail.readonly` and `https://mail.google.com/` (IMAP OAuth) are both restricted. Google's
  text: "Every app that requests access to Google users' restricted data and has the ability to access
  data from or through a third-party server must go through a security assessment." There is no
  on-device exemption that Knowlu could use, because Knowlu judges in the cloud.
  - **App passwords** need 2-Step Verification and are unavailable for school/work accounts. Google
    discourages them, and a password change revokes them.
  - **Student-owned Apps Script** shows the unverified-app screen, and admins can disable it.
  - **Forwarding**: the destination confirms by link. Workspace admins can turn it off.
- **Microsoft.** Graph `Mail.Read` (delegated) has no CASA equivalent. Publisher verification is free
  but needs a Partner Program account. Basic-auth IMAP is gone in Exchange Online and unreliable for
  Outlook.com, so OAuth is the only direct route. Tenants can require admin consent. Since 2021, new
  tenants default external auto-forwarding to Off.
- **Yahoo / iCloud.** Both have app passwords with IMAP. Yahoo OAuth needs Yahoo's approval, and free
  Yahoo accounts lost auto-forwarding.
- **Power Automate.** The HTTP step is premium. A dead end for students.

**Competitors:** no student product found reads email. Student planners pull from the LMS or rely on
manual entry.
- Indie and task apps use a **private forwarding address**: Todoist, Things, OmniFocus Mail Drop,
  Motion, TripIt, Expensify, Sense.
- Funded AI-inbox products use OAuth and state CASA or an equivalent review: Sunsama, Fyxer,
  Shortwave.
- TripIt's ladder, "forward first, then offer automatic sync", is the pattern closest to ours.
- CASA Tier 2 quotes run about $540–1,800 a year (vendor and blog figures, secondhand).

**Unified mail APIs** (Nylas, Unipile, Aurinko) hold tokens and mail on their servers and cost $175+ a
month at 100 accounts. Only Unipile claims to lend its CASA. Not recommended now.

**Inbound mail services** for a forwarding address are cheap. CloudMailin has a free tier of 10K
messages a month; Postmark inbound comes with Pro at about $16.50 a month.

## 2. Experiments (founder's own accounts, throwaway scripts, 2026-09-30)

Harness: a bare `wry 0.55` / `tao 0.35` window (Knowlu's own WebView2 stack) with a throwaway profile
and CDP enabled. Playwright drove it with `connect_over_cdp`, **DOM only, never OS input**. IMAP ran on
the device with an app password read from Credential Manager. The scripts are preserved outside the
repo (§8).

| # | Test | Result |
|---|---|---|
| E1 | Crimson → Graph `Mail.Read` (multi-tenant app, unverified), loopback PKCE | ❌ **"Need admin approval."** Device-code sign-in is separately blocked by UA Conditional Access. The 08-11 spec records a verified publisher's M365 connector blocked the same way, so publisher verification won't unlock UA. |
| E2 | Crimson → external mailbox forwarding (existing) | ✅ A transparent redirect: original From, To, Date and Message-ID. SPF, DKIM (ua.edu), DMARC and ARC (Microsoft) pass at Gmail. `X-MS-Exchange-ForwardingLoop: <mailbox>;<tenant>` names the forwarding mailbox. |
| E3 | Gmail "add forwarding address" confirmation | ✅ Google's `vf-…` link leads to a page with one POST form: no token, no login. A **cookie-less POST from our side completes it** ("Confirmation Success!"). Re-run later: found and completed within 1 s of arrival. |
| E4 | Gmail filter import with a `forwardTo` action | ✅ The forward action survives import, so Knowlu can hand out a ready-made filter. It works only once the address is confirmed. |
| E5 | Gmail IMAP with an app password | ✅ Login 0.7 s; `X-GM-RAW` search; 20 headers and a full message in 3.9 s. Proven, but students find it fussy. |
| X2 | Sign-in inside the Knowlu-controlled WebView2 window | ✅ **Microsoft/UA** (login.microsoftonline → UA Okta → Duo/Okta Verify) and ✅ **Google** (password + phone prompt). No "browser may not be secure" block; `navigator.webdriver` is false. The page title ("Mail - … - Outlook", "Inbox … - Gmail") is a clean logged-in signal. |
| X3a | Script sets Crimson forwarding (Settings → Mail → Forwarding) | ✅ Saved with **no re-authentication**. A test message arrived at the new destination in about 40 s. Reverted afterwards. |
| X3b | Script adds a Gmail forwarding address | ✅ with **one human step**: Gmail asks "verify it's you" in a Google **popup** (the host must allow popups; wry `with_new_window_req_handler → Allow`). The phone tap is the student's. Then E3's server-side confirm finishes it. Removing an address needs no re-auth. |
| B-C Gmail | Backfill: bulk "Forward as attachment" | ✅ Select search results → More → Forward as attachment. Each `.eml` is the **complete original**, including the original **DKIM-Signature** (re-verifiable). 5 messages ≈ 1 MB, so about 100 per 25 MB send. |
| B-C Crimson | Backfill: forward as attachment in new Outlook web | ⚠️ **Per message only.** Forward is disabled for a multi-conversation selection. One message: Forward ▾ → Forward as attachment → a complete original in about 1 s. The outer message passes DKIM for the school domain, which binds the batch to the student. |
| B-D | Backfill: read old mail from the page | ✅ but fragile. About 3 s per message, and the body text is readable. No original MIME, so no DKIM and no Message-ID. Scraper-like. |

**Volume (for sizing):**
- Personal Gmail since 2026-08-17: 959 messages; 680 without promotions/social; 388 from UA,
  Blackboard or Canvas.
- Crimson, 08-17→08-31: 75 conversations (`aria-setsize` of the search results).

## 3. Forwarding loses mail: the investigation

The founder's Crimson → Gmail forward is mailbox-level: "Enable forwarding" plus "Keep a copy". An old
inbox rule also exists, but a rule's scope is not the cause.

**The losses:**
- On 2026-09-30 Crimson received **8 Blackboard emails, and 1 reached Gmail.**
- Earlier, 12 Blackboard emails between 09-23 and 09-28 arrived.
- Losses come in bursts, pairs a few seconds apart.

**Header comparison** (Crimson copies fetched by forward-as-attachment):
- One lost email (09-28 20:32) and one delivered email (09-28 20:35) come from the same Amazon SES
  sender for blackboard.com.
- Microsoft's inbound stamps are identical on both: `SFV:SKA` (allow-listed), SCL -1, BCL 3,
  SPF/DKIM/DMARC pass, no "[EXTERNAL]" tag, and both marked `ForwardingHandled`.
- The lost one is nowhere in Gmail: not in All Mail, Trash or Spam.

**What is ruled out, and what is left:**
- **Ruled out:** a Microsoft inbound spam verdict, and the DKIM-broken-by-tagging hypothesis.
- **Left:** loss on Microsoft's outbound forward or at Gmail's door. SRS sends any bounce to the
  original sender, so the student never sees it. Only an admin message trace can name the cause.
  Rate limiting on bursts is a guess.

**Also found:** Gmail silently drops a forwarded copy whose Message-ID it already holds. A student's
own sent mail coming back is one example.

## 4. Direction: five layers that make sure mail arrives

1. **Forward to our own receiver (the Knowlu address).**
   - One private address per student, tagged per account (`<secret>+crimson@…`).
   - Knowlu sets up forwarding in its sign-in window: automatic on M365, one phone tap on Gmail.
   - The receiver **never rejects on SPF/DKIM/DMARC** during delivery. It stores the raw MIME and the
     `Authentication-Results`, `ARC-*` and `Received` headers, and judges afterwards.
   - Removing Gmail as the last hop removes the likeliest place the Blackboard mail died.
2. **Backfill at setup.** In the same session, forward everything since the semester started as
   attachments: bulk on Gmail, per message on M365. Originals take the same intake path as live mail.
3. **Canary.** At setup and then periodically, Knowlu sends a test email to each connected address and
   expects it back at email forwarding to Knowlu. A missing canary shows "this source went quiet".
4. **Reconcile.** An **opt-in saved session** (kept WebView2 profile, on the device only) lists what
   the mailbox received (sender, date, Message-ID) and compares it with what email forwarding to Knowlu got.
5. **Recover.** The saved session forwards the missing emails as attachments, and re-enables
   forwarding if it was turned off. An expired session asks the student to sign in only when a repair
   is needed.

**On arrival:** a message counts only if it reached that student's secret address **and** passes
DKIM/DMARC for its sender, or a Microsoft/Google ARC seal. Backfilled originals keep their own DKIM.
Microsoft's `ForwardingLoop` header and the address tag bind a message to an account.

**Direct paths where they exist:**
- Gmail OAuth (the signed spec) for testers, and for everyone after CASA. CASA becomes optional, not
  blocking.
- Graph for any school that allows user consent.
- IMAP with an app password in reserve.

**Server path:** an inbound mail service → webhook → the existing judge → queue → approval-card path
that `gmail-read` feeds. Server handling stays transient, as the Gmail design already promises.

## 5. The saved session: benefits and costs

**Benefits:** repair without asking, long backfills across sessions, recovering specific missing
emails. It could even replace forwarding by reading on each slot, but that is **not recommended**: the
most brittle option, and closest to what Microsoft's and Google's terms discourage.

**Costs:**
- A saved session is full mailbox access. Windows encrypts it to the user's account (DPAPI), but
  malware running as the user can still use it. The privacy page must say so.
- Sessions expire unpredictably: Conditional Access, Duo/Okta re-prompts, Google's risk checks and
  device-bound cookies.
- Precedent: the 08-11 design kept a never-cleared browser profile for the myBama/Blackboard session
  (`2026-08-11-personal-ops-system-design.md:263`).

**Recommendation:** opt-in, device-only, a dedicated profile, used for repair and backfill only,
deleted on Disconnect.

**Running measurement:** a scheduled task checks a kept profile twice a day until 2026-10-08 (§8).
Result pending: how long each session survives with twice-daily use.

## 6. What an implementation must handle (from the spikes)

- **Sign-in window:** allow popups (Google's verify-it's-you), suppress the "open email links"
  protocol-handler prompt that Outlook raises, and cover the page with a checklist during scripted
  steps rather than hiding it.
- **Discard sessions** when setup ends, unless the student opted into the saved session.
- **Outlook:** the forwarding switch is `input[role=switch]`, so read `checked`, not `aria-checked`. A
  first script run flipped it unsaved; it was discarded.
- **Gmail:** the buttons are `<input type=button value=…>`, and the settings DOM sometimes needs a
  hash round-trip to render.
- **Driver:** a Playwright `reload()` over CDP detaches from the WebView2 target. Use hash or `goto`
  navigation.
- **Both UIs change without notice.** Every scripted step needs a fallback to guided manual
  instructions.
- **Crimson backfill:** per-message sends of about 30 a minute leave copies in Sent Items (the script
  can delete them). Untested: whether turning conversation view off enables multi-select forward.
- **Receiver provider:** before choosing, test each candidate with a deliberately DMARC-failing
  message. Postmark's spam threshold must be off. The others' handling is unverified.
- **Privacy page:**
  - email forwarding to Knowlu (all forwarded mail passes through us, is judged and discarded);
  - the canary emails;
  - the saved session.

## 7. Open decisions for the spec

1. **Stage.** Recommended: **MVP**. It is the only way to read Crimson, and backfill is what makes a
   mid-semester sign-up useful.
2. **Receiver** (CloudMailin, Postmark, SES or other) and the address domain.
3. **Forward everything, or a student-chosen filter?** Everything gives coverage; a filter gives
   privacy. The server-side cheap screen discards noise either way.
4. **Canary cadence**, and whether reconciliation runs on every slot or daily.
5. **Saved session:** default off, with a clear opt-in; what happens on expiry.
6. **Whether to ask UA IT for a message trace** on one lost Blackboard email. This is the only way to
   name the loss cause, and it is optional because the design does not depend on the answer.
7. The **legal note's stale Crimson line** (§0) needs a correction.

## 8. Artifacts and state outside the repo

- **Spike folder:** `%LOCALAPPDATA%\knowlu-email-spike\`, holding:
  - `FINDINGS-2026-09-30.md` (the raw lab notes);
  - `scripts\` (the wry host source and every experiment script);
  - `check.py` and `profile\`, the latter holding **live Crimson and Gmail sessions**;
  - `sessions.log`.
- **Scheduled task `KnowluEmailSessionSpike`:** every 12 h from 09:00, ending 2026-10-08. After that,
  read `sessions.log`, then delete the task and the whole folder.
- **Account state after the spikes:** Crimson → Gmail forwarding restored as it was; Gmail has no
  forwarding address.
- **Still to clean up on the founder's side:**
  - revoke the spike app password, and remove its Credential Manager entry `knowlu/dev/gmail-imap`;
  - delete the spike Gmail filter, if one was created;
  - optionally delete the Entra app `knowlu-mail-spike`;
  - optionally delete the "knowlu-spike…" test messages in both mailboxes.
