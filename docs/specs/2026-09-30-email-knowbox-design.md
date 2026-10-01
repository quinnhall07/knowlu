# Email (MVP): the Knowbox — automatic forwarding setup, receiver, claims and reconcile

**Date:** 2026-09-30. **Status: DRAFT for Quinn's signature, revised after review and after Quinn's
answers to §13** (*Review revisions (2026-09-30)* lists every finding and what changed; the *Signing
sheet* at the end lists what signing commits to). Nothing here is built. Quinn's answers of 2026-09-30
to the seven questions of the research (stage, automatic, receiver, filter, backfill and change
tracking, surfacing, saved session, school IT) are binding on this spec and are cited as **Q1–Q7** (Q5b
for surfacing). Quinn's answers of the same day to the eleven questions §13 raised are binding too and
are cited as **§13 Q1–Q11**; §13 keeps each question beside its answer. §10 is the amendment text Quinn
signs with it, including **A13**: ruling 12's university-policy-read gate suspended, for the kept
Blackboard session and the saved mail sign-in alike, until Knowlu has 20 paying users.

**Authority.** `docs/specs/2026-09-09-knowlu-cloud-design.md` and its amendments; this spec amends D12,
§13 and ruling 10 (§10). Then `VISION.md`, `CLAUDE.md`'s two overriding rules, and the signed Gmail
connect spec (`docs/specs/2026-09-29-gmail-connect-design.md`), which stays the direct path (D1). If this
spec disagrees with the cloud design after signing, the cloud design as amended here wins.

**Read with:**
- `docs/notes/2026-09-30-email-access-options.md` (the spikes E1–E5, X2, X3a/b, B-C, B-D; the loss
  investigation §3). Every spike fact cited here comes from it.
- The research workflow's synthesis of 2026-09-30 (four read-only reports and a synthesis). Its stage
  split, its "saved session off" and its "Gmail filter" recommendations were overruled by Q1, Q6 and Q4.
- `docs/specs/2026-09-29-grades-design.md` (the kept-session pattern) and the cloud design's
  Amendment 2026-09-29, ruling 12 (the one named exception to "keep nothing" so far).

**Base.** `main` at `199cd1f` (M1 grades #25 and Gmail connect #26 both merged). §1 is verified at
that commit; §9 says what this spec takes from the merged Gmail lane.

**Rule 1.** Nothing below names a student's vault, machine or account. The founder's mailboxes appear
only as the spikes' evidence; the live proof (§12.6) runs on founder-owned *test* accounts.

---

## 0. The student-facing goal

A student opens Knowlu and asks **what's next?** Much of the honest answer arrives by email: a
professor moves a due date, an exam room changes, a Blackboard notice says a quiz opens tonight, a
club needs a sign-up by Friday. At the three pilot schools the repo names, student mail is school
Microsoft 365 (research note §0), which Knowlu cannot read today: Graph needs a school admin's
approval (E1), and Gmail's API covers only personal Gmail, for at most 100 testers.

After this spec:

- **Right after sign-in, Knowlu says email comes next.** The wizard's panel after sign-in reads "Next:
  connect your school email", and when Knowlu's main window first opens, the school mail sign-in opens
  with it (§13 Q9). The student signs in to their school mail, and to their personal Gmail if they
  want it, in a Knowlu window. That window is only for signing in.
  Once they are signed in it gets out of the way, and Knowlu's own window shows each step as it
  happens: forwarding turned on, a test message received, mail since the term started brought in.
  The window comes back only if the mail site needs the student (Google's "verify it's you", an MFA
  prompt). If a step cannot be done automatically, Knowlu shows the student how to do it by hand.
- **The first page already knows the term.** Mail since the term started is read once. Assignments
  announced by email, due-date moves, schedule and room changes and professor information are applied
  to the plan, and each is listed under "From your email" with an Undo. Anything that conflicts with
  what the student set, or with the LMS, becomes a card instead. A typical school mailbox (a few
  hundred messages since the term began) is brought in within about an hour of the app being open;
  a heavier one takes longer, at a pace Microsoft's sending limits allow (§4.2 step 5), and the
  progress view says how much is left. Nothing is applied until the whole batch has arrived (§4.8).
- **From then on, new mail is proposed.** A new obligation or a change found in a new email is a card
  in the deck, like every other thing Knowlu noticed.
- **Silence is never ambiguous.** Knowlu checks that mail still arrives. When a mailbox stops
  forwarding (the school turned it off, the student changed a setting) the app says "this source went
  quiet" and, with the student's saved sign-in, repairs it and brings in what was missed.

This answers VISION's test on all three counts. The answer is **more complete** (obligations that live
only in mail reach the list, including those from before setup), **clearer** (a moved deadline moves in
the plan), and **more honest** (a broken source says so).

## 1. What is on main today (verified 2026-09-30, at `199cd1f`)

| Claim | On main |
|---|---|
| Email ingestion | Only Gmail by OAuth, server-side: `gmail-read` lists 7 days (`gmail-read/handler.ts:34`), reads at most 60 per call (`:36`), judges each through the one pipeline with origin `gmail_api`, queues the verdict in `gmail_queue` and discards the text (`:23-28`). Nothing receives forwarded mail. A `quiet: true` answer is read by the device as an error before any item is read, and every reason it does not know reads as `Revoked`, "re-connect from settings" (`engine/src/cloudmodel.rs:667-675`). |
| The forwarding placeholder | `judge-email/index.ts:3-5` names "§13's forwarding fallback if it is ever built" as a caller with origin `device`. The cloud design's §13 (`:381`) documents a Cloudflare Email Routing inbox as that fallback, "not built". |
| Email verdicts | Six tiers (`judge_validate.ts:30`): `task`, `borderline`, `event`, `opportunity`, `information`, `completion`. A verdict below `CONFIDENCE_FLOOR` 0.6 is not a verdict (`:6`, `:156`). |
| Rules before the model | `lms_receipts.ts` recognises a templated Blackboard submission receipt with no model call, data-driven by `RECEIPT_TEMPLATES` (`:34-47`); `gmail-read` runs it before the pipeline (`handler.ts:314-337`). |
| The due resolver | `_shared/judge_due.ts` (`parseDateLine` `:171`, `resolveDue` `:375`) resolves a relative deadline against the message's Date line and the vault's timezone. |
| Caps and ceiling | `DAILY_CAP` email 120 a day (`judge_caps.ts:39`), doubled on an account's first two UTC judging days (`20260923000100`, comment at `:36-37`); `MONTHLY_CEILING_USD` 2.0 (`:60`). |
| Judgment kinds, everywhere | The three-kind check `kind in ('task','event','email')` pins `models.kind` (`20260911000100_judgment_service.sql:39`), `judgments.kind` (`:70`), `corrections.judgment_kind` (`:102-103`), `rules.kind` (`:113`), `usage_daily.kind` (`:155`) and both eval tables (`20260911000700_eval.sql:9`, `:31`); `telemetry/handler.ts:55` keeps its own list. `judgment_features` emits features for any kind (`20260911000500_rule_promotion_fix.sql:72-90`), and `promote_rules` and `backfill_correction_judgments` read every judgment. |
| Judgment origins | `judgments.origin in ('device','gmail_api','events')` (`20260911000100_judgment_service.sql:82`); the training export excludes `gmail_api` and `events` (`20260916000100_provider_swap.sql:61`), pinned by `migrations_test.ts:826`, which checks for the substring `origin not in ('gmail_api', 'events')`. |
| The device's Gmail pass | `enrich::pull_gmail` (`engine/src/enrich.rs:538`), actor `GMAIL_ACTOR`, `agent:knowlu.gmail` (`:512`), up to `PULL_ROUNDS` 3 rounds a slot (`:516`), dedup by `state/ingest-seen.md`. The `task` tier files a card through `write_gmail_card` (`:648`, `:879`); a Gmail note (`GMAIL_NOTE`, `:526-529`) is written `needs_enrichment: false`, `judgment_kind: email`, so the device's task pass never judges Gmail text a second time (`:518-525`). Labels are reported only for `task` and `event` cards (`labels_to_report`, `:1140-1147`). |
| Judge once | `write::write_literals` applies judge-once **only** when `opts.judged`, the actor is an agent, and the field is in `judged_fields_for(kind)` (`write.rs:298`). For a task that set is `JUDGED_FIELDS_TASK`: `effort_hours`, `effort_confidence`, `importance`, `importance_reason`, `course`, `domain` (`provenance.rs:52-59`). **`due` and `title` are not in it.** Under `propose` it files a card through `write::propose_amendment` (`:736`), whose body says "re-judged … judge-once rule" and which carries no sender or quote. `Journal::human_set` counts a human `set` or a human `create` carrying the field (`journal.rs:394-405`); `human_edited` counts only a `set` (`:413-419`). |
| Who executes an approved card | `executor_ctx()` is `agent:approvals` (`app/src/commands.rs:177-180`); `apply_amendment` writes with `WriteOpts::default()` (`approvals.rs:697`, `:792`). `status` and `due` are in `AMENDABLE_FIELDS` (`:43-54`). |
| The LMS feed overwrites | `ingest` writes as `agent:ingest.blackboard` (`engine/src/ingest.rs:626`), compares each active note's `title` and `due` to the feed's and writes when they differ (`:676-696`) with `WriteOpts::default()`, so **judge-once never applies**: a `due` the student set by hand is overwritten by the next ingest exactly as an agent's is. The cloud design's Amendment 2026-09-29, ruling 3 (`cloud-design:566-571`) already says no source overwrites a field the student set by hand (`human_edited`); `ingest` predates it and does not yet honour it. |
| The slot's app steps | The grades capture runs **in the app**, after the entitlement step and before the child-process loop (`app/src/scheduler.rs:845-858`); the loop starts with `sync` (`:922`). `slot_argv` builds only engine argv (`:474-516`), and the engine's `grades` step consumes the bundle the capture wrote. |
| Imported past | `ingest::IMPORTED_PAST` (`engine/src/ingest.rs:536`): R-OB-3's archive reason for an item already past due on first import. |
| Info items | `engine/src/info.rs`: `open_info` writes `info/<id>.md` (`:69-105`) with `kind` in `package`, `notice`, `heads-up`, `other` (`:27`) and an optional `close_key`; closing by key closes every open item with it. |
| Undo | No undo command exists. `set_fields` and `delete_note` exist (`commands.rs:469`, `:471`). |
| The kept-session pattern | `app/src/grades.rs`: one WebView2 profile at `<data_dir>\lms-session` (`:239-250`), opened visible for sign-in (`open_visible`, `:549-563`) and hidden for a refresh (`open_hidden`, `:593-607`, `visible(false)`, `skip_taskbar(true)`), deleted by `forget` with a path guard (`:254-279`); the window has no capability grant (`:54-57`); one capture at a time (`Capturing`, `:462-477`). |
| Account deletion, locally | `account::delete_local_data` removes the whole profile data folder (`account.rs:1232-1267`), so anything kept under it goes too. `PRIVACY_VERSION` is `2026-09-24` (`:25`), moved only with the page (`:21-23`). |
| The slot | `sync → coursework → ingest → grades → judge → rank` (`app/src/scheduler.rs:464-516`); a step that cannot run is a named `(skipped: …)` at exit 0 (`:427`, `:901`). |
| First-day rules | The only rule keyed to the vault's first day is the commitment model's: no commitment proposal card is filed on vault day 1 (`engine/src/cli.rs:850-852`, `commitments::vault_day`, `commitments.rs:1483`). Separately, the service doubles an account's daily judgment caps on its first two UTC judging days (`20260923000100_first_day_cap.sql`). |
| Entitlement on the server | Every entitled function admits `active` and `trialing` only (`_shared/entitlement.ts:22`, `:47`). The 72-hour grace is the device's: how stale a cached good answer may be (`engine/src/entitle.rs:29-31`), not a payment grace. Google connect's status and disconnect routes are gated on sign-in alone (`google-connect/handler.ts:102-109`, `index.ts:14`). |
| Account deletion, server | `account/index.ts:46-82`: the Google grant is revoked first, then every table that holds the student's data is named in a list "whether or not the cascade would also reach it" (`:70`), then the account row. Gmail's disconnect deletes its queue rows "delivered or not" (`20260929000200_gmail_disconnect_purge.sql:10`, `:33`). |
| The term's start | **Nothing records it.** No campus preset (`app/assets/campus/`), no `Curated` field (`app/src/scaffold.rs:26-43`) and no engine config carries a term or semester start date. |
| The school's mail | `Curated` has no mail domain or provider (`scaffold.rs:26-43`); the IPEDS list carries only a web host. |
| Note folders | `ids::NOTE_FOLDERS` is `tasks, approvals, archive, courses, issues, info, commitments, grades` (`engine/src/ids.rs:23-24`); `commitments::LOCAL_CARD_KINDS` is the two commitment card kinds (`commitments.rs:27`). |
| The privacy page | `site/privacy.html:77` says Gmail "is not connected in this version"; `:85` says message text is discarded and only the id, the decision and the card's fields are kept; `:56` names what reaches a model; the sub-processor list is `:62-70` (no AWS). |
| The legal note | `docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md:121` and `:748` say Crimson mail is Google Workspace. It moved to Microsoft 365 in 2021 (research note §0); the line is stale. |

**Findings that shape the design:**

- **F1. Nothing set on an LMS item lasts against the feed, the student's own value included.** `ingest`
  rewrites `title` and `due` from the feed whenever they differ, with `WriteOpts::default()`
  (`ingest.rs:684-696`), so judge-once never applies to it, whoever set the field. A due date moved by a
  professor's email but not in Blackboard is reverted at the next slot, even when the student approved
  it, and even if it was written as the student. Ruling 3 already says a source must not overwrite a
  hand-set field; making `ingest` honour it changes every existing vault's behaviour, so it went to
  Quinn as §13 Q7. Quinn's answer: fix it now (D11 (5), T8b).
- **F5. Judge-once does not cover `due`.** `write`'s judge-once is limited to the six judged task fields
  (`provenance.rs:52-59`); `due` and `title` are not among them. A mail pass that relied on `write`'s
  judged set would overwrite a due date the student typed, with no card. The reconciler therefore
  decides card or direct itself, from the journal (D11).
- **F6. The grades capture is not a step in the engine chain.** It runs in the app before the
  child-process loop (§1). The saved-session mail step runs beside it, not between `grades` and `judge`
  (§4.11).
- **F2. There is no term start anywhere.** Backfill "to the semester start" (Q5) needs a new datum (D7).
- **F3. There is no undo.** Q5b's "undoable" is built here from the two primitives that exist (D10).
- **F4. A separate queue avoids the Gmail Quiet trap.** The research worried that `pull_gmail_queue`
  returning `Err(Quiet)` before reading items (Gmail spec `:967`) would hide Knowbox cards. It would only
  if the Knowbox reused `gmail_queue`; it does not (D8).

## 2. Stage and scope

**Stage: MVP, all of it (Q1).** Ruling 10's MVP list gains it (§10, A5). It is proven on a founder-owned
scratch profile and founder-owned test mailboxes on staging, with no second person (§12.6). No release
carries it before privacy bump #1 (D18).

**In scope:**
- the receiver (the Knowbox): AWS SES inbound → S3 → a Supabase function, one secret address per
  connected mailbox (D2, D3);
- scripted setup for **school Microsoft 365** and **personal Gmail**, with a guided fallback for every
  step and for every other provider (D4–D6, §4.2–§4.4), including Gmail's server-side forwarding confirm
  (E3);
- the progress view in Knowlu's main window (§4.5);
- full backfill to the term's start (D7), through the same intake as live mail;
- claims extraction, cheapest first, and the deterministic reconcile on the device (D9–D12);
- "From your email": what the backfill applied, with Undo per item (D10);
- the setup canary, the periodic canary and the "source went quiet" state for Knowbox mailboxes (D14);
- the saved session, on by default with a switch: reconcile, recover, re-enable forwarding (D15);
- Disconnect per mailbox, and account deletion (D17);
- `ingest` honours ruling 3 for `title` and `due` (D11 (5); §13 Q7);
- ruling 12's policy-read gate suspended until Knowlu has 20 paying users, for the kept Blackboard
  session of M1 grades and the saved mail sign-in alike (D25; §10, A13; T19);
- the privacy sentences for bump #1 (§7) and the amendments (§10).

**Out of scope, each with its home:**
- reading mail by Graph or IMAP: Graph needs admin consent at UA (E1); IMAP app passwords stay in
  reserve (research note §4). Not designed here.
- Yahoo, Outlook.com and iCloud scripted setup: guided only (§4.4). A provider gains a script when a
  student needs it, through its own small spec.
- writing `commitments/` from mail (a class moved, cancelled or relocated): shown as an info notice in
  the MVP (D24); a change card into the commitment model is a later lane.
- events found in mail as event cards with Accept/Decline: mail events are task-shaped cards in the
  MVP (D24); the events spec's card is campus-feed only.
- the per-course digest's reasoning-model summary is built last and may be cut (D13).
- two desktops (Launch): until the fetch-turn lease lands, one desktop per student runs setup, the
  saved session and the slot's mail step (§9).
- the R-PS-4 re-consent screen and privacy bump #1 itself (Pilot); this spec drafts its sentences.
- asking school IT anything (Q7).

## 3. Decisions

Each cites the answer it carries out. "Cost if wrong" is what the build or the student pays.

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| **D1** *(Q1)* | **All of email ships in the MVP**: receiver, addresses, scripted setup for school M365 and personal Gmail with guided fallback, Gmail's server-side confirm, setup and periodic canary with "source went quiet", full backfill, reconcile and recover. **Gmail OAuth (PR #26) stays as the direct path** and is not changed except by D22. | Quinn's Q1. The pilot schools' mail is M365, unreachable any other way (E1); a mid-semester sign-up needs backfill. | The MVP exit moves out by this lane's size (§14: XL). |
| **D2** *(Q3)* | **The receiver is AWS SES inbound on a subdomain** (MX on `in.knowlu.com`; §13 Q5). An SES receipt rule has **one action, the S3 action, with its own `TopicArn`**: it stores each message in an S3 bucket whose lifecycle expires objects after **one day** and notifies an SNS topic with a notification that carries the message's headers and recipients but never its body (SES's separate SNS action, which publishes the whole message through SNS, is never used). S3 counts a lifecycle day to the next UTC midnight, so an object left behind lives up to about two days plus AWS's own lag; every path that finishes with an object deletes it at once (§4.6, §4.7). SNS delivers over HTTPS to the Supabase function `knowbox-inbound`, which verifies the SNS signature (only the configured topic, a certificate only from `sns.<region>.amazonaws.com`) before reading anything. **No message is ever rejected at the door on SPF, DKIM or DMARC**: SES records the verdicts, and intake judges afterwards (§4.6). **Gate before any build commits to SES:** a deliberately DMARC-failing test message reaches the bucket (T0). | Quinn's Q3. SES is the only candidate confirmed not to act on the verdicts; Cloudflare Email Routing (§13's old choice) rejects DMARC failures; the apex's Cloudflare routing for `hello@`/`support@` is untouched because the MX is on the subdomain. SNS needs no AWS-side code. | If SNS proves awkward (retries, ordering, the subscription handshake), a 20-line Lambda that POSTs with a shared secret replaces it; intake is unchanged behind the same verify-then-read seam. |
| **D3** | **One secret address per connected mailbox**: `<token>@<domain>`, the token 128 random bits in lower-case base32 (26 characters). The service keeps an HMAC of the token for lookup and the token itself encrypted (the `SOURCES_ENC_KEY` pattern of `_shared/crypto.ts`) so Settings can show it again. **Rotate** mints a new token for the mailbox and keeps the old one accepted until the new address's canary arrives (at most 7 days), so mail sent before forwarding is re-pointed is not lost; on Gmail the new address needs its own confirm (§4.3 steps 3–5). **Disconnect** revokes it. An account holds at most **three** live addresses and mints at most six in 30 days. | A token per mailbox binds every message to one mailbox without trusting plus-addressing through each provider's forwarding form, and lets one mailbox be revoked alone. A dump of the table is not a list of live addresses. | None structural. A student with three mailboxes has three addresses; Settings lists them. |
| **D4** *(Q4)* | **Forward everything.** On M365: mailbox forwarding ("Enable forwarding") to the mailbox's address with **"Keep a copy of forwarded messages" on**. On Gmail: Settings → Forwarding → "Forward a copy of incoming mail to" the address, **keeping Gmail's copy in the Inbox**. No filter, and no filter import (E4 is not used). The server's cheap screen (§4.7, step 1) discards noise before any model sees it. | Quinn's Q4: coverage over a student-chosen filter. Keeping the copy means forwarding never removes mail from the student's own mailbox. | Every message transits the Knowbox, which §7 states plainly; the screen's quality decides the model cost. |
| **D5** *(Q2)* | **Setup is automatic and DOM-only.** The mail window (label `mail`, its own WebView2 profile per mailbox, no capability grant, like `lms-grades`) is visible **only** to sign in. A step script runs only while the page's origin is one of the provider's mail hosts (a fixed list per provider, compiled in with the scripts); on an SSO, consent or redirected page nothing runs and the step waits or falls back. Once the page signals a signed-in mailbox (X2's title signal, confirmed by a DOM probe), the window is hidden or moved off-screen (T0 decides which keeps the page rendering) and the scripted steps run in it, in-process, by script evaluation in the page: **never CDP, never a debugging port, never synthetic OS input.** The **main window** shows the progress view (§4.5). The mail window comes back, centred and focused, only when the site needs the student: Google's "verify it's you" popup, an MFA or SSO re-prompt, a consent page. Every scripted step verifies its own effect by reading the page after it acts; a step that cannot find its controls, or whose check fails, falls back to **guided steps** for that step only. | Quinn's Q2. X2 proved both sign-ins work in Knowlu's own WebView2; X3a/X3b proved the settings changes; CDP was the spikes' harness only. | A hidden window may stop rendering (T0); off-screen is the fallback. A page that changes breaks a script until a release (D19); guided steps cover the gap. |
| **D6** *(Q2, §13 Q9)* | **When it runs.** The wizard's panel right after sign-in and subscription **announces it**: "Next: connect your school email", one line on what Knowlu will do, and a one-click **Not now** under which sits risk 12's sentence for a school whose rules forbid forwarding. When the console first opens after Finish, the school mail sign-in window **opens with it** and the progress view shows in the console (§4.5); personal Gmail is offered in the same view, skippable. Settings → Email offers setup at any time after: to a student who chose Not now, and to one whose install was adopted or restored. | Quinn's Q2 ("offered right after Knowlu sign-in") as Quinn confirmed it in §13 Q9: the announcement in the wizard, the sign-in and the progress at the console's first open. The wizard window has no `ConsoleState` and no profile folder until Finish (C1), and a mailbox's backfill matched against no courses would be matched again later. | The student waits until Finish to see setup start; the wizard says it comes next. |
| **D7** *(Q5)* | **Backfill reaches the start of the current term.** The date comes from the curated campus row, which gains `terms: [(start, end), …]` for the academic year (UA's dates in the MVP). For an uncurated school, or a row with no term covering today, the setup view shows a computed default (the most recent of 5 January, 15 May and 15 August before today) **and asks the student to confirm or change it**. **No age cutoff** inside that window: an item already past due is recorded and archived as `imported-past` (R-OB-3); everything else counts. | Quinn's Q5: "relevance, not age"; the purpose is a complete picture of what changed. F2: nothing records a term today. | A wrong default reads too little or too much mail; the student sees the date before anything is read. |
| **D8** | **Intake is two stages and a separate queue.** `knowbox-inbound` verifies, binds and splits: each original message (a live forward, or one `.eml` unwrapped from a backfill send) becomes its own S3 object under `work/` (same one-day lifecycle) and one `knowbox_work` row holding only its key, account, mailbox, phase and arrival time. A worker, `knowbox-process` (cron every minute, and kicked by intake), takes items within its wall-clock budget: parse, screen, extract claims, queue them in **`mail_claims`**, delete the object. The device pulls `mail_claims` through `mail-pull`; **`gmail_queue` is not touched** (`gmail-read` changes only by D22). **The `knowbox_seen` row is written together with the `knowbox_work` row, only after the S3 put succeeds**, so a failed put leaves nothing that would make SNS's retry look like a duplicate. | A 25 MB backfill send of ~100 originals cannot be judged in one invocation; the raw text lives only in S3 and in a worker's memory, never in a table; a separate queue keeps the Gmail path's quiet rules (F4) out of the Knowbox's. | An item not processed before its object expires is lost to that pass: the sweep counts it `expired` on the mailbox **and deletes its `knowbox_seen` row**, so the next reconcile lists it as missing and recover's re-forward is accepted (D15). |
| **D9** *(Q5)* | **Claims, cheapest first.** Per message: (1) the screen; (2) deterministic parsers for templated senders (Blackboard, Canvas and zyBooks notifications, as data rows beside `RECEIPT_TEMPLATES`), with the existing due resolver and course-code matching; (3) the rest to a **small model** (judge's tier 3, a new kind `mail_claim`) returning a fixed schema: up to five claims, each `{kind: new_item \| change \| course_info \| noise, course, target, field, value, evidence, confidence}`; a `new_item` also carries `effort_hours` and `importance` from fixed ranges, as a Gmail `task` verdict does, so a note made from mail is never sent to the device's task pass (§5.2). **`evidence` must occur verbatim in the message** (after whitespace folding), or the claim is refused as `incomplete`; the confidence floor is the existing 0.6. | Quinn's Q5. VISION: classification over generation, schema on every call, rules first. The evidence check is deterministic and makes a model's invented change unqueueable. | A real change worded only across a quoted reply may be refused; the student still sees the mail in their mailbox, and live mail of that shape files no card. |
| **D10** *(Q5, Q5b)* | **Reconcile is deterministic, on the device, in `judge`.** Claims are applied oldest-first by the message's **ordering date** (its Date header, capped at the time the Knowbox received it for live mail, or at the provider's own delivery time, else the batch's opening, for backfilled and recovered mail; §4.8), then uid, then claim index; a later claim supersedes an earlier one for the same item and field, held across pulls by a per-field `mail_asof` on the note (§5.2). **A backfill or recover batch is delivered whole**, in that order, only once it has closed, and a mailbox's live claims wait behind its open batch (§4.8), so the same messages give the same vault however they arrive. **Backfill applies silently and lists; live mail proposes.** §4.9's table says, per claim kind and phase, which writes are direct agent writes and which are cards. **Undo** is per item in "From your email": a created item is deleted with `delete_note`, a changed field is set back to the journal's `old` value with `set_fields` (as the student, so judge-once then protects it). | Quinn's Q5 and Q5b. `rank` never calls a model; the reconcile needs no model, only the vault. Undo reuses two commands that exist (F3). | An Undo of a field makes it the student's, so later mail about that field becomes a card. That is the intended reading of "the student decided". |
| **D11** *(Q5)* | **A mail change never silently overrides the student or an authoritative source.** (1) **The reconciler decides card or direct itself**, for every field it touches, from `Journal::human_set(id, field)`, where a human `create` carrying the field counts (a note the student made by hand is theirs); it never relies on `write`'s judged set, which does not cover `due` (F5). (2) A change to a field the student set is a `kind: amend` card in both phases. A change to a note owned by an authoritative feed (an LMS or homework-platform item, by its `created_by`) is a card in both phases unless the note already holds the claimed value (then nothing is written); §4.9 gives the template rows. (3) **Mail cards are written by `mail.rs`'s own card writer** through `write::create`, in exactly the shape `approvals::validate_amendment` accepts (`kind: amend`, `target`, `changes: {field: {from, to}}`, `proposed_at`, `first_proposed_at`, `expires: null`, `snooze_until: null`) plus `created_by: mail` and D21's fields, and only after `write::find_pending_amendment` finds no pending card for the same note and fields (the `write_gmail_card` precedent; `propose_amendment`'s "re-judged" text would be false here). (4) **Approving a mail `kind: amend` card writes its changed fields as the vault's human actor** (`journal::read_human_actor`), not as `agent:approvals`, so the approved value is the student's and later mail about it is a card. If `read_human_actor` fails (ruling 11: a bad `config/actor.yaml`), the approval stops with that named error and the card stays pending; it never falls back to another actor. (5) **Against the feed, `ingest` honours ruling 3 for `title` and `due`** (§13 Q7, Quinn: fix it now): where `human_edited(id, field)` exists and the feed differs, it files one `kind: amend` card instead of writing (none while one is pending; a value the student rejected is not proposed again until the feed's value changes), so an approved mail value holds and a later move in the LMS is a card. A mail `kind: task` card materialises as today. | VISION commitment 5, ruling 3 ("no source overwrites a field the journal shows the student set by hand; that change is filed as an amend card instead") and Q5's judge-once. F1, F5. | `approvals.rs` and `ingest.rs` change (§8): every vault whose student hand-set an LMS item's `title` or `due` gets a card where today the feed overwrites silently, which is what ruling 3 says and what Quinn chose. |
| **D12** *(Q5)* | **A reasoning model only for ambiguous references, batched per course.** When a change's `target` matches no note, or more than one, deterministically (§4.8), the device sends one batch per course to `mail-resolve`: the claims and the course's candidate items (id, title, due). The answer names one candidate or none for each claim, and is **stored on the claim row** so a re-pull replays the same answer. Unresolved claims become an info notice ("Knowlu could not match this to an item"), never a guess. | Quinn's Q5. Determinism under replay: the model's answer is data. | One more pinned kind (`mail_resolve`), a few calls per backfill. |
| **D13** *(Q5)* | **Course information becomes info notices, superseding by key.** A `course_info` claim (schedule, room, instructor, office hours, policy) opens an `info/` item of kind `notice`, titled by course and topic, with `close_key: mail:<course>:<topic>`; a later claim for the same course and topic closes the earlier one and opens its own (`info.rs`'s close-by-key). Notices are written in both phases: they inform and change nothing in the plan. **The per-course digest** (a reasoning model folding a course's notices into one `info/` note) is optional: built last, behind `mail_digest: false` by default, and cut if time is short. | Quinn's Q5. Uses `info.rs` as it is; supersession is the existing key rule, so no new state. | If Quinn reads a notice as "Knowlu noticed → propose", notices become cards; the table in §4.9 changes one row. |
| **D14** | **Canary and "source went quiet" ship in the MVP for Knowbox mailboxes.** At setup, a canary from Knowlu's own domain (DKIM-signed, a nonce in a header and the subject) is sent to the student's mailbox and must come back through forwarding; the setup waits up to three minutes. After setup a mailbox is watched passively (`last_received_at`); a periodic canary is sent only after the mailbox has gone quiet, and at least 72 hours after that mailbox's last canary (§13 Q2; §4.10). States: `active`, `quiet`, `blocked` (forwarding set and verified on, but the setup canary never arrived), `off` (the saved session found forwarding turned off or pointing elsewhere), `paused_send` (Microsoft
refused a backfill or recover send, §4.2 step 5), `lapsed` (no entitlement, §4.6 step 3), `revoked`. Each
non-active state is a named line in the run and in Settings, and an issue in the Issues panel. | Quinn's Q1 puts the periodic canary in the MVP; VISION: silence is never ambiguous. Ruling 10 put "source went quiet" in the Pilot for Gmail; this moves the Knowbox's own to the MVP (§10, A5). | A canary to the student's own mailbox is mail Knowlu sends them; it obeys VISION's "one daily email at most" (§4.10). |
| **D15** *(Q6)* | **The saved session is on by default for everyone, at every school, with a clear switch** (Q6; §13 Q6, Quinn: no policy-read gate for it, and ruling 12's gate for the Blackboard session suspended beside it until Knowlu has 20 paying users, D25). The mail window keeps **one WebView2 profile per connected mailbox** at `<data_dir>\mail-session-<address_id>` (never the vault, never synced, no password kept; Windows encrypts the profile's cookies with DPAPI, but the pages and mail the site caches in the profile are stored there unencrypted). It is used to **reconcile** (compare what the mailbox received in a window with what the Knowbox holds, on the device), **recover** (forward the missing messages as attachments) and **re-enable** forwarding found turned off, automatically and then with a notice, never for a mailbox the student turned off in Knowlu's own Settings (§4.11; §13 Q11). Reconcile runs once a day, revisited after the 10-08 session-lifetime spike (§13 Q3). An expired session asks the student to sign in **only when a repair is needed**. Switching it off deletes every mailbox's profile; setup still works, in a `mail-setup` profile wiped when the window closes and swept at start-up if a crash left it. A mailbox's profile is deleted on that mailbox's Disconnect, and every mail profile on Knowlu sign-out and on account deletion; a delete that fails is retried at the next start-up and named in Settings until it succeeds. Named on the privacy page. It is a second named exception to §11a's "keep nothing", beside ruling 12's (§10). | Quinn's Q6, on ruling 12's precedent. One profile per mailbox means "Disconnect Gmail" leaves no Google session behind while the school mailbox stays connected. | A saved session is full mailbox access on the device; malware running as the student can use it (§7, §11). |
| **D16** *(Q7)* | **No contact with school IT.** The setup canary detects a tenant that blocks external forwarding (Microsoft's default for tenants since 2021, NDR 5.7.520, which the student never sees) and names it: "Your school is blocking forwarding to Knowlu." The setup then offers only what remains: personal Gmail, and the guided steps. | Quinn's Q7. E2 and X3a proved UA's forwarding works today. | A school that blocks later is found by the periodic canary, not in advance. |
| **D17** | **Disconnect and deletion leave nothing that serves the connection.** Disconnect (per mailbox) turns forwarding off in the mailbox (the saved session, or a sign-in, or guided steps), revokes the address, deletes the mailbox's `knowbox_seen`, its `mail_claims` **delivered or not** (the Gmail disconnect precedent), its `knowbox_work` rows and their S3 objects (by key, read from `knowbox_work` first), its batches and canaries, and that mailbox's saved-session profile. Mail that still arrives at a revoked address is deleted **without its body ever being fetched** (§4.6 step 2). `DELETE /account` calls the same purge for each mailbox before the cascade and names every Knowbox table in its list (§4.13); the app tries to turn forwarding off with the saved session first and says so on the confirm. The account's export includes its mailboxes (provider, `mailbox_hint`, status, dates, counters) and its queued claims; never the token, its HMAC, a key or a hash. | The Gmail spec's D14 rule, applied here: delete what exists only to serve the connection. The judgment rows (no text) go with the account, as today. | A forwarding the student never turns off keeps sending mail to a black hole. The disconnect copy says so, and guided steps show how to stop it. |
| **D18** | **This lane moves no `PRIVACY_VERSION` and edits no `site/privacy.html`.** §7 drafts the sentences; they join privacy bump #1 (Pilot), whose PR moves the page, its date and the constant together after its lawyer read. **No release is tagged from a `main` that carries the Knowbox setup, or the suspended grades gate (D25), until bump #1 has merged** (the guard the Gmail spec's D13 already put in HANDOFF, widened twice). | Ruling 12's and the Gmail spec's pattern. The MVP proof runs on a dev build against staging. Ruling 12 let a release carry M1's code before bump #1 only "because without a date it offers grades nowhere"; with the gate suspended that no longer holds (D25). | A release cut early would ship a page that does not name the Knowbox, AWS, the saved session, or a kept Blackboard session now offered at every school. |
| **D19** | **The step scripts are compiled into the app** (one readable JS file per provider and step, with its selectors and its check) and versioned with the release. No script is fetched from a server in the MVP. | A server-supplied script would run inside a signed-in mailbox; that needs signed delivery and its own review. The MVP's one user can take a release. | A provider's UI change breaks a step until a release; guided steps cover the gap (D5). Signed remote scripts are a Pilot or Launch item. |
| **D20** | **Names.** Journal actor `agent:knowlu.mail` for every reconcile write; `created_by: mail` on notes and cards; uids `mail:<h>` where `<h>` is the first 32 hex of SHA-256 of the message's Message-ID (a message with none: of its Date, From and Subject), and `mail:<h>:<n>` for its *n*-th claim. `state/ingest-seen.md` records them like `gmail:` uids. | `agent:` keeps judge-once (`provenance::is_agent` is a prefix test). Hashing keeps Message-IDs, which can carry addresses, out of the vault. | None. |
| **D21** | **Mail cards, notices and applied changes name their sender** (display name and address) and the message's date. | VISION commitment 5: "showing who it came from"; a moved deadline is judged by who moved it. | It diverges from Quinn's decision for Gmail cards (the Gmail spec's D15, decided at `gmail-connect-design:901`: no sender in the MVP). Quinn chose it (§13 Q10: mail cards show the sender) and the ruling text Quinn signs (§10) states the divergence; disclosed in §7. |
| **D22** | **One mailbox, one path; the Knowbox wins.** When a Gmail address has an active Knowbox and is also connected by OAuth, `gmail-read` reads nothing new and answers an ordinary, **non-quiet** pull: `{items: <undelivered>, read: 0, quiet: false, more: false}`, plus a field `via_knowbox: true` that today's device ignores. Already-queued items are still delivered and acked. Calendar is untouched. **Never `quiet`:** the merged device reads every unknown quiet reason as `Revoked` ("re-connect from settings") and stops before reading items (`cloudmodel.rs:667-675`). | No cross-path dedup is needed, and the Knowbox's backfill and canary are the stronger path. Zero device change. | One small change to the merged `gmail-read` (§9), with a handler test that the answer is not quiet and carries the undelivered items. |
| **D23** | **A one-time backfill allowance** above the daily email cap: an allowance of `mail_claim` judgments **per account per term** (held on `knowbox_accounts`, keyed by the term's start, so Disconnect and reconnect never renew it), shared by the account's mailboxes, usable within 14 days of the first setup, charged by its own RPC. Its calls' tokens are recorded through `record_tokens` like every other call's, so `monthly_spend` sees their cost, while their count is kept on `knowbox_accounts` and never in `usage_daily.calls`, so a backfill never uses up the live daily cap; while the allowance is open, `enforce_budget`'s ceiling for that account is `MONTHLY_CEILING_USD` plus the allowance's dollars, a hard per-account ceiling, and back to `MONTHLY_CEILING_USD` after. Size (§13 Q1): **2,000 `mail_claim` judgments per account per term, and $1.00 above that account's ceiling while the allowance lasts.** | Q5 asks for it sized. The screen and parsers remove most mail before the model; what is left is bounded by the window. Holding it per account means a mint-and-disconnect loop buys nothing (D3 also caps addresses). | Too small: backfill trickles over days at the daily cap. Too large: a looping bug costs more before it trips. |
| **D24** | **Events and schedules stay the student's call.** A dated event in mail (a meeting, a club sign-up) is a `new_item` with `item_kind: event` and, **when it is still ahead**, is always a card, in both phases. An event already past when the claim is applied files no card: in the backfill it is recorded and archived `imported-past` (Q5: "recorded and archived"); live or recovered, it is counted in the run line's past count and nothing is written, since no one can accept a past event. A claim about a class meeting (moved, cancelled, new room) is a notice (D13); it never writes `commitments/`. | The events spec's D2 ("obligations are asked, never auto-created") and the commitment model (commitments are what the student confirmed). | A cancelled class still blocks its time until the student edits it; the notice tells them. |
| **D25** *(§13 Q6)* | **Ruling 12's policy-read gate is suspended until Knowlu has 20 paying users, for both kept sessions.** The kept Blackboard session (M1 grades) and the saved mail sign-in (D15) are offered and used at every school, with no recorded university-policy read. For grades, "every school" is every curated campus row whose `lms_kind` is `blackboard`: the host still comes only from the curated row, never a typed address, so an uncurated Blackboard school still reads "not available at your school yet". For mail it is every school (§4.1). The gate stays mechanical and in one place: `grades::availability` (`app/src/grades.rs:40-50`) reads one constant, `POLICY_READ_GATE`, set to `Suspended`, the same in every build (no `cfg`, feature or environment variable). The `policy_read` field, the date-and-bump test and the named reason all stay, so restoring the gate is one constant, with the `Enforced` arm's predicate tests already written; the undated cases T19 rewrites in the command and scheduler tests go back with it, T19's diff being their record (T19). **Review trigger:** the 20th paying account, counted as an `active` paid subscription (`trialing` and founder-owned test accounts not counted); Quinn rules again before the next release tagged after it. HANDOFF's queue carries the trigger, set 2026-09-30, and the controller's count-only query checks it at each milestone update. **Disclosure first:** D18's release guard covers the suspension. | Quinn's answer to §13 Q6 (2026-09-30): "change both to allow … Once we have 20 paying users, we'll come back to this … give ourselves as much of an advantage as possible." The MVP and the Pilot prove the concept as a desktop app, and the reads would hold back the founder's own grades proof. | A school whose policy forbids a kept SSO or mail session is learned about from the school, after the fact, not from a read (§11, risk 17). The lawyer read in bump #1 still happens. |

## 4. The flows

### 4.1 Which mailboxes, and the address

1. **Which provider.** The school mailbox's provider and domain come from the curated campus row, which
   gains `mail_provider` (`m365`, `google`, `other`) and `mail_domain` (UA: `m365`; its student domain).
   For an uncurated school, the setup asks once for the school email address and the service decides
   the provider from the domain's MX records (`*.mail.protection.outlook.com` is M365; Google's MX hosts
   are Google); an unknown provider is guided (§13 Q4: the curated row, else this MX lookup). Personal
   Gmail is always offered beside it, and skippable.
2. **The address.** `POST /knowbox/addresses {provider}` mints the mailbox's address (D3) and answers it
   with an `address_id`; a fourth live address, or a seventh mint in 30 days, is refused with a named
   reason. The address is shown in the progress view (the student may want it) and is never logged.
3. **The binding.** After sign-in the script reads the signed-in mailbox's own address from the page and
   sends it as the mailbox's `mailbox_hint`. Intake binds live mail to the mailbox by it (§4.6), and the
   canary is sent to it.

### 4.2 School Microsoft 365, step by step

The mail window opens at Outlook on the web, visible, on the mailbox's profile (`mail-session-<address_id>`
with the saved session on, `mail-setup` with it off; D15). The main window's progress view lists the
steps below as they run.

1. **Sign in.** The school's own sign-in (Microsoft, then the school's SSO and Duo or Okta Verify; X2).
   Knowlu never sees the password. Done when the title reads as a signed-in mailbox and the mail list
   is present. A window the student closes is "setup cancelled", nothing changed.
2. **The window steps aside** (D5). From here the student watches the main window.
3. **Forwarding.** Navigate to Settings → Mail → Forwarding (by navigation, never a page reload, which
   breaks the driver; research note §6). Read the switch's `checked` property, not `aria-checked`.
   - **Already forwarding elsewhere:** the step stops and asks in the progress view: forward to Knowlu
     instead (the school inbox keeps every message), or stop. The script never replaces an existing
     forwarding address without that answer. (An inbox rule that forwards to both is untested; T0 S6
     decides whether it becomes a third choice.)
   - **Set:** switch on, the Knowbox address, "Keep a copy of forwarded messages" on, Save. No
     re-authentication was needed in X3a.
   - **Check:** navigate away and back; the switch reads on, the address reads back, the copy box is on.
4. **Test message (canary).** `POST /knowbox/canary` sends one; the progress view waits up to three
   minutes for it at the Knowbox (X3a: about 40 s). Arrived: the mailbox is `active`. Not arrived: the
   view says the school may be blocking forwarding (D16), shows guided checks (Junk, the setting), and
   leaves the mailbox `blocked` with Retry.
5. **Mail since the term started** (D7). Search for mail received since the start date **in the
   mailbox's own folders, excluding Junk Email and Deleted Items**, and read the result count (the
   list's `aria-setsize`; volume in the research note §2); `POST /knowbox/batches` opens the backfill
   batch with that count as `expected`. Then the **recover routine** (§4.11) runs over that window: it
   lists the messages, compares them on the device with what the service holds, and forwards each
   missing one as an attachment, one at a time (Outlook forwards one message at a time; B-C), **oldest
   first**, to the Knowbox address, with the batch's nonce in the subject.
   **The pace is Microsoft's, not ours.** Exchange Online allows 30 messages a minute per mailbox and
   reacts to outbound bursts by restricting the sender, which would stop the student's school account
   from sending until school IT unblocks it (Q7 rules out asking them). So the routine forwards at most
   **10 a minute and at most 600 a mailbox per UTC day** (provisional; T0 S6 measures and may lower
   them), and **stops at the first non-delivery report or send failure** it sees in the page, with the
   named outcome `mail: forwarding paused — Microsoft refused a send` in the view, Settings and the run
   line; nothing retries until the next day. The view shows N of M and a time left. When the last
   forward is sent, the device closes the batch with the number it sent (`POST /knowbox/batches/<id>/sent`).
   Each forward leaves a copy in Sent Items; **the script deletes only the copies it created** (matched
   by the nonce and the Knowbox recipient) and the view says it will.
6. **Finish.** With the saved session on (the default, D15) the window closes and the mailbox's profile
   stays; with it off, the `mail-setup` profile is deleted. A backfill still running when the student
   quits **resumes the next time the app is open** with the saved session (it is the same recover
   routine; §4.11), or at "Resume" in Settings.

### 4.3 Personal Gmail, step by step

1. **Sign in** at Gmail (password, then Google's phone prompt; X2). Done when the title reads
   "Inbox … Gmail"; the address in the title is the `mailbox_hint`.
2. **The window steps aside.**
3. **Add the forwarding address.** Settings → Forwarding and POP/IMAP (by hash; a hash round trip
   re-renders a section that did not draw, research note §6), "Add a forwarding address", the Knowbox
   address, Next, Proceed. The buttons are `<input type=button value=…>`.
   - **Google asks "verify it's you"** in a popup (X3b). The popup is allowed (T0 S1); the mail window
     comes back, centred, and the progress view says "Google wants to confirm it's you. Check your
     phone." The phone tap is the student's. The window steps aside again when the popup closes.
   - **Already forwarding elsewhere:** as §4.2 step 3.
4. **Knowlu confirms the address** (E3). Google mails a confirmation to the Knowbox address. Intake
   recognises it (§4.6 step 5) and completes it server-side with a cookie-less request to Google's
   confirmation page. The view polls the service until the mailbox reads `confirmed`.
5. **Turn forwarding on.** Back in the Forwarding section: "Forward a copy of incoming mail to" the
   address, "keep Gmail's copy in the Inbox", Save Changes. Check by re-reading the section.
6. **Test message**, as §4.2 step 4.
7. **Mail since the term started.** Open a batch as on M365, then search `after:<start date> -in:spam
   -in:trash` (Gmail's default search already leaves both out; the terms make it explicit), select a
   page of results, oldest page first, More → **Forward as attachment** (B-C Gmail: each `.eml` is the
   complete original with its own DKIM and Gmail's own `Authentication-Results` and ARC set from
   delivery), in sends under Gmail's 25 MB per message (about 100 messages), each with the nonce, and
   close the batch with the count sent. Gmail's sending limits are far above this volume; the routine
   still stops at the first send failure with a named outcome. The script deletes the Sent copies it
   created, as on M365.
8. **Finish**, as §4.2 step 6.

### 4.4 Every other provider, and every failed step: guided

A guided step shows the address with **Copy**, the provider's own path to the setting in four lines or
fewer, and **I've done this**, which runs the step's check (a canary for forwarding; a backfill count
for a manual forward-as-attachment). Outlook.com, Yahoo (whose free accounts lost auto-forwarding) and
iCloud are guided only. A scripted step that fails falls back to that step's guided version and the
steps after it continue scripted when they can. Nothing is ever retried in a loop.

### 4.5 The progress view (main window)

A panel in the console, opened by setup and from Settings → Email. Per mailbox, one row per step, each
with a state: *waiting*, *working*, *done*, **needs you** (with the reason, and the mail window brought
back if the site is asking), *done by hand*, *skipped*, or *failed* (with the named reason and the guided
step). Above the rows, one sentence of what is happening now. Backfill shows N of M and a time left.

- **Leave** closes the panel; setup carries on in the background, and the Email row in Settings shows
  its progress. Quitting Knowlu stops it; with the saved session it resumes the next time the app is
  open (§4.11).
- **Stop** ends setup after the current step and says exactly what was done ("Forwarding is on; mail
  before 12 September was not brought in").
- No step's text names a selector, a URL or the address's token in a log.

### 4.6 Receiver intake (`knowbox-inbound`)

Every arrival, in order. Each step that ends the message deletes its S3 object at once. **"Deleted
unread"** below means deleted without the object ever being fetched: the function then holds only the
SNS notification, which carries the message's headers and recipients (SES puts them there), reads only
the recipients from it, and logs nothing from it.

1. **Verify the notification** (D2): SNS signature, topic, certificate host. A subscription handshake is
   confirmed only for the configured topic. Anything else answers 403 and reads nothing.
2. **Find the mailbox, from the notification alone.** For each address in the notification's
   `receipt.recipients` at the Knowbox domain, look the token up by HMAC (a token retired by Rotate still
   counts until its grace ends, D3). Unknown or revoked: **deleted unread** (counted globally, never per
   account).
3. **Entitlement.** An account without an `active` or `trialing` entitlement: **deleted unread**, no
   `knowbox_seen` row, and the mailbox marked `lapsed` with `lapsed_since` (shown in Settings). The
   server has no payment grace (the 72-hour grace is the device's cache rule, §1); instead a lapse loses
   nothing for good while the saved session works: when the account is entitled again, the next
   reconcile covers from `lapsed_since`, at most 30 days back, and recover brings in what was dropped
   (§4.11).
4. **Fetch** the object from S3 with a key scoped to the bucket (get, put, delete; nothing else).
5. **Classify the arrival:**
   - **Canary:** from Knowlu's canary sender, DKIM passing for Knowlu's domain, carrying the nonce of an
     outstanding canary for this mailbox: mark it arrived. A canary arrives through the mailbox's real
     forwarding, so intake also records the forwarding evidence it carries (for M365, the tenant id in
     `X-MS-Exchange-ForwardingLoop`) as the mailbox's `forward_hint`, which binds later live mail. Done.
   - **Gmail forwarding confirmation** (E3): from **exactly** Google's forwarding-confirmation sender,
     with DKIM passing and aligned for `google.com`; the requester address parsed from the message,
     case-folded, **equal** to the mailbox's `mailbox_hint` (never a substring: `notjane@gmail.com` is
     not `jane@gmail.com`); refused while the hint is null; inside the mailbox's confirm window (15
     minutes from setup's step 3); its link's host and path on a fixed allow-list. The page is fetched
     cookie-less with `redirect: manual` (a redirect is refused), and its one form is submitted only when
     the form's action is on the same allow-list. Anything else is refused and counted: a stranger
     cannot point their Gmail at a student's Knowbox.
   - **Backfill or recover send:** the outer From is the mailbox's `mailbox_hint`, the outer message
     passes DKIM aligned with that address's domain, and the subject carries an open batch nonce for this
     mailbox. Each `message/rfc822` attachment becomes one work item with the batch's id and phase
     (`backfill` or `recover`) and counts toward the batch's `received`. Every other attachment and the
     outer text are dropped without being parsed.
   - **Live forward:** bound to the mailbox **only by proof the provider's own forwarding hop added**,
     never by a header a sender can write. Both must hold: (i) the highest-instance ARC set is sealed by
     the provider (`d=google.com`; for M365 the Microsoft signing domain T0 S0 records), verifies, and its
     ARC-Message-Signature's `h=` covers the binding header, which names this mailbox (Gmail's
     `X-Forwarded-For` naming `mailbox_hint` and the Knowbox address; Microsoft's
     `X-MS-Exchange-ForwardingLoop` naming the mailbox and its `forward_hint` tenant); and (ii) SES's SPF
     verdict passes for the envelope sender, whose shape is the provider's forwarding return path for
     this mailbox (Gmail's `<local>+caf_=…@gmail.com` naming the mailbox; Microsoft's SRS address at the
     mailbox's own domain), as T0 S0 records. A forged `X-Forwarded-For`, or a seal from an attacker's
     own Gmail or M365 tenant, fails one of the two. One work item, phase `live`.
   - **Anything else** is unbound: deleted and counted on the address. A rising count means the address
     has leaked, and Settings offers **Rotate** (D3). Unbound mail never touches `last_received_at`.
6. **Authenticity, as the provider saw it at delivery.** Only one record counts: for a live forward, the
   ARC-Authentication-Results of the provider's hop that step 5 verified; for a backfilled or recovered
   original, the provider's own `Authentication-Results` stamped at delivery (Gmail's `mx.google.com`;
   for M365 the header EOP stamped, identified as T0 S0 records) or that provider's ARC set. Every other
   `Authentication-Results` in the message, including any the sender wrote, is ignored. The item is
   **`aligned`** when that record shows `dmarc=pass`, or an aligned `dkim` or `spf` pass, for the From
   domain, else **`unaligned`**.
   - A live `unaligned` item is deleted and counted `unauthenticated`.
   - A backfilled or recovered `unaligned` original is kept (keys rotate, and the student did receive
     it) but **may produce only cards and notices, never a direct write** (§4.9). The outer send's DKIM
     proves only that the batch is the student's.
   - The words are recorded on the work item, never the headers. (E2 showed UA professor mail signed by
     `ua.edu`; T0 S0 checks an intra-school message too, and if a school's internal mail proves unsigned,
     Quinn hears the proportion before T2 fixes this rule.)
7. **Dedup.** `knowbox_seen` (per account) holds `HMAC_k(Message-ID)` and a fingerprint
   `HMAC_k(sender, normalised subject)` with the message's **received minute**, where `k` is the
   account's fingerprint key: for live mail, the Knowbox's arrival; for a backfilled or recovered
   original, the provider's delivery time from its own topmost `Received` header, else its Date. Keyed
   hashes, so a stolen table cannot confirm a guessed message. A Message-ID already there: deleted, done.
   This is what makes recover idempotent and drops a message that reached two mailboxes.
8. **Hand off.** Put the original as its own object under `work/`; **only after the put succeeds**,
   insert its `knowbox_work` row and its `knowbox_seen` row in one transaction; delete the `inbound/`
   object; update `last_received_at` (live items that passed step 6 only); answer 200. A failed put
   answers 500 with no row written, so SNS's retry is processed as new.

### 4.7 Claims extraction (`knowbox-process`)

Per work item, within the worker's wall-clock budget, oldest arrival first:

1. **Parse.** Headers (From, Date, Subject, Message-ID, `List-*`, `Precedence`), the first `text/plain`
   part, else the HTML part reduced to text. No attachment is opened. The text is cut to 2,000
   characters and passed through `scrubForPrompt` before any model sees it.
2. **Screen, no model.** In order; the first that fires decides, and the item is never judged and its
   object is deleted:
   - bulk or list mail (`List-Unsubscribe`, `Precedence: bulk` or `list`) from a sender not on the
     allow-list (the school's `mail_domain`, any `.edu`, and the LMS and homework-platform vendors' own
     domains) is `noise`;
   - Microsoft's bulk score, where the header exists, at 7 or above is `noise`;
   - the account's promoted rules (`judge_rules.ts`, tier 2) answering noise;
   - the student's own mail coming back (From is the mailbox itself, outside a batch) is `noise`.
3. **Templates, no model.** Data rows beside `RECEIPT_TEMPLATES` for the LMS and homework platforms'
   notification mail (new assignment, due soon, due date changed, announcement, submission received),
   each matched on the vendor's exact sending address, its subject and its body, and **only when the
   item is `aligned`** (§4.6 step 6) for the vendor's domain. A template's claims are marked
   `source: template` (§4.9 says what that changes). Due dates go through `judge_due.ts`; course codes are matched against the known courses.
   Adding a vendor is a data row; the code never names one.
4. **The small model** for everything left: kind `mail_claim`, pinned in `models` like every kind
   (prompt and grammar `mail-1`), with the subject, sender, date, text, the account's known courses
   (from its synced `courses/` notes, else from its judgments as `gmail-read` does) and the timezone.
   Validation (`judge_validate.ts`): the kind and field sets of §5.1, a known course or null, `due`
   through the resolver, a title, `effort_hours` and `importance` on `new_item`, a topic from the fixed
   set on `course_info`, the evidence check (D9) and the 0.6 floor. A refused reply is no claim.
   **Step 5 runs before this step's call, never after it.**
5. **Charge, before the model.** A `backfill` item charges the account's allowance (D23); a `live` or
   `recover` item charges the daily cap of `mail_claim`; then `enforce_budget` (with D23's raised
   ceiling while the allowance is open) must pass. Capped or over budget: no model call; the item waits
   in `knowbox_work` until the next UTC day, at most three attempts, and when its object expires first
   it is counted `expired` and its `knowbox_seen` row is deleted (D8).
6. **Queue and forget.** One `mail_claims` row per message that has any non-noise claim: the uid, the
   message's Date and its ordering date (§4.8), the sender's display name and address, the phase and
   batch id, the authenticity word, the claims and the judgment id. **Every final path deletes the work
   object** (queued, noise, refused, template-only, unauthenticated, expired), and a test asserts no
   object is left after processing. The text was never written anywhere.

### 4.8 Reconcile, on the device (`judge`'s mail pass)

The `judge` step gains a mail pass after the Gmail pull, entitlement-gated like the rest of `judge`.

1. **Pull** `POST /mail-pull {ack}`. Every answer is in (ordering date, uid) order, where a message's
   **ordering date** is the earlier of its Date header and its received time (the Knowbox's arrival for
   live mail; the provider's delivery time, else the batch's opening, for backfilled and recovered
   mail), so a message dated in the future cannot outrank later mail. The service delivers:
   - a mailbox's live claims as they are queued, while it has no open batch;
   - a **backfill or recover batch only once it has closed** (received reaches the count the device
     reported sent; or 2 hours after the device closed it; or 14 days after it opened) and then whole;
   - live claims queued while the mailbox's batch was open, after that batch.
   Live mail takes up to three rounds a slot, each bounded like `pull_gmail`'s. **A closed batch is
   drained past three rounds**, until `judge`'s time budget is spent, and the rest continues at the next
   slot; when a backfill batch closes, the app starts a slot at once through `scheduler::run_slot`
   (`scheduler.rs:681`) rather than waiting for the next one.
2. **Resolve each claim's target, deterministically:** a `target` equal to a note's `source_uid` or LMS
   id; else a note in the claim's course whose normalised title (case, punctuation and whitespace folded)
   equals it; else the one active note in the course whose title holds every token of it. Zero or two
   or more: ambiguous. A `new_item` whose normalised title equals an active note in its course is read as
   a change to that note's `due`, never a duplicate.
3. **Ambiguous claims**, one batch per course, go to `mail-resolve` (D12); the answers come back stored.
4. **Already applied elsewhere?** Before creating a note or a card for `mail:<h>:<n>`, look for a note in
   `tasks/`, `archive/` or `approvals/` whose `source_uid` is that uid (another desktop's write, arrived
   by sync). Found: nothing is written and the uid is recorded seen. `state/ingest-seen.md` is local to
   one desktop; the synced note is not.
5. **Apply** in order through `write`, journal first, as `agent:knowlu.mail`: §4.9 decides direct write or
   card, and the reconciler decides it from the journal (D11), never from `write`'s judged set. Per note
   and field, a claim whose ordering date is not later than the note's `mail_asof` for that field is
   **superseded** and skipped; a write sets `mail_asof` for the field. **On the vault's first day**
   (`commitments::vault_day` is 1), a mail card is written with `proposed_at` and `snooze_until` both the
   vault's second day (`first_proposed_at` is the day it was filed), so it charges day 2's budget and
   shows from day 2; it is never dropped. Direct writes and notices are not held.
6. **Ack** and record each `mail:<h>` in `state/ingest-seen.md`.
7. **One line:** `mail: N new, M changed, K proposed, J notices, P past, S superseded, U unmatched`.
   No mailbox connected prints nothing; a quiet, blocked, off, lapsed or paused mailbox adds its named
   line.

Same vault, same claims and same day give the same writes; and because a batch is applied whole, in
ordering-date order, the same messages give the same vault however their arrival was split across
pulls. The model's only inputs to the device are claims and stored resolutions, both data.

### 4.9 Which writes are direct and which are cards

**Exactly which writes are direct agent writes and which are cards** (Q5b). "Direct" is a write as
`agent:knowlu.mail` through `write`, journal first, listed in "From your email" with Undo and shown in
"what changed"; **only the backfill makes direct writes, and only from an `aligned` original** (§4.6
step 6): an `unaligned` backfilled original's direct rows become cards. "Card" is a proposal through the
normal path: charged to the 15-a-day budget, overflow snoozed, never deleted, and on the vault's first
day snoozed to day 2 (§4.8). **Live** is mail forwarded after setup and mail recovered after setup
(recover brings in live mail the Knowbox missed); the recover batches that finish an interrupted
backfill are backfill. Card or direct is decided by the reconciler from `Journal::human_set` (D11).

| Claim, and what it touches | Backfill (mail since term start) | Live (forwarded, or recovered after setup) |
|---|---|---|
| `new_item`, task-shaped, due ahead or no due | **direct**: a task note (no due: "needs a date") | `kind: task` card |
| `new_item`, task-shaped, already past due | **direct**: created and archived `imported-past` | `kind: task` card |
| `new_item`, `item_kind: event`, still ahead | card (D24) | card |
| `new_item`, `item_kind: event`, already past | **direct**: recorded and archived `imported-past` (D24) | nothing; counted in the run line's past count |
| `change` to a mail-created note, field not set by the student | **direct** | `kind: amend` card |
| `change` to a field the student set (`human_set`; a note they made by hand counts) | `kind: amend` card | `kind: amend` card |
| `change` (from the model) to an LMS or platform item | `kind: amend` card, unless the note already holds the value | `kind: amend` card, unless already equal |
| `change` from an LMS template, `aligned` vendor sender, to an item a feed supplies | nothing: the feed is the authority and runs every slot, so a historical notice never overwrites its newer value; counted superseded | `kind: amend` card, unless already equal (`ingest` runs before `judge`, so a move the feed already carries is equal) |
| `change` from an LMS template to a note no feed supplies | **direct** | `kind: amend` card |
| `change` to any other note (a Gmail or events note) | `kind: amend` card | `kind: amend` card |
| `change` with `field: cancelled` | `kind: amend` card proposing `status: active → archived`; never a direct write | the same |
| a submission receipt (template) | `completion::propose_done`, which always proposes and asks once per task | the same, as T9 built it |
| two claims, same note, field and ordering date, different values | `kind: amend` card naming both | `kind: amend` card |
| unmatched after resolve | info notice | info notice |
| `course_info` | info notice, superseding by key (D13) | info notice |
| `noise` | nothing | nothing |

Every mail card carries `created_by: mail`, the sender and date (D21), the model's one-line `why` and
the evidence quote, and is written by `mail.rs`'s card writer (D11). Approving a mail **amend** card
writes its changed fields as the vault's human actor (D11); approving a mail **task** card materialises
the task as today, as `agent:approvals`.

### 4.10 The canary and "source went quiet"

- **The message.** From a Knowlu sender on the verified domain, through the provider that already sends
  Knowlu's account mail; subject "Knowlu delivery check — no need to open this", a nonce in a header and
  the subject; one sentence of body saying what it is and where to turn it off.
- **At setup:** always, per mailbox (§4.2 step 4).
- **After setup:** the mailbox is watched by `last_received_at`. A canary is sent **only after the
  mailbox has gone quiet** (no bound mail for 24 hours on a weekday, 48 over a weekend) and **at least
  72 hours after that mailbox's last canary** (§13 Q2), and never when the student has had any Knowlu
  email that day (VISION: one daily email at most).
- **Missing:** a canary not back within two hours sets the mailbox `quiet`; with the saved session on,
  the next slot's mail step checks the forwarding setting (§4.11): found off, it sets `off`, turns it
  back on and then says so (Q6, §13 Q11), unless the student turned that mailbox off in Knowlu's
  Settings; found pointing elsewhere, it changes nothing and asks the student. A setup canary that
  never arrives sets `blocked` (D16).
- **Limits:** `POST /knowbox/canary` sends only to a `mailbox_hint` whose domain fits the mailbox's
  provider (the campus row's `mail_domain` or the MX check for school mail; `gmail.com` or
  `googlemail.com` for Gmail), at most three canaries per address per UTC day, so the endpoint cannot
  make Knowlu's domain mail third parties at will.
- **Shown:** the mail pass reads each mailbox's state from the pull's answer and keeps one info item of
  kind `heads-up` per non-active mailbox (`close_key: mail-source:<address_id>`), closed when the mailbox
  is active again; the Settings row and the run line name the same state.

### 4.11 The saved session: reconcile, recover, re-enable (D15)

**An app step before the engine chain (F6).** The `mail` step is the app's, like the grades capture, and
runs where the capture runs: after the slot's entitlement step and after the grades capture, before the
child-process loop starts with `sync` (`scheduler.rs:845-922`). The engine never opens a window and the
step never writes the vault. Running first also gives mail it recovers a few minutes to reach
`mail_claims` before `judge` pulls; what has not arrived by then is pulled at the next slot. It adds one
`RunSummary` row named `mail`, or a named skip at exit 0: `mail (skipped: saved sign-in off)`,
`mail (skipped: not due)` (reconcile runs once a day, §13 Q3), `mail (skipped: no entitlement)` (the slot's existing
`est`). It has its own one-window lock; the grades capture and the mail step run one after the other,
never at once.

**An open backfill is not bound by the slot.** While a mailbox has an open backfill batch, the routine
runs on setup's background thread whenever the app is open (at launch, and again as its pace allows),
not only in a slot, and the 30-a-slot cap does not apply; Microsoft's pace (§4.2 step 5) is the only
limit. A school mailbox of a few hundred messages finishes in about an hour of open app; a larger one
takes days at the daily ceiling, and the progress view and the Settings row say so.

Per mailbox, with the mailbox's own kept profile:

1. Open the profile **hidden** at the mailbox (grades' `open_hidden` pattern) and check it is signed in.
   Signed out: the step records `mail (needs sign-in)` and stops. **No window is shown**; the student is
   asked to sign in only when a repair is needed, by a heads-up item and the Settings row, and signing
   in opens the visible window.
2. **Read the forwarding setting.**
   - On, to this mailbox's Knowbox address: nothing to do.
   - **Off, and the student turned this mailbox off in Knowlu's Settings** (its `address_id` is in
     `mail.json`'s `turned_off`, written first at §4.12): nothing is done, now or ever; the step skips a
     listed mailbox before step 1 opens its profile. Knowlu never re-enables what the student turned
     off in its own Settings (§13 Q11).
   - **Off, otherwise:** the mailbox is `off`; the step turns forwarding back on with setup's step
     script, automatically (Q6, §13 Q11), sends a canary, and reports it (`POST
     /knowbox/addresses/<id>/state {state: reenabled}`, which stamps `reenabled_at`). The next mail pass
     opens a heads-up item for that mailbox (`close_key: mail-reenabled:<address_id>`) whose text is
     exactly: "Email forwarding was off; Knowlu turned it back on. Turn off email in Settings to stop
     this." Its title names the mailbox (school email or Gmail); the student dismisses it.
   - **Pointing to any other address:** never changed in a hidden step (§4.2 step 3's rule, which needs
     the student's answer). The mailbox is `off` (reported `{state: elsewhere}`), and a heads-up item
     asks "Forwarding in your school mail now goes to another address. Forward to Knowlu instead?"; only
     a yes acts, through the visible progress view.
3. **Reconcile, on the device.** `POST /knowbox/reconcile {address_id, since, until}` answers the
   fingerprints the service **holds** for the window (each an `HMAC_k(sender, normalised subject)` with
   its received minute, §4.6 step 7) and the key `k`. The device lists what the mailbox received in the
   window (sender, subject, received time; T0 S2 records which form of the sender each provider's list
   shows, and intake fingerprints that same form), computes the same HMACs and diffs locally: a listed
   message is held when a held entry has the same HMAC and a received minute within 15 minutes. Nothing
   about a message the service does not hold leaves the device. The window is the last three days
   (§13 Q3), reaching back to `lapsed_since` after a lapse (§4.6 step 3) and, after forwarding was found
   off, to when it was last known on.
4. **Recover.** Open a `recover` batch and forward the missing messages as attachments, **oldest first**,
   at most 30 a slot when no backfill is open, at Microsoft's pace, deleting the Sent copies it created;
   close the batch with the count sent (§4.8 holds it until it is whole).
5. Close the window. The line: `mail: forwarding on, 2 recovered` (or what happened).

A backfill is this routine run over the term's window (§4.2 step 5), which is why an interrupted
backfill resumes by itself.

### 4.12 Disconnect

Settings → Email → a mailbox → **Turn off email** (the control the re-enable notice names, §4.11 step
2; called Disconnect below), two steps like *Delete my data*. The confirm says what happens: "Knowlu
turns off forwarding in this mailbox, stops reading it and deletes what it holds to serve it. What it
already added stays in your plan."

0. Record the mailbox's `address_id` in `mail.json`'s `turned_off`, before anything else, so no later
   step, slot or retry ever turns its forwarding back on, even if a step below fails (§13 Q11).
1. Turn forwarding off (and on Gmail, remove the address from the list; no re-authentication, X3b), with
   the saved session; if signed out, the visible window for a sign-in; if that is declined or a step
   fails, the guided steps, with the address shown so the student can find it.
2. `DELETE /knowbox/addresses/<id>`: revoke and purge (D17). Allowed for a lapsed or canceled account
   (sign-in only, §6.3).
3. Close the mail window and delete that mailbox's `mail-session-<address_id>` profile; a delete that
   fails is retried at the next start-up and named in Settings until it succeeds (D15).

### 4.13 Account deletion

`delete_my_data` gains one step before its server call: for each mailbox, try to turn forwarding off with
the saved session (hidden, bounded to a minute each). Then `DELETE /account` as today. Its purge now
calls `delete_knowbox_address` for each mailbox first (the `work/` objects deleted by the keys
`knowbox_work` holds, since the IAM user cannot list the bucket; an `inbound/` object lives only for one
intake call and its one-day rule catches a straggler), then names all seven Knowbox
tables in its list (`knowbox_accounts`, `knowbox_addresses`, `knowbox_batches`, `knowbox_canaries`,
`knowbox_seen`, `knowbox_work`, `mail_claims`) whether or not the cascade would reach them, as the list
does for every table that holds the student's data. `exportAll` gains the rows D17 names. Then the mail
windows are closed before `delete_local_data` removes the profile folder and every kept mail profile with
it. The confirm adds one sentence: "If Knowlu cannot turn forwarding off, your mail keeps going to
Knowlu's address and is deleted unread; you can turn it off in your mail settings."

**Knowlu sign-out** (`account::sign_out`) also closes the mail window and deletes every
`mail-session-*` profile and any `mail-setup` profile of that Knowlu profile: a signed-out Knowlu keeps
no signed-in mailbox. Forwarding is untouched; signing back in asks for the mail sign-in only when a
repair is needed.

### 4.14 "From your email" and Undo (D10)

A view, opened from the backfill's last progress row and from Settings → Email, lists what the backfill
applied, by course: items created (title, due), fields changed (old → new, sender, date), past items
archived (a count that expands), notices opened. Each row has **Undo**:

- a created item: `delete_note`; its uid stays in `state/ingest-seen.md`, so it is never re-created;
- a changed field: `set_fields` with the journal's `old` value, as the student;
- a notice: `close_info`.

A row whose note has changed since (or is gone) shows "changed since" and no Undo. The list comes from a
read-only engine command (§6.1); the app computes nothing.

## 5. Data model

### 5.1 Cloud

One new migration, `<date>_knowbox.sql`, stamped after every existing one; no applied migration is
edited. Every table has RLS on and no grant to `anon` or `authenticated`: only the service role reads
them, as `google_accounts` does. Every `account_id` references `accounts(id) on delete cascade`.

| Table | Holds | Kept |
|---|---|---|
| `knowbox_accounts` | `account_id`; the per-account fingerprint key, encrypted; `digest` (D13, default false); the allowance (D23): `allowance_term_start`, `allowance_left`, `allowance_usd`, `allowance_until`; `mints_30d` (D3) | until the account goes |
| `knowbox_addresses` | `id`; `provider` (`m365`, `google`, `other`); `token_hmac` (unique) and the token encrypted with its IV; `retired_token_hmac` and `retired_until` (Rotate's grace, D3); `mailbox_hint`; `forward_hint` (§4.6 step 5); `status` (`pending`, `confirmed`, `active`, `quiet`, `blocked`, `off`, `paused_send`, `lapsed`, `revoked`); `lapsed_since`; `reenabled_at` (§4.11 step 2); `confirm_until`; `term_start`; `last_received_at`, `last_canary_sent_at`, `last_canary_ok_at`, `canaries_today`; counters `unbound`, `unauthenticated`, `expired` | until Disconnect (then the row is deleted) or the account goes |
| `knowbox_batches` | `id`, `address_id`, `phase` (`backfill`, `recover`), the nonce's HMAC, `since`, `expected`, `sent` (the device's count at close), `received`, `opened_at`, `sent_at`, `closed_at` | 30 days after closing |
| `knowbox_canaries` | `id`, `address_id`, the nonce's HMAC, `sent_at`, `arrived_at` | 30 days |
| `knowbox_seen` | `account_id`, `msg_hash` and `fp_hash` (both HMACs under the account's key, §4.6 step 7), `received_minute`, `address_id`, `phase`, `seen_at`; primary key `(account_id, msg_hash)`; written with its `knowbox_work` row, deleted if that item expires | 180 days, pruned nightly (a term and its overlap) |
| `knowbox_work` | `id`, `account_id`, `address_id`, `batch_id`, the S3 key, `phase`, the authenticity word, `arrived_at`, `attempts`, `next_attempt_at` | until processed, or until its object expires (then counted `expired`) |
| `mail_claims` | `id`, `account_id`, `address_id`, `batch_id`, `uid`, `message_date`, `ordering_date`, `sender_name`, `sender_addr`, `phase`, `authenticity`, `claims` (jsonb, below), `judgment_id`, `queued_at`, `delivered_at` | delivered: 7 days; undelivered: 30 days, then deleted and counted; all of a mailbox's rows on its Disconnect |

**Changes to existing objects, in the same migration, constraint by constraint** (§1's list; each is
pinned by test 14):
- `judgments.origin` gains `knowbox`; `judgments.kind` (`:70`) gains `mail_claim`, `mail_resolve`,
  `mail_digest`.
- `models.kind` (`:39`) and `usage_daily.kind` (`:155`) gain the same three, so `models` takes their rows
  and `charge_call` and `record_tokens` can count them. Without `usage_daily`, `charge_call` answers
  23514, `capStore.charge` reads it as capped, and every claim would be `capped`.
- **Not widened, deliberately:** `corrections.judgment_kind` (`:102-103`), `rules.kind` (`:113`), both
  eval tables (`20260911000700_eval.sql:9`, `:31`) and `telemetry/handler.ts`'s `JUDGMENT_KINDS`. Mail
  judgments never become rules, corrections or eval rows: `promote_rules` and
  `backfill_correction_judgments` are redefined to read only `origin <> 'knowbox'` (otherwise
  `judgment_features` would offer a `mail_claim` feature and the `rules` insert would abort the whole
  nightly promotion for every account), and the device never reports a label on a mail card
  (`labels_to_report` already passes over every kind but `task` and `event`; test 28b pins it).
- `export_training_rows` is redefined to exclude `knowbox` beside `gmail_api` and `events`; the
  assertion at `migrations_test.ts:826` moves to the new last definition and checks
  `origin not in ('gmail_api', 'events', 'knowbox')`, naming all three origins, so it is stronger, never
  looser.
- `enforce_budget` takes D23's raised ceiling while an account's allowance is open; `DAILY_CAP` gains
  `mail_claim` and `mail_resolve` (the allowance is 2,000 judgments and $1.00 per account per term, §13
  Q1; the daily caps start at email's 120 for `mail_claim` and 20 for `mail_resolve`).
- RPCs: `charge_allowance(p_account)` (one statement, like `charge_call`); `delete_knowbox_address(p_address)`
  (revoke, then delete the mailbox's `knowbox_seen`, `knowbox_work`, all its `mail_claims`, batches
  and canaries, then the row; `security definer`, pinned `search_path`). Its callers (Disconnect and
  the account purge) first read the mailbox's `work/` keys from `knowbox_work` and delete those objects.
- Cron (pg_net, the Vault token, as the existing jobs): `knowbox-process` every minute;
  `knowbox-canary` hourly; `knowbox-sweep` nightly (expire work and delete the expired items' seen rows,
  close timed-out batches, prune seen, sweep claims, batches and canaries).

**A claim** (the `claims` array's element; validated server-side, re-checked for shape on the device):

```
{ kind: new_item | change | course_info | noise,
  course: <known course code> | null,
  target: <the item as the mail names it> | null,          // change
  field: due | cancelled | null,                            // change; cancelled is always a card (§4.9)
  value: <resolved due, wire shape> | "true" | null,        // change
  item: { title, due | null, item_kind: task | exam | event,
          effort_hours: 0.25..40, importance: 1..5 } | null,  // new_item (§5.2: no device re-judgment)
  topic: schedule | room | instructor | office_hours | policy | other | null, // course_info
  evidence: <verbatim, at most 140 characters>,
  why: <one line, at most 140 characters>,
  confidence: 0..1,
  source: template | model,
  resolution: { target_id: <note id> | null } | null }      // written by mail-resolve
```

**AWS** (Quinn's account; a runbook in `cloud/supabase/README.md`, no secret in the repo): an SES domain
identity and the subdomain's MX in Cloudflare DNS; a receipt rule for the subdomain whose **only action
is the S3 action with its `TopicArn` set** (never SES's separate SNS action, which would publish the
whole message through SNS; never "stop" or "bounce"); an S3 bucket with public access blocked,
encryption at rest, and a one-day expiry on `inbound/` and `work/` (up to about two days in practice,
§7); an SNS topic with one HTTPS subscription; one IAM user limited to get, put and delete on that
bucket (no list). Function secrets, named only:
`KNOWBOX_DOMAIN`, `KNOWBOX_ADDRESS_KEY`, `KNOWBOX_ENC_KEY`, `KNOWBOX_BUCKET`, `KNOWBOX_SNS_TOPIC_ARN`,
`AWS_REGION`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`. `config.toml` sets `verify_jwt = false` for
`knowbox-inbound` only (SNS cannot send a bearer; the signature is its check).

### 5.2 Vault

No new folder, no new local card kind, no change to an existing field's meaning.

- **A task created from mail** has the shape of `enrich.rs`'s Gmail note (`GMAIL_NOTE`, `:526-529`)
  with these differences: `created_by: mail`, `source_uid: mail:<h>:<n>`, `judgment_id`,
  `judgment_kind: mail_claim`, `effort_hours` and `importance` from the claim, and **`needs_enrichment:
  false`**, exactly as a Gmail note is written (`:518-525`): the device's task pass never sends a mail
  note's title or body to `judge-task`, so no `device`-origin judgment is ever made from mail text and
  nothing mail-derived reaches the training export, the rules or the corrections back-fill (test 28c).
  When `due` came from mail it also carries `mail_asof: {due: '<ordering date>'}`. A past-due item is
  created `status: archived`, `archived_reason: imported-past`.
- **`mail_asof`** is a new single-line flow mapping, field → the ordering date (§4.8) of the last mail
  value applied to that field, written through `write` like `judgment:`. It is how a later claim supersedes
  an earlier one across pulls (D10). Additive; no existing note carries it.
- **Each direct change** sets the field and `mail_asof`, and appends one body line:
  `<date> · <sender>: <why> — "<evidence>"`. The note itself then carries its own history in text.
- **Cards** are today's shapes: `type: approval`, `kind: task` with the fenced task payload
  (`write_gmail_card`'s shape) or `kind: amend` with `target` and `changes`; plus `created_by: mail`,
  `source_uid`, `judgment_id`, `judgment_kind: mail_claim`, and three additive fields: `from`,
  `mail_date`, `evidence`. A `cancelled` claim's card is `kind: amend` with `changes: {status: {from:
  active, to: archived}}`; `status` is amendable (`approvals.rs:43-54`), and no new field is added.
- **Every mail-derived string is untrusted text.** `from`, `why`, `evidence` and a mail title are written
  through `write::to_literal` and rendered in the console only through `h()`; a static test puts
  `<img src=x onerror=…>` in `evidence` and checks it renders inert (§12.4).
- **Notices** are `info/` items (`type: info`): kind `notice` with `close_key: mail:<course>:<topic>`, or
  kind `heads-up` with `close_key: mail-source:<address_id>`; `opened_by: agent:knowlu.mail`; the body is
  the same one-line shape.
- **`state/ingest-seen.md`** gains `mail:<h>` lines, like `gmail:`.

### 5.3 Journal

Actor **`agent:knowlu.mail`** for every write the mail pass makes, `via` the slot's (`local-runner`).
Undo writes with the student's context (`console_ctx`, `dashboard`). Approving a mail card follows D11:
an approved **amend** card's changed fields are written as the vault's human actor
(`journal::read_human_actor`; a bad `config/actor.yaml` stops it with ruling 11's error), while an
approved **task** card, and the card's own status bookkeeping, are executed as today's executor,
`agent:approvals`. No new `via`.

### 5.4 App data (the profile's data folder, never the vault, never synced)

- `<data_dir>\mail-session-<address_id>\`: one kept WebView2 profile per mailbox (D15), each behind
  the `is_session_dir` guard. With the switch off, setup uses `<data_dir>\mail-setup\` and wipes it when
  the window closes; start-up sweeps a `mail-setup` a crash left, and retries any profile delete that
  failed before.
- `<data_dir>\mail.json`: `{keep_session: true, last_reconcile: {<address_id>: <ts>}, pending_deletes:
  […], turned_off: [<address_id>, …]}`, a file beside `settings.json` and never a field in it (the
  precedent of `grades.json`, `grades.rs:59-63`). `turned_off` only grows; the mail step never
  re-enables a listed mailbox (§4.11 step 2, §4.12 step 0).
- **One window label, `mail`** (D5 and every section use it), visible or hidden, with no capability
  grant; one mail window at a time.
- No new Credential Manager target. Knowlu writes the address to no file and no log of its own; it
  asks the service for it when a step needs it (the mailbox's own settings hold it, by design).

## 6. Surfaces and commands

### 6.1 Engine (`knowlu-engine`)

- **New `engine/src/mailreconcile.rs`, pure.** Input: a view of the vault's notes (id, path, course,
  title, due, status, `source_uid`, `created_by`, `mail_asof`, and, per field it may touch, whether
  `Journal::human_set` finds a record), the vault day and the ordered messages with their claims, phase
  and authenticity word. Output: an ordered list of actions (create, set, archive, card, notice, skip
  with reason). It implements §4.8 steps 2, 4 and 5 and §4.9's table, and is the only place card or
  direct is decided (D11). No file, clock or network access: `today` is a parameter.
- **New `engine/src/mail.rs`.** `pull_mail(vault, client, opts, budget) -> Vec<String>`: the pull, the
  resolve batch, the journal reads that build `mailreconcile`'s view, the call into it, the writes
  through `write` (`write::create` for notes and for cards, through its own card writer that checks
  `write::find_pending_amendment` first; `write_literals` with `WriteOpts::default()` for direct sets;
  `info::open_info` / `close_info` for notices; `completion::propose_done` for receipts), the ack and
  the line. One call from `enrich::run_lines_with`, after `pull_gmail`; that hunk is the only edit to
  `enrich.rs`.
- **`engine/src/cloudmodel.rs`:** `pull_mail_claims(client, ack)` and `resolve_mail(client, batch)`, with
  every failure shape a named outcome (no session, 402, 429, 5xx, timeout), as `pull_gmail_queue` has.
- **New read-only command `knowlu-engine mail-applied --vault <v> [--since <ts>]`.** Prints JSON: every
  journal record by `agent:knowlu.mail` since the backfill began, joined to the note's current value
  (§4.14). It never writes, and is never entitlement-gated (like `surface`).
- **`engine/src/ingest.rs` (§13 Q7: ruling 3 now):** in `sync_tasks`' update branch
  (`ingest.rs:676-703`), for `title` and `due`, when `Journal::human_edited(id, field)` finds a record and
  the feed differs, file one `kind: amend` card instead of writing; none while
  `write::find_pending_amendment` finds one; none for a value the student already rejected on a card for
  that note and field (until the feed's value changes); a field no human edited is written exactly as
  today. The card goes through `write::create` in the shape `approvals::validate_amendment` accepts, as
  the ingest actor, with its own "Why proposed" line ("Blackboard now says <to>; you set <from> by hand,
  so this is a proposal"), **never through `write::propose_amendment`**, whose "re-judged … (judge-once
  rule)" line would be false for a feed change (R20's reason, applied here). The run line says
  `proposed <stem>: due`. Approving it writes as today's executor; the field stays human-edited in the
  journal, so a later feed move is a card again.
- **Unchanged:** `rank` (it never calls a model and never sees a claim), `surface`, `write`, the `gmail`
  pass, every frozen reference. `judge` still exits 0 on every failure shape.

### 6.2 App (`knowlu`)

- **New `app/src/mail.rs`.** Pure pieces: the provider's step list and its state machine, the term-start
  default (D7), the `mail-session-<address_id>` directories and their guard (grades' `is_session_dir`
  rule), `mail.json`, the origin allow-list per provider (D5), the pace and daily ceiling (§4.2 step 5).
  The window: `open_visible`, `open_hidden`, the sign-in detector, the steps-aside move, the popup and
  protocol-prompt handling (T0 S1, S3), the script runner, which refuses to run a script unless the
  page's origin is on the list (T0 S1), `forget` (per mailbox, retried at start-up when it fails), and
  the start-up sweep of `mail-setup`. The step scripts live in
  `app/assets/mail/<provider>/<step>.js` (D19), read at build time.
- **Console commands** (all in the console window's `generate_handler!` list; none in the wizard's):

  | Command | Does |
  |---|---|
  | `mail_status` | the mailboxes (from `GET /knowbox/addresses`), each state, backfill progress, `keep_session` |
  | `mail_setup_start(provider)` | starts setup on a background thread; one at a time |
  | `mail_setup_progress` | the step rows for the progress view (polled) |
  | `mail_setup_answer(choice)` | a needs-you answer: replace forwarding, stop, the term date |
  | `mail_setup_stop` | stop after the current step |
  | `mail_sign_in(address_id)` | the visible window, for a repair |
  | `mail_disconnect(address_id)` | §4.12 |
  | `mail_rotate(address_id)` | a new address, then the forwarding step with it |
  | `mail_keep_session(on)` | the switch; off deletes every `mail-session-*` profile |
  | `mail_applied` | runs `knowlu-engine mail-applied`, passes the JSON through |

  Ten commands; `main.rs` is single-owner, so the names are a controller hand-off (§14). Recount at merge.
- **`scheduler.rs`:** the `mail` step (§4.11) in `run_slot_with` after the grades capture and before
  the child-process loop, its `RunSummary` row and named skips; `slot_argv` is unchanged. The open
  backfill's runner is setup's background thread, not the scheduler.
- **`account.rs`:** `delete_my_data`'s pre-step and `sign_out`'s profile deletion (§4.13). Contract
  list (§8).
- **The wizard:** the announcement on the panel after sign-in and subscription (D6, §13 Q9: "Next:
  connect your school email") writes a marker in the profile's data folder (the
  `onboarding::offer_marker` precedent) unless the student chose Not now; the console's first open reads
  it, opens the school mail sign-in window and shows the progress view.
- **`app/src/grades.rs` (D25, T19):** a new pure `availability_with(gate, row, lms)` holds the predicate,
  with the one branch the gate decides; `availability(row, lms)` keeps its signature and returns
  `availability_with(POLICY_READ_GATE, row, lms)`. While the constant is `Suspended`, a curated
  `blackboard` row is `Available` with or without `policy_read`. Its four callers (`grades_status`'s
  seam, `connect_with`, `refresh_with`, `scheduler::grades_step`) are unchanged, take no gate parameter
  and still never re-derive it, so they always see `Suspended`: the `Enforced` arm is tested only at
  `availability_with` (test 39), and the caller tests' undated cases change to the `Suspended`
  expectation (tests 40–41). Nothing else in M1 changes.
- **`app/static`:** the progress view, Settings → Email (one row per mailbox: state, address with Copy,
  Resume, Rotate, Disconnect, the saved-sign-in switch), "From your email", the wizard's offer, and the
  card's `from`, `mail_date` and `evidence` on the deck.

**The engine chain is unchanged:** `sync → coursework → ingest → grades → judge → rank`. Before it, the
app runs its own window steps: the grades capture, then the saved-session `mail` step. Neither writes
the vault. CLAUDE.md's slot line gains that sentence (§10, A12).

### 6.3 Cloud (`cloud/supabase/functions/`)

| Function | Auth | Does |
|---|---|---|
| `knowbox-inbound` | SNS signature (`verify_jwt = false`) | §4.6 |
| `knowbox-process` | the cron's Vault token | §4.7 |
| `knowbox` | **sign-in only** (`requireUser`, as google-connect's status and disconnect) for `GET /addresses` and `DELETE /addresses/<id>`, so a lapsed or canceled account can always see and disconnect its mailboxes; **session + `requireActiveEntitlement`** for every other route | `GET /addresses`, `DELETE /addresses/<id>`, `POST /addresses` (D3's caps), `POST /addresses/<id>/rotate`, `POST /canary` (§4.10's limits), `POST /batches` (opens a backfill or recover batch and answers its nonce), `POST /batches/<id>/sent`, `POST /reconcile` (answers held fingerprints and the key, §4.11), `POST /addresses/<id>/state` (`off`, `elsewhere` or `reenabled` from the mail step, §4.11 step 2), `GET /provider?domain=` (MX lookup through `Deno.resolveDns`; §13 Q4) |
| `knowbox-canary` | the cron's Vault token | §4.10's periodic canary |
| `mail-pull` | session + entitlement | undelivered `mail_claims` by §4.8 step 1's rules; ack |
| `mail-resolve` | session + entitlement | D12; one call per course batch; stores each answer on its claim |
| `account` (changed) | as today | the purge of §4.13 (each mailbox by `delete_knowbox_address`, objects by key first; all seven tables named); `exportAll` gains D17's rows |
| `gmail-read` (changed) | as today | D22's non-quiet empty read |

Every function follows the house rules: no body, header or address token in any log line; errors name a
class and a status only (`gmail-read/handler.ts:379`).

## 7. Privacy

**What changes for the student, said plainly.** Every email that reaches a connected mailbox is forwarded
to Knowlu, whole: school mail, personal mail, everything (Q4). It passes through Amazon's mail service;
Amazon's notification to us carries its headers (sender, recipients, subject), never its text. It waits
in Amazon's storage until Knowlu has read it, and is deleted then; anything left over is removed by
Amazon's one-day expiry, which in practice means **up to about two days, plus Amazon's own lag**. It is
read in our server's memory. **Only bulk and mailing-list mail from senders outside school and the LMS
is dropped without a model reading it.** Every other message, including personal mail from a friend, a
doctor or a bank, has its subject, sender, date and up to the first 2,000 characters of its text read
once by the pinned model host, which keeps none of it, so that Knowlu can tell whether it is about
school. What Knowlu keeps is what it found about school, not the email.

| Data | Where | How long |
|---|---|---|
| The whole message | AWS (SES, S3), then our function's memory | deleted when read; otherwise up to about two days in S3, plus AWS's expiry lag |
| The message's headers (sender, recipients, subject) | AWS's notification to our function (SNS) | for the length of that call; never logged |
| Attachments | AWS, never opened (a backfill's attached originals are the one exception: each is read as an email) | as above |
| The subject, sender, date and up to 2,000 characters of every message the screen does not drop, personal mail included | the pinned model host, one call | zero retention, as today |
| Course item titles and due dates (resolve) | the pinned reasoning model host, one call | zero retention |
| Your own mailbox's address (`mailbox_hint`), so mail can be matched to it | our database | until Disconnect or deletion |
| A keyed hash of each Message-ID and of (sender, subject), with the minute it arrived | our database | 180 days |
| What was found: the model's one-line reason, a quote of at most 140 characters, the sender's name and address, the date | our database until your computer takes it; then your vault, which the account holds | 7 days after delivery (30 if never delivered); in the vault until you delete it |
| The judgment record (no text) | our database | until the account is deleted, as today |
| The forwarding address | our database, encrypted | until Disconnect or deletion |
| The saved mail sign-in, one per mailbox | your computer only, in Knowlu's app folder | until you turn it off, disconnect that mailbox, sign out of Knowlu, or delete your account |

**What Knowlu does inside the student's mailbox,** with them present at setup or, later, with the saved
sign-in they allowed: turns forwarding on (and off at Disconnect); forwards older mail to Knowlu's address
as attachments and deletes the Sent copies those forwards created; turns forwarding back on if it was
turned off, and says so. It never changes forwarding that points somewhere else without asking. Nothing
else is changed, sent or deleted. Knowlu also sends the mailbox a delivery check now and then (§4.10).
None of this produces anything sent to another person (VISION commitment 3).

**The saved sign-in** is full access to that mailbox on this computer. Windows encrypts its sign-in
cookies for the student's Windows account; the pages and mail the mail site keeps in its own cache are
stored in the same folder without that encryption. A program running as the student could use either.
It is on by default (Q6) and the switch is in Settings → Email.

**Kept sign-ins at every school, for now (D25, A13).** Until Knowlu has 20 paying users, two kept
sign-ins are on at every school, and **no school's policy on them has been read first**: the saved
mail sign-in, at every school, and the kept Blackboard sign-in for grades, at every school Knowlu has a
curated row for (an uncurated school still has no grades). Before this, ruling 12 promised the released
app would never offer or use the Blackboard sign-in at a school without a recorded read. That promise
is suspended, not withdrawn: Quinn rules on it again when the 20th paying account exists. The page says
so in plain words, and no release carries either sign-in before it does (D18).

**Turning it off.** If forwarding is found off, Knowlu turns it back on and then says so: "Email
forwarding was off; Knowlu turned it back on. Turn off email in Settings to stop this." A mailbox
turned off in Knowlu's own Settings is never turned back on (§13 Q11).

**Telemetry and training.** No Knowbox content in telemetry, ever; no new telemetry row shape. `knowbox`
judgments are excluded from the training export, the rules and the corrections back-fill (§5.1), and a
note made from mail is never re-judged on the device (§5.2), so no judgment of any origin carries mail
text into training. Google's Limited Use policy does not bind mail
that arrives by forwarding, since it is not obtained through Google's API; the Knowbox is held to the
same rules anyway.

**Sub-processor:** Amazon Web Services (SES, S3, SNS) joins the list, "receives the email you forward to
Knowlu and holds it until Knowlu has read it, at most about two days".

**Where these sentences go (D18).** Nowhere on the site from this lane. They join privacy bump #1's list
in HANDOFF (T13), beside M1's and the Gmail spec's drafts, for its one lawyer read:
1. `privacy.html:56`: "…and up to the first 1,200 characters of the message" gains "(2,000 for email
   you forward to Knowlu)".
2. A new section, *Email you forward to Knowlu*, after the Gmail section: the paragraph that opens this
   section, the table's rows as prose, the mailbox actions, the canary and the saved sign-in.
3. "What we collect" gains *Email you forward to Knowlu, if you set it up*, and *Your forwarding
   addresses*.
4. "How long we keep it" gains the Knowbox's bullets: until read, at most about two days in Amazon's
   storage (plus its expiry lag), 180 days for the keyed hashes, the mailbox's own address until
   Disconnect, 7 or 30 days for what was found before it reaches the vault.
5. The sub-processor line for AWS.
6. The kept sign-ins paragraph that ruling 12 adds for Blackboard names the mail sign-in too, and says
   that neither waits on a school's policy read until Knowlu has 20 paying users (D25).
7. `:85` stays true of Gmail; the new section says what is kept from forwarded mail (the quote).

The lawyer packet gains: the Knowbox and AWS; the mailbox actions done with a saved session (Microsoft's
and Google's terms on automated access, which the research flagged); the saved mail sign-in; and the
suspended university-policy reads (D25): both kept sign-ins on at every school, no school's policy read,
until 20 paying users. **`PRIVACY_VERSION` does not move here**, and the release guard of
D18 is the line T13 adds to HANDOFF's Pilot gate.

## 8. Contract-list impact

| Contract | Impact |
|---|---|
| `engine/src/approvals.rs` | **Changed (D11):** a `kind: amend` card with `created_by: mail`, when approved, applies its changes with the vault's human actor (`journal::read_human_actor`), not the executor's; when that read fails, the card stays pending and the run names ruling 11's error, never another actor. Every other card kind and every other amend executes exactly as today. contract-engineer, reviewed by contract-reviewer. |
| `engine/src/ingest.rs` (not on the list) | **Changed (§13 Q7; §6.1):** ruling 3's amend card for a hand-edited `title` or `due`, written through `write::create`, not `propose_amendment`. It changes behaviour in every existing vault, so contract-engineer, reviewed by contract-reviewer; the oracle and golden references must stay byte-identical (no fixture vault holds a hand-set LMS `due` that the feed contradicts; T8b's first step checks that before any code). |
| `app/src/grades.rs` (not on the list) | **Changed (D25, T19):** the `POLICY_READ_GATE` constant, the pure `availability_with` seam with the one branch the gate decides, and `availability` delegating to it; its callers unchanged. No contract-list file, test or fixture changes; the tests that pinned "an undated Blackboard row is not available" change with the ruling, each named in T19. |
| `app/src/account.rs` | **Changed (§4.13):** `delete_my_data`'s forwarding-off pre-step, which calls into `mail.rs` and never blocks the deletion, and `sign_out`'s deletion of the mail profiles. `PRIVACY_VERSION` is not touched (D18). contract-engineer. |
| `write.rs`, `journal.rs` (`VIAS`), `yamlemit.rs`, `yaml.rs`, `pystr.rs`, `ledger.rs`, `ids.rs`, `provenance.rs` | **Used, not changed.** Every write goes through `write`; the new actor starts `agent:`; no new `via`; no new id kind (`task_` and `info_` exist). `JUDGED_FIELDS_TASK` is **not** widened to `due`: that would change judge-once in every vault, and the reconciler's own journal check (D11) makes it unnecessary. `mail_asof` is written the way `judgment:` is; if that needs a `write` mode that does not exist, the task stops and goes to contract-engineer. |
| `sync.rs` | **Not changed.** No new note folder, so the two tripwires stay true; mail cards are not local kinds and sync like Gmail cards; `mail_asof` is note text. |
| `entitle.rs` | **Not changed.** `judge` is already gated, and the mail pass is inside it. The app's `mail` step checks the slot's entitlement state. |
| `wincred.rs`, `reconcile.rs`, `app/src/{credentials,updates}.rs` | Not touched. No new credential. |
| The oracle, sync and entitlement tests | **Unchanged, and must stay green.** `rank` and `surface` see no new input on a vault without mail notes. |
| `engine/tests/fixtures/**` | **Not touched.** Every engine test builds its vault in a temporary directory; every test message is synthetic. |
| Note frontmatter | Additive only: `mail_asof` on tasks; `from`, `mail_date`, `evidence` on cards. No existing note is rewritten. |
| `cloud/supabase/migrations/migrations_test.ts` | The export-filter assertion (`:826`) moves to the new definition and names all three origins (stronger, never looser); new assertions pin RLS and grants on every Knowbox table and each constraint decision of §5.1. |

**Frozen references:** none is regenerated, and none may be. No fixture vault holds a mail note, so
`golden-today-*.md` and the three `surface-today-*.json` stay byte-identical.

## 9. The Gmail OAuth lane, and other lanes

- **Gmail OAuth (PR #26, signed spec, merged at `199cd1f`).** It stays as the direct path. This lane
  shares `enrich.rs` (one call), the console's handler list, Settings in `app/static`, and HANDOFF's
  bump-#1 list. D22 is one small change to the merged `gmail-read`, a non-quiet empty read, because the
  merged device reads any unknown quiet reason as "re-connect from settings". The wizard keeps #26's
  **Connect Gmail** button on the Gmail panel as "Connect Gmail directly (Google testers only)"; the
  Knowbox announcement is its own panel after sign-in (D6). Settings shows the Google row (#26) and the
  Email row (this) side by side. The release guard is one rule for both. #26's D15 (no sender) and this
  spec's D21 differ, and Quinn chose that (§13 Q10).
- **Events.** No shared behaviour: mail events are task-shaped cards (D24). `approvals.rs` is shared with
  the events lane's T3: whichever merges second rebases its arm, and D11's change is a separate arm.
  `ingest.rs`'s change (§13 Q7) touches only `sync_tasks`' update branch.
- **M2 editing.** This lane only appends body lines, a primitive that exists. M2's body replacement does
  not affect it.
- **M1 grades.** The kept-session pattern is reused, not shared: each `mail-session-<address_id>` is its
  own profile, with its own window label and its own one-at-a-time lock; the slot runs the grades capture
  and then the `mail` step, both before the engine chain. The registrar never shares any of these
  profiles. **One change reaches M1:** D25 suspends its policy-read gate (T19, `app/src/grades.rs`), so
  the founder's grades proof no longer needs UA's read or a proof branch carrying UA's date; it runs on
  a dev build of `main` once T19 merges (T19 may merge ahead of the rest of this lane, §14).
- **Two desktops (Launch).** `mail_claims` are pulled once per account; until the fetch-turn lease, two
  awake desktops can both pull. A claim applied on one desktop is a no-op on the other once its note has
  synced (§4.8 step 4: the synced `source_uid` is checked before any create), as well as being bounded by
  the server's ack. The lease must cover the mail pass and the `mail` step; setup and the saved session
  belong to one desktop. A hand-off to that spec at its next revision.
- **C5, the relay (Launch).** The step scripts are client-side in the MVP (D19). If C5's principle (fetch
  sequences in the cloud, the client a relay) is extended to mail, the scripts move behind signed
  delivery; not designed here.
- **Stream J.** `lms_receipts.ts` is used as it is; the new notification templates live in a new
  `_shared/mail_templates.ts`.

## 10. The amendment Quinn signs

Signing this spec signs the text below. The signing commit appends the ruling to the cloud design as
**"Amendment 2026-09-30 — the Knowbox"**, adds the markers, and makes the VISION, Gmail-spec,
grades-spec and CLAUDE.md edits in the same commit, as the earlier amendments did. Each item quotes the
current text.

**The ruling (appended to the cloud design):**

> **Email has two paths (Quinn's rulings of 2026-09-30).** (1) **The Knowbox:** each connected mailbox
> forwards everything to a private Knowlu address; AWS SES receives it, S3 holds it until it is read
> (its one-day rule removes anything left within about two days), and our service screens it, extracts
> claims and queues them; it never rejects mail at the door for SPF, DKIM or DMARC. Knowlu sets the
> forwarding up itself in a sign-in window the student sees only to sign in, for school Microsoft 365
> and personal Gmail, with guided steps for every other case; it backfills to the term's start,
> applies that backfill and lists it with Undo (proposing only what conflicts), proposes every change
> and new item after setup, and watches each mailbox with a canary. Mail cards, notices and applied
> changes name their sender and date, unlike Gmail cards in the MVP (the Gmail spec's D15). (2) **Gmail
> OAuth** stays the direct path for testers, and for everyone after CASA; a Gmail address with an
> active Knowbox is read by the Knowbox only. **A second named exception to §11a's "keep nothing"**
> joins ruling 12's: a saved mail sign-in, one WebView2 profile per connected mailbox under the app's
> data folder, never in the vault, never synced, never sent, its cookies DPAPI-encrypted, no password
> kept; **on by default for everyone, at every school, with no university-policy read until Knowlu
> has 20 paying users (A13)**, with a switch in Settings; used only to reconcile, recover and re-enable
> forwarding (re-enabled automatically and then announced, never for a mailbox the student turned off
> in Knowlu's Settings); deleted when switched off, on that mailbox's Disconnect, on Knowlu sign-out
> and on account deletion; named on the privacy page in bump #1. Ruling 10's MVP gains email by the
> Knowbox, including its own "source went quiet", and its exit gains the Knowbox's live proof. No
> release carries it before privacy bump #1.

**A1. D12** (`cloud-design:24`). Now: "**Email ingestion is Gmail OAuth (`gmail.readonly`),
server-side.** Quinn chose it over forwarding on 2026-09-09 knowing the cost…". Gains the marker
*(amended 2026-09-30: two paths, the Knowbox and Gmail OAuth; see the amendment at the end)*.

**A2. §13** (`:381`). Now: "email ingestion by forwarding (a Cloudflare Email Routing inbox — documented
as the fallback if Google refuses verification, not built)". Becomes: "email ingestion by forwarding
*(decided: Amendment 2026-09-30; the Knowbox, on AWS SES, because Cloudflare Email Routing rejects DMARC
failures)*".

**A3. §3.1** (`:71`). After the row "| Gmail read (OAuth) | **cloud** (`/gmail/*`) | D12; …|", a new row:
"| Email by forwarding (the Knowbox) | **cloud** receives, screens and judges; **device** sets up
forwarding and, with the saved sign-in, reconciles | Amendment 2026-09-30 |".

**A4. §5.3's campus line** (`:164`). Now: "**Campus accounts:** a Workspace admin can block third-party
apps for `@crimson.ua.edu`; the wizard recommends the personal account (which is where Crimson mail
already lands for anyone who forwards it)…". Gains *(amended 2026-09-30: the pilot schools' student mail
is Microsoft 365, read through the Knowbox; see the amendment)*.

**A5. Ruling 10.** The MVP list (`:596-601`) now reads "It holds commitment-model phases 1–2, M1 grades,
Gmail connect in the app, events (…), body and profile editing, each small parity item done or cut, and
UA's university-policy read (ruling 12)." It gains, after "Gmail connect in the app": "email by the
Knowbox (the receiver, automatic setup for school Microsoft 365 and personal Gmail, backfill to the
term's start, the canary and "source went quiet" for Knowbox mailboxes, and the saved sign-in's
reconcile and recover)". Its *Exit* (`:601-605`) gains, after the founder's grades proof: "the
founder's Knowbox proof (the email spec's §12.6) has passed on a dev build against staging;". The
Pilot list's "the OpenRouter 402 and "source went quiet" shown as issues" becomes "…and Gmail's "source
went quiet" shown as issues (the Knowbox's is the MVP's)". **Ruling 9** (`:589`): "The MVP is the
founder's quinn-ops feature set plus grades from Blackboard" becomes "…plus grades from Blackboard and
email by the Knowbox". Marker on both: *(amended 2026-09-30, the Knowbox)*.

**A6. The Gmail spec, §5** (`gmail-connect-design:430-431`). Now: "**Before Launch, nothing else is
used.** There is no email forwarding inbox (cloud design §13's fallback, not built), no `gmail.metadata`
and no local reading." Becomes: "**Before Launch, nothing else is used for Gmail by OAuth:** no
`gmail.metadata` and no local reading. Forwarded mail is the Knowbox's
(`2026-09-30-email-knowbox-design.md`), a separate path; a Gmail address with an active Knowbox is not
read here (that spec's D22)."

**A7. VISION, "Where it gets its information"** (`VISION.md:89`). Now: "- **Calendars and email**:
Google Calendar and Gmail, and Outlook / Microsoft 365, read-only." Becomes: "- **Calendars**: Google
Calendar and Outlook / Microsoft 365, read-only. **Email**: any mailbox that can forward, through the
student's own private Knowlu address (school Microsoft 365 and personal Gmail set up automatically,
others with guided steps), and Gmail directly by Google's permission."

**A8. VISION, "Knowlu reads"** (`:165-166`). Now: "- **Knowlu reads.** Its one planned write outside the
vault is its own dedicated calendar, behind a scope the student grants for that purpose." Becomes: "-
**Knowlu reads.** Outside the vault it writes only its own dedicated calendar, behind a scope the
student grants for that purpose, and, in the student's own mailbox, what brings their mail to Knowlu:
forwarding turned on at setup with the student present, older or missed mail forwarded to Knowlu's own
address (whose Sent copies it removes), and forwarding turned back on, and then announced, with the
saved sign-in the student allowed, never for a mailbox the student turned off in Knowlu's Settings. It
never sends anything to anyone else."

**A9. VISION, data minimisation** (`:181`). Now: "- **Data minimization is a rule:** collect the least
that answers the question." Gains: "Where a source cannot be narrowed (forwarded mail arrives whole), the
cheapest deterministic screen discards what cannot answer it before any model reads it, and Knowlu keeps
what it found, never the message."

**A10. VISION, commitment 5** (`:43-46`; and the Amendment 2026-09-29, ruling 3). The table stands. Below
it, a new sentence: "**One exception, at setup:** mail from before a mailbox was connected, read once
when it is connected, is applied and listed under "From your email", each item with Undo; what
conflicts with the student or with an authoritative source is proposed. Mail after setup is proposed
like anything else Knowlu noticed, an LMS's own notification email included; a notice that changes
nothing in the plan (a room, office hours, a mailbox gone quiet) is shown, not proposed." Ruling 3
gains the marker *(amended 2026-09-30: the backfill exception)*.

**A11. VISION, Build order** (`:211-212`). Now: "- sources: the LMS feed, zyBooks, VHL, Google Calendar,
Gmail and campus events, with judgment in the cloud;". Becomes: "- sources: the LMS feed, zyBooks, VHL,
Google Calendar, email (school Microsoft 365 and personal Gmail through the Knowbox, and Gmail directly)
and campus events, with judgment in the cloud;". And Beyond (`:231`): "Outlook and more homework
platforms" becomes "the Outlook calendar and more homework platforms" (HANDOFF `:331` likewise).

**A12. VISION, telemetry** (`:167-169`): "never content derived from the Gmail API" becomes "never
content derived from the Gmail API or from forwarded mail". **CLAUDE.md:** the slot line "A slot is
`sync → coursework → ingest → grades → judge → rank`" keeps its chain and gains a sentence: "Before the
chain, the app runs its own window steps, the grades capture and then the saved-sign-in `mail` step;
neither writes the vault." The actor list gains "forwarded mail as `agent:knowlu.mail`".

**A13. Ruling 12 of Amendment 2026-09-29: the policy-read gate suspended until 20 paying users**
(§13 Q6; D25). Quinn's words (2026-09-30): "change both to allow. We'll keep the school sign-in and mail
sign-in for now. Once we have 20 paying users, we'll come back to this." Ruling 12's current text, in the
five passages that change (`cloud-design:690-691`, `:695-698`, `:703-711`, `:699-703` and `:714-717`);
every other sentence of ruling 12 stands, read with the heading marker below:

> "A release may carry M1's code before then, because without a date it offers grades nowhere."

> "the founder's MVP grades proof runs on a dev build from a proof branch that carries UA's date (the
> existing live-proof practice: dev build, scratch profile), and that branch is never merged ahead of
> bump #1; MVP exit therefore needs UA's read recorded and the proof passed on the proof branch, not
> UA's date on main."

> "The promise: **Knowlu's released app never offers or uses the session at a school without a
> recorded read.** In the Pilot, each other pilot student's university gains its read (ruling 10). The
> gate is mechanical, not operational, and no build skips it: the grades spec (§2–§4 and §9–§12 as
> signed; the spec's text takes edits 3–5 in the signing commit) puts one predicate in
> `app/src/grades.rs`, true only where the curated campus row carries the read's date (`policy_read`),
> never at an uncurated school through a typed address. The strip's status, Connect, a manual refresh
> and the scheduler all check it; Connect and refresh refuse with the same named reason, and the slot
> records `grades (skipped: not available at your school yet)` at exit 0."

> (*Policy read*, from "and UA's read moves") "and UA's read moves to MVP exit (ruling 10). Quinn is a
> UA student and the MVP's only user, so the founder's grades proof runs through the real gate on the
> proof branch; the MVP's grades proof waits on Quinn recording UA's read."

> (*What it costs*) "one test in M1, and UA's date waits on privacy bump #1 to reach main; the MVP's
> grades proof waits on Quinn recording UA's read. If UA's policy forbids a persisted SSO session, the
> MVP cannot exit as item 9 defines it and Quinn rules again on grades; the pilot may then start
> without them."

The replacements, in order:

> "No release is tagged from a `main` that carries M1's code with the gate suspended until privacy
> bump #1 has merged, because with the gate suspended it offers grades at every curated Blackboard
> school."

> "the founder's MVP grades proof runs on a dev build of `main` with the gate suspended (dev build,
> scratch profile); no proof branch and no `policy_read` date are needed."

> "The promise, suspended: **until Knowlu has 20 paying users, Knowlu's released app offers and uses
> the session at every school without a recorded read, as it does the saved mail sign-in (the email
> spec's D15).** The reads (UA's at MVP exit, each pilot student's university in the Pilot, Launch's
> widening checklist) wait for the review below. The gate stays mechanical, not operational, and no
> build skips it: the predicate in `app/src/grades.rs` stays the one place that decides, and one
> constant there, `POLICY_READ_GATE`, reads `Suspended`, the same in every build (no `cfg`, feature or
> environment variable). While it does, the predicate is true for every curated campus row whose
> `lms_kind` is `blackboard`, dated or not, and still never at an uncurated school through a typed
> address. The strip's status, Connect, a manual refresh and the scheduler all still check it; an
> uncurated Blackboard school still refuses with the same named reason, and the slot records `grades
> (skipped: not available at your school yet)` at exit 0. The `policy_read` field and the
> date-and-bump test stay. *Review trigger (set 2026-09-30):* when Knowlu has its 20th paying account
> (an `active` paid subscription; `trialing` and founder-owned test accounts are not counted), Quinn
> rules again before the next release: restore the gate (`POLICY_READ_GATE` back to `Enforced`, with a
> read recorded for each school whose students use either session) or keep it suspended with a new
> trigger."

> "and UA's read waits, with every other school's, for the review at 20 paying users (the promise
> below). Quinn is a UA student and the MVP's only user, so the founder's grades proof runs through the
> suspended gate on a dev build of `main`; the MVP's grades proof waits on no read."

> "one test in M1, and any school's `policy_read` date still waits on privacy bump #1 to reach main
> (the date-and-bump test). The MVP's grades proof waits on no read, and the MVP's exit does not depend
> on what UA's policy says. If a read taken after the review finds that a school's policy forbids a
> persisted SSO session, Quinn rules again on grades at that school."

Ruling 12's heading gains *(suspended 2026-09-30 until 20 paying users, for the Blackboard session and
the saved mail sign-in alike: Amendment 2026-09-30, A13; wherever this ruling says a read, a date or a
proof branch is needed before grades are offered or proved, that need is suspended with it)*. The same
marker, shortened to *(suspended until 20 paying users: Amendment 2026-09-30, A13)*, goes on:
- **ruling 10** (`:596-605`, `:611-613`, `:619-623`, `:627-628`): the MVP list's "and UA's
  university-policy read (ruling 12)"; the MVP *Exit*'s "Quinn has recorded UA's read" and "from the
  proof branch that carries UA's date", and its last sentence ("The MVP's grades proof therefore waits
  on Quinn recording UA's read"), which are struck; the Pilot list's "the university-policy read for
  each pilot student's university other than UA"; the Pilot *Gate*'s "And no pilot student uses ruling
  12's kept Blackboard session before the policy read for that student's own university is done … only
  where the curated campus row records the read"; and Launch's "each university-policy read not
  already done in the MVP or the Pilot";
- **§11a's grades row** (`:359`), whose 2026-09-29 marker ends "the released app never offers or uses
  the kept session at a school without a recorded read";
- **the grades spec**, §4 (`grades-design:80-92`) and its proof lines (`:281-283`, `:319-321`).

**VISION** (`VISION.md:218-224`). Now: "No one else signs up before the privacy page names the kept
Blackboard sign-in and the grades the account holds, … No student, the founder included, keeps a
Blackboard sign-in before their own university's policy has been read: the app offers the Blackboard
connection only at a school whose read it records (UA's is read before the MVP ends), so until then a
pilot student joins without grades from Blackboard." Becomes: "No one else signs up before the privacy
page names the kept Blackboard sign-in, the saved mail sign-in and the grades the account holds, … Until
Knowlu has 20 paying users, the kept Blackboard sign-in and the saved mail sign-in are offered at every
school without waiting for that school's policy to be read; at the 20th paying user the founder rules on
the reads again (the cloud design's Amendment 2026-09-30, A13)."

**Not amended, and why.** VISION commitment 3 (the AI never produces what the student sends to someone
else): every send here is to Knowlu's own address and carries the student's own mail, not AI output. The
standing rule "one daily email at most" holds: the canary obeys it (§4.10). **Corrections that need no
signature** (docs-keeper, T13): the legal landscape note's stale Crimson lines (`:121`, `:748`) and
HANDOFF's lanes, queue and bump-#1 list.

## 11. Risks, ranked

| # | Risk | How likely, how bad | Mitigation |
|---|---|---|---|
| 1 | **A wrong silent change in the backfill**: the model misreads a due-date move, or a claim lands on the wrong item, and the plan changes with no card. | Likely at some rate; the worst harm here, because nothing asked the student. | Evidence must be verbatim (D9); the 0.6 floor; matching is deterministic and strict, the reasoning model only breaks ties and "none" is a notice (D12); the student's fields and the LMS's items are always cards (D11); every direct write is listed with Undo and leaves a body line (D10, §5.2); a synthetic check fixes the pin (§12.1, 17); live mail is always cards. Only an `aligned` original can make a direct write, and Junk and Deleted are never searched, so a spoofed "due date moved" in the backfill is at most a card (§4.6 step 6). |
| 2 | **A school blocks external forwarding**, now or later. | Microsoft's default for new tenants; UA forwards today (E2, X3a). Bad for that school's students. | The setup canary names it (D16); the periodic canary finds a later block; personal Gmail and guided steps remain; no IT contact (Q7). |
| 3 | **A provider changes its pages** and a step script breaks. | Certain over time. | Each step checks its own effect and falls back to guided steps (D5); scripts are versioned with the release (D19); the controller's live smoke runs before each release that carries them. |
| 4 | **The WebView2 host cannot do what the spikes did through CDP**: a hidden window stops rendering, Google's popup is not allowed, Outlook's protocol prompt is not suppressible, or a script's result cannot come back without IPC. | Unverified (research: "Tauri 2 popup and new-window hook not verified"). Blocks the automatic path. | T0's spikes S1–S4 before any app task; off-screen instead of hidden; a title channel instead of a returned value; if popups cannot be allowed, Gmail's step 3 is guided. |
| 5 | **The saved sign-in**: full mailbox access on the device; Microsoft's and Google's terms on automated access; a school's policy on kept sessions. | Real; a trust and legal risk more than a technical one. | One profile per mailbox, deleted on its Disconnect, on Knowlu sign-out and on account deletion, with a retried delete; DPAPI for the cookies and an honest sentence about the cached pages (§7); one switch; three named actions only; the privacy page and the lawyer read in bump #1; on at every school by Quinn's ruling (D25), whose own risk is 17. |
| 6 | **Silent loss on the path** (8 sent, 1 arrived, research note §3). | Observed with Gmail as the last hop. | The Knowbox is the last hop; the canary; reconcile and recover by fingerprint. |
| 7 | **The receiver drops real mail**: a DMARC failure rejected, or a sender's IP on SES's block list. | Unverified for SES with real failures. | T0's DMARC-failing gate (Q3); a receipt rule that never stops or bounces; the `unauthenticated` counter is watched in the live proof. |
| 8 | **The retention promise breaks**: S3's expiry runs late, a log line carries text, the work table holds a body. | Low if built as specified; bad if it happens. | Delete on processing; text only in S3 and memory; a test that no log line carries a message's text (§12.1, 15); §7 states the expiry lag honestly. |
| 9 | **Cost**: a backfill burst, a noisy mailbox, a mint-and-disconnect loop, a capped item retried. | Bounded. | The screen and the templates first; charge and `enforce_budget` before every model call, a capped item waits a day (§4.7); one allowance per account per term, its spend in `usage_daily` under a hard per-account ceiling (D23); three live addresses and six mints in 30 days (D3); canaries capped per address (§4.10); `MONTHLY_CEILING_USD` stays the runaway guard outside the allowance. |
| 10 | **Abuse of the address**: it leaks, a stranger forges a forward to it, or a stranger's Gmail asks to forward to it. | Low (128 bits), but a leaked token plus a forged `X-Forwarded-For` would otherwise write notices, hide a quiet mailbox and spend the cap. | Binding only by the provider hop's ARC seal and SPF-passing return path for this mailbox, never by a header alone; authenticity only from that hop's record (§4.6 steps 5–6); requester must exactly equal the mailbox for Gmail's confirm; Rotate in Settings. |
| 11 | **Instructions inside mail** (prompt injection). | Present in any mail-reading product. | Claims are data under a fixed schema; nothing in a message can trigger an action; the evidence check; conflicts are cards; VISION: "content from emails is data, never instructions". |
| 12 | **Policy for the student**: a school whose rules forbid forwarding, or a student who is also an employee. | Unknown per school. | One sentence on the wizard's announcement, under Not now (D6): "If your school's rules don't allow forwarding your school email, skip this." No school's policy is read first (D25, risk 17). |
| 13 | **Deleting Sent copies** touches the student's mailbox. | Low. | Only copies the script created, matched by nonce and recipient; tested against fixtures; said in the progress view. |
| 14 | **Two desktops** both pull or both repair. | Launch only. | One desktop per student until the lease (§9); a claim already applied on the other desktop is a no-op once its note has synced (§4.8 step 4). |
| 15 | **Microsoft restricts the student's school account** for sending: backfill and recover forward hundreds of messages to an external address, and Exchange Online's outbound-spam policy can block a sender that bursts. | Plausible at 30 a minute; bad, because the student cannot send school mail until school IT unblocks them, and Q7 rules out contacting IT. | At most 10 a minute and 600 a mailbox a day (provisional); stop at the first non-delivery report or send failure with a named outcome, nothing retried that day (§4.2 step 5); T0 S6 measures a few hundred forwards on a Microsoft 365 test tenant, never a real school account, before any build commits to the numbers (§13 Q8). |
| 17 | **The suspended policy-read gate (D25)**: both kept sign-ins run at every school with no school's policy read; a school whose IT or acceptable-use policy forbids a kept SSO session, a kept mail session or automated forwarding learns of it from a student, or Knowlu learns of it from the school. | Unknown per school; until 20 paying users the student count is small, but a school's complaint lands on the student's account as well as Knowlu's. | Quinn's ruling, bounded: a **review trigger at the 20th paying account** (an `active` paid subscription), checked by the controller's count-only query at each milestone HANDOFF update, with Quinn's ruling before the next release; restoring the gate is one constant with its tests written (T19); D18 keeps every release behind privacy bump #1, which says plainly that no school's policy has been read (§7); the lawyer read still happens; risk 12's sentence on the wizard's announcement; Turn off email and Forget Blackboard sign-in delete each session on the device. |
| 16 | **Recovery re-forwards what the Knowbox holds**, or a lapse loses mail. | Without one agreed time field, intake's and reconcile's fingerprints would rarely match. | Keyed fingerprints on one received minute, diffed on the device within 15 minutes (§4.6 step 7, §4.11); `seen` written only with the work row; an expired item's `seen` deleted; a lapse writes no `seen` and reconcile reaches back to `lapsed_since`. |

## 12. Test plan, test first

Every behaviour gets its failing test before its code. **Tests never leave the machine:** every server a
test needs (S3, SNS's certificate host, Google's confirmation page, the model, the service for the
device) is a listener on `127.0.0.1:0` serving itself. **Every message is synthetic**, written for the
test; no real mail, address or name enters the repo (rule 1). No test touches `engine/tests/fixtures/**`.

### 12.1 Cloud (Deno; cloud-engineer)

1. SNS: a valid signature from a test key served by a local certificate host is accepted; a wrong topic,
   a bad signature, or a certificate host outside the allow-list answers 403 and reads nothing.
2. Binding: an unknown or revoked token is deleted and counted **with zero S3 GETs** (a spy S3), as is
   every message of a lapsed account (no `seen` row written); a retired token inside its Rotate grace
   still binds. A live forward binds only with the provider hop's verified ARC seal covering a binding
   header that names this mailbox **and** an SPF-passing return path for this mailbox. **Forgeries:**
   a direct send carrying a forged `X-Forwarded-For: <student> <token>@…` is unbound; a forward sealed
   by `google.com` from an attacker's own Gmail (its `X-Forwarded-For` and `+caf_=` return path name
   the attacker) is unbound; an M365 forward whose `ForwardingLoop` names another tenant than the
   mailbox's `forward_hint` is unbound; none of them moves `last_received_at` or charges a cap.
3. Never reject, verify after: a DMARC-failing original whose provider-hop ARC-Authentication-Results
   shows an aligned pass is `aligned`; one whose only "pass" is in an `Authentication-Results` the sender
   wrote is `unaligned` and, live, deleted as `unauthenticated`; no authentication result ever produces
   a non-2xx answer.
4. A backfill send: outer DKIM and nonce make each `message/rfc822` attachment one work item counted
   toward its batch; a PDF attachment is ignored; a wrong outer sender is dropped. A backfilled original
   with no aligned delivery-time pass is kept as `unaligned` (test 22b checks it never writes directly).
5. Gmail confirm: the mailbox's own requester, exactly, inside the window is completed by a cookie-less
   GET and POST to a local fake page. Refused, each its own case: `notjane@gmail.com` when the hint is
   `jane@gmail.com`; a null hint; a closed window; a sender other than Google's exact forwarding sender,
   or without aligned DKIM; a link off the allow-list; a redirect (`redirect: manual`); a form whose
   action is off the allow-list.
6. Canary: a matching nonce marks it arrived and records `forward_hint`; a replay changes nothing; a
   wrong sender domain is unbound. `POST /canary` refuses a hint outside the provider's domains and a
   fourth canary for one address in a UTC day. `knowbox-canary` (§13 Q2) sends none to a mailbox with
   bound mail inside 24 hours on a weekday (48 over a weekend), none within 72 hours of that mailbox's
   last canary, and none on a day the student had a Knowlu email; a mailbox quiet past both is sent one.
7. Dedup: one Message-ID twice makes one work item; a message with none dedups on Date, From and Subject.
   `msg_hash` and `fp_hash` are HMACs under the account's key (the same Message-ID under two accounts
   hashes differently). A failed S3 put answers 500 and writes neither `knowbox_work` nor `knowbox_seen`,
   so the retry is processed; an item swept as `expired` loses its `seen` row, and a later recover of
   the same message is accepted (D8).
8. Screen: bulk mail from a sender off the allow-list is `noise` with **zero** model calls (a spy model);
   an `.edu` sender with `List-Unsubscribe` passes on.
9. Templates: a synthetic LMS "due date changed" mail that is `aligned` for the vendor yields a template
   claim with a resolved due; the same mail `unaligned` goes to the model instead.
10. Validation: evidence not in the text is refused `incomplete`; an unknown course is null; an
    unresolvable due is null; a sixth claim is dropped; below 0.6 is refused; a `new_item` without
    `effort_hours` or `importance` in range is refused.
11. Allowance and cost: the allowance opens at 2,000 judgments and $1.00 above the account's ceiling
    (§13 Q1), and the 2,001st backfill item waits for the daily cap; a backfill item charges the
    account's allowance, not the daily cap; the charge
    and `enforce_budget` run **before** the spy model, and a capped or over-budget item makes zero model
    calls, waits to the next UTC day, at most three attempts, then expires with its `seen` row deleted;
    Disconnect and reconnect do not renew the allowance; a fourth live address and a seventh mint in 30
    days are refused; allowance calls add tokens to `usage_daily` (and so to `monthly_spend`) but no
    `calls`, so the same day's live cap is untouched; the raised ceiling ends with the allowance.
12. Pull: (ordering date, uid) order; a message dated 2099 orders at its arrival time, not 2099; a batch
    is withheld until it closes (received reaches sent, or the timeouts), then delivered whole; a
    mailbox's live claims queued during its open batch come after it; ack marks delivered; an ack
    outside the `mail:` shape is dropped.
13. Resolve: the stored answer is replayed on a re-pull; an id outside the batch reads as none.
14. Migrations: RLS on and no `anon`/`authenticated` grant on every Knowbox table; `models.kind`,
    `usage_daily.kind` and `judgments.kind` accept the three mail kinds and `charge_call(…, 'mail_claim',
    120)` succeeds; `corrections.judgment_kind`, `rules.kind` and both eval tables still refuse them;
    `promote_rules` and `backfill_correction_judgments` pass over a `knowbox` judgment and the nightly job
    still promotes another account's rules; the last `export_training_rows` checks `origin not in
    ('gmail_api', 'events', 'knowbox')`; `delete_knowbox_address` deletes exactly one mailbox's rows,
    delivered claims included; account deletion purges every mailbox and its objects by key, names all
    seven tables, and the export carries D17's rows and never a token, key or hash.
15. Logs: an intake and a process run over a message carrying a marker string print no line containing it.
16. S3: SigV4 GET, PUT and DELETE against a local fake; a failed PUT answers 500 so SNS retries. **No
    object is left behind:** after a run over one message of each final path (queued, noise, refused,
    template-only, unauthenticated, expired, unbound, revoked), the fake bucket's `inbound/` and `work/`
    are empty.
16b. Entitlement: `GET /addresses` and `DELETE /addresses/<id>` succeed for a canceled account (402 never
    blocks a disconnect, as 89c4042 did for google-connect); every other `knowbox` route answers 402.
16c. `gmail-read` (D22): an account whose Gmail address has an active Knowbox gets a non-quiet answer with
    its undelivered items, `read: 0`, and no Gmail API call.
17. The pin's check: before `mail_claim` and `mail_resolve` are pinned, the controller runs about twenty
    synthetic messages (a due-date move, a new assignment, a room change, an announcement with no
    obligation, a bulk mail, an injection attempt) through each on staging and records the answers, as
    the provider swap's live check did. The eval seed stays empty (R-C2-E12); the gate wakes when
    consented corrections exist.

### 12.2 Engine (Rust; temporary vaults)

18. **Determinism:** one vault and one claim list planned twice give identical actions; the same
    messages delivered in a shuffled order give the same actions; **a backfill split across pulls** (a
    "due moved" claim whose creating email arrives later) gives the same vault as one pull, because the
    batch is applied whole (with the service's withholding faked at the loopback).
19. Supersession inside one pull, and across pulls through `mail_asof`; a live claim older than a
    backfill value already applied is skipped as superseded; a message whose Date is in the future does
    not block later claims (its ordering date is its received time).
20. §4.9, row by row and phase by phase (one test each), every `change` row using **`due`**.
21. Judge once, with `due`: a task the student created by hand (a journal `create` by the human actor
    carrying `due`) and a task whose `due` the student `set` each get a `kind: amend` card from a live
    and from a backfill claim, never a set, written by the mail card writer; a second claim while that
    card is pending files no second card.
22. LMS items, with `due`: a model change is a card; an equal value writes nothing; a backfill template
    change to an item a feed supplies writes nothing; a backfill template change to a note no feed
    supplies is direct; a live template change is a card unless equal.
22b. Authenticity: an `unaligned` backfilled original's direct rows become cards; an `aligned` one's stay
    direct.
22c. `cancelled`: a card proposing `status: active → archived` in both phases; never a direct write, and
    no new field appears.
22d. First day: on vault day 1 a mail card (backfill conflict and live) is written with `proposed_at`
    and `snooze_until` set to day 2 and `first_proposed_at` day 1; direct writes and notices are not
    held; on day 2 the card is in the deck.
22e. Two desktops: a claim whose `source_uid` already exists on a synced note or card writes nothing.
23. Past due: backfill creates and archives `imported-past`; live files a card. A past event: backfill
    archives `imported-past`, live writes nothing and counts it; a future event is a card in both.
24. A `new_item` whose title equals an active note in its course becomes a change, never a duplicate.
25. Ambiguity: one `mail-resolve` call per course (a scripted fake); "none" is a notice; a stored
    resolution replays with zero calls.
26. Notices supersede by `close_key`.
27. `mail_asof` is a single-line flow mapping; every other byte of the note is unchanged.
28. `state/ingest-seen.md` and the ack; the run line's exact text; nothing printed with no mailbox; a
    closed batch drains past three rounds until the budget is spent.
28b. Labels: a rejected mail card (`judgment_kind: mail_claim`) is never reported by `report_labels`.
28c. A note created from mail is written `needs_enrichment: false` with the claim's effort and
    importance, and a `judge` run over the vault makes **no** `judge-task` call for it (a spy
    loopback service), so no `device`-origin judgment is ever made from mail text.
29. `mail-applied`: the JSON, "changed since", and the vault's bytes unchanged after it runs.
30. Loopback: `pull_mail_claims` and `resolve_mail` over `127.0.0.1:0`; every failure shape is a named
    line and `judge` exits 0.
31. *(contract-engineer, `approvals.rs`)* An approved `created_by: mail` amend writes its fields as
    the human actor; with a bad `config/actor.yaml` it stops with ruling 11's named error and the card
    stays pending; a non-mail amend executes exactly as today.
31b. *(contract-engineer, `ingest.rs`; §13 Q7)* After an approved mail amend moved `due` to Fri, an
    `ingest` whose feed says Wed files **one** `kind: amend` card (Fri → Wed) and leaves `due` at Fri; a
    second ingest files no second card; after the student rejects it, the same feed value is not
    proposed again, and a new feed value is; a `title` the student `set` by hand behaves the same; a
    `due` no human edited is still overwritten by the feed exactly as today. The card passes
    `approvals::validate_amendment`, carries the ingest actor and its own "Why proposed" line, and never
    the text "re-judged"; approving it moves `due` to Wed and a later feed move is a card again.

`oracle.rs`, `surface_oracle.rs` and every sync and entitlement test pass unchanged.

### 12.3 App (Rust, and Deno for the step scripts)

32. `mail.rs`'s pure pieces: each provider's step list; a failed step falls back to guided for that step
    only and the later steps continue; an existing forwarding address is a needs-you; the term-start
    default for given dates; `is_session_dir`'s refusals; `forget(address_id)` deletes only that
    mailbox's `mail-session-<address_id>`, and a failed delete is recorded and retried at start-up; the
    start-up sweep removes a leftover `mail-setup`; the pace and daily ceiling, and the stop on the first
    send failure.
33. The `mail` window label has no capability grant; one mail window at a time; the script runner
    refuses to run a step script on a page whose origin is off the provider's list (a synthetic SSO
    page).
34. **No OS input, no debugging port:** a source test that `app/src/mail.rs` and `app/assets/mail/**`
    name none of `SendInput`, `keybd_event`, `mouse_event`, `SetCursorPos` or `remote-debugging`.
35. Each step script, under Deno with a DOM shim, against **synthetic** fixture pages modelled on the
    spikes' selectors: it acts, its check reads the effect back, and a page missing its control returns
    the named "not found" that triggers guided steps; the M365 switch is read by `checked`.
36. The scheduler: in `run_slot_with`, the grades capture, then the `mail` step, then the child-process
    loop starting with `sync`; `slot_argv` is unchanged; the `mail` `RunSummary` row and each named
    skip at exit 0; no entitlement is a skip; reconcile runs at most once a day (§13 Q3). **Re-enable
    (§13 Q11):** a forwarding found off is turned back on and reported `reenabled`, and the next mail
    pass opens one heads-up whose text is exactly "Email forwarding was off; Knowlu turned it back on.
    Turn off email in Settings to stop this."; a mailbox in `mail.json`'s `turned_off` is never turned
    back on, including when Disconnect's server call or profile delete failed after step 0; a
    forwarding pointing elsewhere is left unchanged and asked about.
37. The ten commands are in the console list and not in the wizard's.
38. *(contract-engineer, `account.rs`)* `delete_my_data` runs the forwarding-off pre-step, a failing
    pre-step never blocks the deletion, and the mail windows close before `delete_local_data`;
    `sign_out` deletes every `mail-session-*` and `mail-setup` profile of that Knowlu profile and
    leaves the other profiles' alone.
39. *(T19, `app/tests/grades.rs`; D25)* **The predicate, both arms, at the pure seam.**
    `POLICY_READ_GATE` is `Suspended`. The six predicate tests that call `availability` today
    (`a_dated_blackboard_row_is_available_with_its_own_host`, `an_undated_blackboard_row_is_not_available_yet`,
    `an_uncurated_blackboard_school_is_not_available_yet`, `a_curated_canvas_row_is_not_a_blackboard_school`,
    `no_school_is_not_a_blackboard_school`, `no_real_campus_is_available_on_this_branch`) make each of
    today's assertions against `availability_with(Gate::Enforced, ..)`, unchanged, so restoring the gate
    needs no new predicate test; this is the only place the `Enforced` arm is reached. Each also asserts
    the `Suspended` arm through `availability` (which is `availability_with(POLICY_READ_GATE, ..)`): an
    undated curated `blackboard` row is `Available` with its own `lms_host`, as a dated one is; every real
    curated `blackboard` row in `CAMPUSES` is `Available` with its own host and every other real row is
    not; an uncurated Blackboard school is still `NotAvailableYet`; a curated `canvas` row and no school
    are still `NotBlackboard`. A test may be renamed to say both arms (the undated and real-campus ones
    must be, since their names state the `Enforced` result); none is deleted. A source test fails if
    `cfg`, a feature or `std::env` reaches the gate.
40. *(T19, `app/tests/grades.rs`)* **The command seams, `Suspended` only.** They call `availability`
    and take no gate, so their undated cases are rewritten to the suspension, and every other case keeps
    today's assertion unchanged. `grades_connect_refuses_without_a_date_and_opens_nothing`: the undated
    row now opens the window and reads the session on its own host, in that order, as the dated row does;
    the uncurated school still refuses with `not available at your school yet` and opens nothing; the
    `NotBlackboard` and failed-window cases are unchanged. `grades_refresh_refuses_without_a_date_and_reads_no_session`:
    the undated row with a saved session opens and reads; the uncurated school still refuses and opens
    nothing; the `not connected` case is unchanged. `grades_status_reports_the_gate_and_with_a_date_the_session`:
    the undated row reports `available: true` with the session fields, as the dated row does; the
    uncurated school still reports `not available` with no session fields; `NotBlackboard` unchanged. The
    three tests are renamed to drop "without a date"/"with a date" where it is no longer true; none is
    deleted.
41. *(T19, `app/tests/scheduler.rs`)* **The scheduler, `Suspended` only**, the same way:
    `the_grades_decision_asks_the_predicate_and_never_rederives_it`: the undated row captures, as the
    dated row does (the capture count becomes 2); the uncurated Blackboard school still skips with
    `grades (skipped: not available at your school yet)` and writes no bundle; the `canvas` and no-school
    skips are unchanged. `the_skip_order_is_school_then_availability_then_entitlement_then_session_then_window`:
    the "not available yet beats no entitlement" pair uses the uncurated Blackboard school (`None`,
    `"blackboard"`) instead of the undated row; every other pair is unchanged.
    `the_slot_records_the_grades_skip_and_runs_and_cleans_up_the_grades_step`: the named-skip half
    (skip at exit 0, `engine_ok`, no capture, the runner-log `ok` line) runs with an uncurated Blackboard
    school; if `GradesSeam` cannot express one without a new field, the skip half asserts a canvas row's
    `grades (skipped: not a Blackboard school)` instead and the uncurated skip stays pinned by
    `grades_step` above (no seam field is added); the dated half is unchanged. The date-and-bump tests
    (`a_dated_row_fails_while_bump_1_has_not_happened` and its neighbours) and
    `no_curated_row_carries_a_policy_read_date` pass unchanged.

### 12.4 Page (console-ui)

Static tests for the progress view (rows, needs-you, Leave, Stop), Settings → Email's states, "From your
email" and its Undo buttons, the wizard's offer, and the card's sender, date and quote;
`scripts/settings-check.py` and `scripts/wizard-check.py` walk them. **Untrusted text:** a card, notice
and "From your email" row whose `from`, `why`, `evidence` and title carry `<img src=x onerror=…>` render
it as inert text, and a static check fails if any mail-derived field reaches the DOM without `h()`.

### 12.5 T0: gates before the build commits (main session, with Quinn where a secret or a sign-in is needed)

- **S0, the receiver (Q3's gate).** SES receiving on staging's AWS account for the subdomain, with the
  S3 action's `TopicArn` as the only action; a deliberately DMARC-failing message (a founder-controlled
  sending domain with `p=reject` and a broken signature) lands in S3 with its verdicts recorded; a
  forwarded M365 message and a Gmail forward land too. **It also records, as evidence for §4.6 steps 5–7
  (headers only, never bodies, kept in the T0 report):** the SNS notification's fields; each forward's
  ARC instances, sealing domains and the AMS `h=` lists; the envelope sender's shape (Gmail's `+caf_=`,
  Microsoft's SRS) and SES's SPF verdict for it; Microsoft's `ForwardingLoop` tenant field; which
  `Authentication-Results` each provider stamps at delivery and how to tell it apart; whether an
  intra-school message (UA sender to UA student) carries an aligned pass; and whether OWA's and Gmail's
  forward-as-attachment keep those delivery headers. Records the SES region. **No cloud task starts
  until S0 passes.**
- **S1–S3, the host.** In a throwaway Tauri 2 window on WebView2: a script's result comes back from an
  external page without IPC; a hidden or off-screen window keeps Outlook's settings page rendering and
  scriptable; Google's "verify it's you" popup is allowed and returns to its opener; Outlook's "open
  email links" prompt is suppressed. **No app window task starts until S1–S3 pass.**
- **S2 also records** what each provider's message list exposes for the sender (display name or
  address) and the received time, which fixes the fingerprint's sender form (§4.6 step 7, §4.11).
- **S4, session lifetime.** The 10-08 `sessions.log` is read; Quinn revisits §13 Q3's once-a-day
  reconcile with it (keep it if sessions survive a week of daily use; repair-only if they die within
  two days).
- **S5 (optional).** Whether an Outlook inbox rule can forward to the Knowbox beside an existing
  forwarding address (§4.2 step 3's third choice).
- **S6, Microsoft's sending pace.** A few hundred forwards-as-attachment, at the provisional 10 a
  minute, from a mailbox on a Microsoft 365 test tenant Quinn creates for it (§13 Q8), never a real
  school account; records any throttling, non-delivery report or restriction, and sets the pace and the daily
  ceiling of §4.2 step 5. **No backfill code is merged with numbers S6 has not confirmed.**

### 12.6 Exit: the live proof (main session, on staging)

A dev build of the merged branch, a scratch profile, a staging session by OTP, a founder-owned school
mailbox and a founder-owned test Gmail account, Quinn at the machine for each sign-in and phone tap.

1. Onboard the scratch profile; the wizard's panel after sign-in reads "Next: connect your school
   email"; at the console's first open the school mail sign-in window opens and the progress view
   shows in the console (§13 Q9). Grades (D25): with UA's curated row carrying no `policy_read`,
   Blackboard Connect is offered and the slot's grades step runs instead of skipping.
2. School M365: sign-in; the window steps aside; forwarding set and checked; the canary arrives; the
   backfill counts N and forwards; Sent copies removed; "From your email" lists what was applied.
3. Undo one created item and one changed field; the next slot does not re-create or re-change them.
4. Gmail: sign-in; the popup and phone tap; the server-side confirm; forwarding on; canary; backfill.
5. Live mail: a synthetic "due date moved" message sent from another founder-owned account becomes an
   amend card at the next slot (snoozed to day 2 if it is still the vault's first day); approving it
   writes `due` as the student; at the next `ingest` the feed's differing date is a card and `due`
   holds (§13 Q7).
6. Turn forwarding off by hand; the next canary misses; the mailbox reads `quiet`, then `off`; the slot's
   `mail` step turns it back on, the heads-up reads "Email forwarding was off; Knowlu turned it back on.
   Turn off email in Settings to stop this.", and the message sent meanwhile is recovered. Point
   forwarding at another founder-owned address: the step changes nothing and asks.
7. Turn off email for both mailboxes in Settings (Disconnect); the controller's count-only query shows
   no Knowbox rows for the account; forwarding is off in both mailboxes, and the next slot turns
   neither back on (§13 Q11).
8. Clean up per the standing rule: the scratch profile, its vault, credentials and autostart; restore the
   school mailbox's forwarding to what it was before the proof.

Pass means every step as written. Quinn's word closes the MVP row.

## 13. Questions for Quinn (all answered 2026-09-30)

What Q1–Q7 left open, and the concerns review raised against two of those answers (Q6 and Q11 here).
Each question is kept as it was asked, with its options and recommendation; **Quinn's answer of
2026-09-30 follows each one, is binding, and is folded into the decisions, flows, data model, privacy,
risks, tests and tasks above.** Nothing in §13 is open now.

**Q1. The backfill allowance (D23).** The research note's volumes: one personal Gmail took 959 messages
in six weeks (680 without promotions and social); one school mailbox took 75 conversations in two weeks.
A full term is roughly 2.5 times that. After the screen, perhaps half reach the model. At the measured
email prompt sizes (`HANDOFF.md` §1: about 372 tokens in, 86 out) a claim call costs well under a tenth
of a cent; at twice those sizes, about $0.0003.
- (a) **2,000 `mail_claim` judgments per account per term, within 14 days of the first setup, raising
  that account's ceiling by $1.00 while it is open**; the daily cap unchanged and `MONTHLY_CEILING_USD`
  back in force after. *Recommended:* it covers a full term of two mailboxes at the heaviest volume seen,
  Disconnect cannot renew it, and a looping bug stops at a dollar.
- (b) 1,000 and $0.50 per account: a late-term setup finishes over several days at the daily cap.
- (c) No allowance: the backfill trickles at 120 a day (240 on the first two days) and a mid-term setup
  takes a week.

**Answered 2026-09-30: (a).** 2,000 judgments per account per term, raising that account's spending
ceiling by $1 while it lasts (D23, §5.1, test 11).

**Q2. The periodic canary's cadence (D14).** Every canary is an email in the student's inbox, and VISION
allows one Knowlu email a day.
- (a) **Only when a mailbox has been silent: 24 hours on a weekday (48 over a weekend), at least 72
  hours between canaries for one mailbox, never on a day the student already had a Knowlu email.**
  *Recommended:* a school mailbox that is working is rarely silent for a day, so a working student almost
  never sees one.
- (b) A fixed weekly canary per mailbox: predictable, but a broken mailbox can go a week unnoticed.
- (c) Daily: catches a break within a day, and breaks the one-email rule for two mailboxes.

**Answered 2026-09-30: (a).** A canary only after a mailbox goes quiet, at least 72 hours apart (D14,
§4.10, test 6).

**Q3. The saved session's reconcile cadence (D15), after the 10-08 spike.**
- (a) **Provisionally once a day, in the first slot after 10:00, over the last three days, at most 30
  recovered a slot; revisited with `sessions.log`:** if sessions survive a week of daily use, keep it;
  if they die within two days, reconcile only when a mailbox is `quiet` or `off` (repair-only).
  *Recommended.*
- (b) Every slot: catches losses within hours, and opens the mailbox twice a day in the background.
- (c) Repair-only from the start: the gentlest on the session and the provider's terms; losses the
  canary does not see (a burst like §3's) are found only when something else is wrong.

**Answered 2026-09-30: (a).** Saved-session reconcile once a day, revisited after the 10-08 spike
(D15, §4.11, T0 S4).

**Q4. Which provider, when the campus row has no mail field (§4.1).**
- (a) **Curated rows gain `mail_provider` and `mail_domain` (UA's in the MVP); at an uncurated school the
  setup asks once for the school email address and the service decides from its MX records; unknown is
  guided.** *Recommended:* one typed address, which is a sign-in identifier, not data entry.
- (b) No question: infer the provider from where the LMS sign-in window's SSO went (Microsoft's or
  Google's login host). Zero typing, but it holds only where the LMS and mail share one identity
  provider; a later improvement on (a).
- (c) Two buttons, "School email (Microsoft)" and "School email (Google)": the student guesses.

**Answered 2026-09-30: (a).** The provider from the curated campus row, else an MX lookup of the
school's domain (§4.1, §6.3, T14).

**Q5. The address domain (D2).**
- (a) **`in.knowlu.com`, an MX on a subdomain of the product's own domain**, in the SES receiving region
  S0 records. *Recommended:* the apex's mail (`hello@`, `support@` through Cloudflare) is untouched, and
  the address a student sees in their settings says Knowlu.
- (b) A separate domain, to keep the receiver's reputation apart from the product's: one more domain to
  hold and explain, and an address that does not say Knowlu.

**Answered 2026-09-30: (a).** `in.knowlu.com` (D2).

**Q6. Q6's saved session beside ruling 12's promise: a direct conflict for Quinn to resolve.** Your Q6
answer is binding and this spec is written to it: the saved mail sign-in is **on by default for
everyone** (D15, the ruling text in §10). Ruling 12 promises that "Knowlu's released app never offers or
uses **the session** at a school without a recorded read". Read literally, "the session" is the
Blackboard grades session, so Q6 does not break it; read as a promise about any kept school session, a
saved school-mail session at a school with no recorded read breaks it. The two documents must not
disagree once both are signed, so the ruling text in §10 carries a bracketed clause that follows your
answer here.
- (a) **Q6 stands as answered: on by default for every mailbox; ruling 12's promise is about the
  Blackboard session, and the lawyer read and the privacy page carry the mail session on their own.**
  *Recommended:* it is your answer, it needs no gate code, and UA's read, which you record before MVP
  exit anyway, can simply be asked to look at kept mail sessions too; if UA's policy forbids them you rule
  again then.
- (b) Ruling 12's promise extends to mail: a school mailbox's saved sign-in is on by default only where
  the curated row carries `policy_read` (one predicate beside grades'); elsewhere setup works and the
  session is discarded, so reconcile and recover wait for a sign-in. Personal Gmail is not gated. This
  narrows Q6; choose it only if you read ruling 12 as covering every kept school session.

**Answered 2026-09-30: neither option as written; both sessions allowed.** Quinn: "change both to
allow. We'll keep the school sign-in and mail sign-in for now. Once we have 20 paying users, we'll come
back to this … since we're just trying to prove this concept can work as a desktop app, let's give
ourselves as much of an advantage as possible." So until Knowlu has 20 paying users, ruling 12's
per-school `policy_read` gate is suspended for the kept Blackboard session (M1 grades) and the saved
mail sign-in alike; both run at every school without a recorded read; the 20th paying account is the
review trigger. A new amendment item, A13 (§10), quotes ruling 12 and its replacement; D25 and D15
carry it, T19 changes M1's gate, risk 17 names the cost, and §7 says it plainly.

**Q7. Should `ingest` honour ruling 3 now (F1, D11)?** Ruling 3 says no source overwrites a field the
student set by hand; on `main` the LMS feed overwrites `title` and `due` whoever set them. That decides
whether an approved mail change to an LMS item lasts.
- (a) **`ingest` honours ruling 3 for `title` and `due`: where the journal shows a human edit and the feed
  differs, it files one amend card instead of writing; a rejected value is not re-proposed until the feed
  changes again.** *Recommended:* it carries out a ruling you already signed, it makes "approve" mean
  something on an LMS item, and Blackboard moving an item after the student's decision becomes a card
  instead of a silent change. Cost: a behaviour change in every vault whose student hand-edited an LMS
  due date (today silently overwritten), and `ingest.rs` work by contract-engineer (T8b).
- (b) `ingest` unchanged in this lane: an approved mail change to an LMS item lasts only until the feed
  next disagrees, and the card says so. Ruling 3's gap stays open for another lane.
- The alternative of adding `due` to `JUDGED_FIELDS_TASK` is not offered: it changes judge-once for every
  agent in every vault, and the reconciler does not need it (D11).

**Answered 2026-09-30: (a), fix it now.** `ingest` files a `kind: amend` card instead of overwriting a
`due` or `title` the journal shows the student set (ruling 3), as `judge` does for its fields (D11 (5),
§6.1, §8, T8b, test 31b).

**Q8. Where S6 measures Microsoft's sending pace (§4.2 step 5, risk 15).** A restriction blocks the
sending mailbox until its IT unblocks it.
- (a) **A Microsoft 365 developer or trial tenant you create for the purpose (a spend of nothing to a few
  dollars), never your UA account.** *Recommended:* a restriction there harms nobody. Until S6 passes,
  backfill code ships with 10 a minute and 600 a day and the proof's own UA backfill stays under 600.
- (b) Your UA account at a low pace: no new account, but a restriction would stop your school mail.

**Answered 2026-09-30: (a).** S6 runs on a Microsoft 365 test tenant, never a real school account
(§12.5 S6, risk 15, §15).

**Q9. "Offered right after Knowlu sign-in" (D6).** The wizard offers setup on the panel after sign-in and
subscription, and setup **runs** at the console's first open after Finish.
- (a) **As written.** *Recommended:* the wizard window has no console state and no profile folder before
  Finish (C1), and a backfill run before `courses/` exists would be matched against nothing.
- (b) Run setup inside the wizard: the student sees forwarding happen sooner, at the cost of a wizard
  that carries the mail window and a backfill re-matched once courses arrive.

**Answered 2026-09-30: (a), yes.** "Offered right after sign-in" means the wizard announces "Next:
connect your school email", and the mail sign-in opens as the console first opens; progress shows in
the console (§0, D6, §6.2, §12.6 step 1).

**Q10. The sender on mail cards (D21) differs from your Gmail decision (Gmail spec D15: no sender in
the MVP).** Mail cards, notices and applied changes here show the sender's name and address and the date.
- (a) **Show the sender on mail cards in the MVP; the Gmail path keeps D15 until its Pilot item.**
  *Recommended:* VISION commitment 5 says "showing who it came from", and a professor's due-date move is
  judged by who sent it; the MVP's only user needs no disclosure, and bump #1 carries it before anyone
  else.
- (b) Match Gmail's D15: no sender until the Pilot, for both paths.

**Answered 2026-09-30: (a).** Mail cards show the sender (D21; the Gmail path keeps its D15).

**Q11. Re-enabling forwarding the student turned off (Q6, §4.11 step 2).** Your Q6 answer has the saved
session turn forwarding back on when it is found off, and this spec does that and then says so. The
concern: a student who turned it off on purpose has that choice reversed until they notice the heads-up,
and recover then brings in mail from the period they held back.
- (a) **As answered: turn it back on, recover the gap, and keep a heads-up ("Knowlu turned forwarding
  back on… if you meant to stop, disconnect it") until the student dismisses it.** *Recommended:* it is
  your answer, the school or a setting change is the likelier cause than the student, and the heads-up
  makes the change visible the same day. Forwarding that points somewhere else is never changed without
  asking (that is not "turned off").
- (b) Ask first: a heads-up card "Forwarding to Knowlu is off. Turn it back on?" and act only on yes,
  recovering the gap only if the student says so. Gentler, and it narrows Q6.

**Answered 2026-09-30: (a), with Quinn's text and one limit.** Knowlu re-enables forwarding
automatically, then shows a notice: "Email forwarding was off; Knowlu turned it back on. Turn off email
in Settings to stop this." It never re-enables a mailbox the student turned off in Knowlu's own
Settings (`mail.json`'s `turned_off`, written first by Turn off email; §4.11 step 2, §4.12 step 0, §5.4,
§7, test 36).

## 14. Task sketch

One branch and worktree, cut from `main` (at or after `199cd1f`, which carries PR #26) after signing.
Files are disjoint by task except where a row says "after". Each task is test first: §12's tests named
in its row fail before its code. A cheaper agent's work goes through `reviewer` before any push;
`approvals.rs`, `account.rs`, `ingest.rs` and the reconciler go through `contract-reviewer`. Reviews land
in `docs/reports/`.

**The plan splits the large rows.** T2, T3, T5, T9 and T10 are each well over about 80 lines of edits.
The plan breaks each into tasks of about 80 lines or fewer, with disjoint files and a roster label on
every one; this sketch names the seams, the plan fixes them:
- T2 → `_shared/sns.ts`, `_shared/s3.ts`, `_shared/mime.ts`, a new `_shared/knowbox_bind.ts` (binding
  and authenticity, §4.6 steps 5–6), then the `knowbox-inbound` handler (all cloud-engineer).
- T3 → `_shared/mail_templates.ts`; **one task owns each shared judge file**
  (`judge_validate.ts`'s `mail_claim` region; `judge_prompts.ts` and `judge_caps.ts` together), so no two
  tasks edit the same file; then `knowbox-process` (all cloud-engineer).
- T5 → `engine/src/mailreconcile/` as new files (`mod.rs` with the action type and test 18;
  `resolve.rs` for §4.8 step 2; `rules.rs` for §4.9's table), each contract-engineer, in that order.
- T9 → the window and runner, and the session directories with `forget` and the sweep, as two new files
  (contract-engineer each).
- T10 → the step state machine (`mail_steps.rs`), then each provider's scripts and their Deno tests
  (`app/assets/mail/m365/**`, `app/assets/mail/google/**`) (implementer each).

| Task | Agent, and why | Files | Tests (§12) |
|---|---|---|---|
| **T0** Gates S0–S6 | **main session**: AWS secrets, a live mailbox and Quinn's sign-ins are the controller's; a report in `docs/reports/` (headers only, never a body) | none in the repo | S0–S6 |
| **T1** Schema | **cloud-engineer** (Opus, high): `cloud/` is theirs; a wrong grant leaks every student's mail metadata | new `migrations/<date>_knowbox.sql`; `migrations/migrations_test.ts` | 14 |
| **T2** Intake | **cloud-engineer**: student mail, signatures and secrets; after T1 and S0 | `functions/knowbox-inbound/**`; new `_shared/{sns,s3,mime}.ts` | 1–7, 15, 16 |
| **T3** Claims | **cloud-engineer**: the screen and the model path; after T2 | `functions/knowbox-process/**`; new `_shared/mail_templates.ts`; the `mail_claim` regions of `_shared/judge_{validate,prompts,caps}.ts` | 8–11, 17 |
| **T4** Endpoints | **cloud-engineer**: after T1 | `functions/{knowbox,knowbox-canary,mail-pull,mail-resolve}/**`; `functions/account/**` (the purge and the export) | 6 (canary limits), 11 (address caps), 12, 13, 14 (purge, export), 16b |
| **T4b** D22 | **cloud-engineer**: one branch in the merged `gmail-read`; after T1 | `functions/gmail-read/handler.ts`, its test | 16c |
| **T5** The reconciler | **contract-engineer** (Opus, xhigh): off the list, but a silent error changes a student's plan with no card; the events spec routed its series carry the same way. Its **first commit** is the action type and test 18 alone | new `engine/src/mailreconcile/**` | 18–24, 22b–22d, 26 |
| **T6** The mail pass | **implementer** (Sonnet, high): specified, off the list, checked by tests; starts from T5's first commit | new `engine/src/mail.rs` (with its card writer); `engine/src/cloudmodel.rs` (two calls); one call in `engine/src/enrich.rs`; `engine/src/lib.rs` (`mod` lines) | 21 (the writer's half), 22e, 25, 27, 28, 28b, 28c, 30 |
| **T7** `mail-applied` | **implementer**: read-only, specified | new `engine/src/mailapplied.rs`; the `main.rs`/`cli.rs` lines are a controller hand-off | 29 |
| **T8** D11 | **contract-engineer** (Opus, xhigh): `approvals.rs` is on the list; after the events lane's T3 if it merged first | `engine/src/approvals.rs` | 31 |
| **T8b** Ruling 3 in `ingest` *(§13 Q7: fix it now)* | **contract-engineer** (Opus, xhigh): off the list, but it changes the feed's behaviour in every existing vault and writes cards through `write::create`, whose shape `approvals::validate_amendment` (on the list) must accept; its first step checks no fixture vault holds a hand-edited LMS `title` or `due` the feed contradicts (if one does, it stops and asks before any code, since frozen references are never regenerated); reviewed by contract-reviewer | `engine/src/ingest.rs` (`sync_tasks`' update branch and its card writer) | 31b |
| **T9** The window and the session | **contract-engineer** (Opus, xhigh): a signed-in mailbox, a kept profile and its deletion; no Opus-high implementer exists and this is the case contract-engineer's description names; after S1–S3 | new `app/src/mail.rs` (window, runner, sign-in detection, `forget`) | 32 (session), 33, 34 |
| **T10** Steps and scripts | **implementer** (Sonnet, high): pure step machines and DOM scripts against synthetic pages | new `app/src/mail_steps.rs`; new `app/assets/mail/**` and their Deno tests | 32 (steps), 35 |
| **T11** Commands and the slot | **implementer**: after T9 and T10 | the commands appended to `app/src/mail.rs`; `app/src/scheduler.rs` | 36, 37 |
| **T12** Deletion and sign-out | **contract-engineer** (Opus, xhigh): `account.rs` is on the list | `app/src/account.rs`; `app/tests/account.rs` | 38 |
| **T13** Docs | **docs-keeper** (Sonnet, medium) | `HANDOFF.md` (the lane, Quinn's queue, bump #1's list, the release guard widened for the Knowbox and the suspended gate, A13's review trigger at 20 paying users, UA's policy read moved out of the MVP rows (`:167`, `:260`, `:383`) and the Pilot and Launch read rows (`:220`, `:230`, `:300`, `:331`) marked suspended, production parity's migration and functions); `docs/reference/{app,engine-commands}.md`; `cloud/supabase/README.md` (the AWS runbook); the legal note's two lines | — |
| **T14** Campus rows and the offer | **mechanical** (Sonnet, low): two struct fields and a marker, fully specified; UA's term dates come from the main session (a public calendar read) | `app/src/scaffold.rs` (`Curated` gains `mail_provider`, `mail_domain`, `terms`); `app/src/onboarding.rs` (the marker); `app/tests/scaffold.rs` | the rows' shape |
| **T15** The page | **console-ui** (Sonnet, medium): `app/static` is theirs | `app/static/{index.html,console.js,console.css}`; `app/tests/static_assets.rs`; `scripts/{settings,wizard}-check.py` | §12.4 |
| **T16** The digest *(optional, last, cuttable)* | **cloud-engineer** | `functions/mail-digest/**` and its `models` row | its own |
| **T17** Integration | **integrator** (Opus, high): the shared single-owner files and the merge train | `app/src/main.rs` (ten commands, recounted); `engine/src/{main,cli}.rs` (`mail-applied`); the CI line that runs the step scripts' Deno tests | the full gate, 0 warnings |
| **T18** Deploy and proof | **main session**: staging pushes, the OTP session, AWS and the sign-ins are the controller's | none | §12.6 |
| **T19** Ruling 12's gate suspended (D25, A13) | **implementer** (Sonnet, high): `app/src/grades.rs` is off the contract list and the change is one constant, one pure `availability_with(gate, row, lms)` seam holding the one branch, and `availability` delegating to it with its signature and its four callers unchanged (§6.2), fully specified and pinned by tests; reviewed by `reviewer`, and the diff goes to Quinn at the signing checkpoint's follow-up because it carries a signed promise. It edits only the grades tests in `app/tests/scheduler.rs` (`:1503-1700`), none of its entitlement tests; if a change would reach one, it stops and goes to contract-engineer. Test first: 39–41 fail before the constant exists. The `Enforced` arm is reachable only at `availability_with`, so only the six predicate tests keep today's assertions against it (test 39) and gain the `Suspended` arm's through `availability`. The command and scheduler tests go through the unchanged callers, which always see `Suspended`: their undated cases are rewritten to the `Suspended` expectation and every uncurated, `NotBlackboard`, no-session and failed-window case keeps today's assertion (tests 40–41: `grades_status_reports_the_gate_and_with_a_date_the_session`, `grades_connect_refuses_without_a_date_and_opens_nothing`, `grades_refresh_refuses_without_a_date_and_reads_no_session`; in `scheduler.rs`, `the_grades_decision_asks_the_predicate_and_never_rederives_it`, `the_skip_order_is_school_then_availability_then_entitlement_then_session_then_window`, `the_slot_records_the_grades_skip_and_runs_and_cleans_up_the_grades_step`). No gate parameter is added to any caller; no assertion is weakened beyond the undated case each names; none is deleted, and the date-and-bump tests are untouched. If a kept assertion cannot pass with the callers unchanged, the implementer stops and asks rather than editing it. May merge ahead of this lane as its own PR, but only together with T13's HANDOFF release-guard line (D18) | `app/src/grades.rs` (the constant, `availability_with`, `availability`); `app/tests/grades.rs`; `app/tests/scheduler.rs` (the grades tests only) | 39–41 |

**Order.** T0 first; nothing commits to SES before S0, no app window task before S1–S3, and no backfill
pace merged before S6. Then T1; then T2, T4, T4b and T5 in parallel (T5 needs no cloud); T3 after T2; T6
and T7 after T5's first commit; T8, T8b and T19 any time after signing (T19 needs no T0 gate and may
merge first, with T13's guard line); T9 then T10 and T11; T12 and T14 in parallel with the app tasks;
T15 once T11's command names are fixed; T13 throughout; T17; T18. T16 only if time allows.

**Checkpoints for Quinn:** at signing (§10 with A13, the *Signing sheet*); after T0 (S0's DMARC result
and header evidence, S1–S3, the sessions log and §13 Q3's revisit, S6's pace on the test tenant); after
T5's, T8's and T8b's contract review and T19's review, with the diffs; before T18, with the whole-branch
review; and, outside this lane, at the 20th paying account (A13's review trigger).

**Size: XL.** The cloud (T1–T4) is about four M tasks; the engine (T5–T8) about one L; the app (T9–T12)
about one L; the page one M. It is the largest MVP lane, and it moves the MVP's exit by its length.

## 15. What signing changes elsewhere

- The signing commit applies §10: the cloud design's Amendment 2026-09-30 and its markers (A13's on
  ruling 12, ruling 10 and §11a's grades row included), the Gmail spec's §5 line, the grades spec's
  markers, VISION's seven edits (A7–A12 and A13's), and CLAUDE.md's slot and actor lines.
- HANDOFF (T13): an MVP lane "Email: the Knowbox" after Gmail connect; Quinn's queue gains, one at a time,
  the AWS account (a spend), the receiving domain's MX in Cloudflare DNS (`in.knowlu.com`), the Knowbox
  secrets on staging, UA's mail fields and term dates for the curated row, and an M365 test tenant for
  S6 (§13 Q8); "Record UA's university-policy read" leaves the MVP queue, and A13's review trigger (the
  20th paying account) joins it; bump #1's list gains §7's drafts; the Pilot gate's release guard names
  the Knowbox and the suspended grades gate.
- `docs/notes/2026-09-29-vision-program.md` gains the email row as an MVP item.
- Production parity, when the Pilot reaches it: the Knowbox migration in step (1)'s ordered list, its
  functions in step (2), its secrets and production's own AWS receiving domain in step (3).

## Review revisions (2026-09-30)

The review's 39 findings (R1–R39, in the order given), plus one the reviser found while re-verifying at
`199cd1f` (R40). Each was checked against the repo at `199cd1f` before anything changed. "Refuted" means
the repo shows a premise is wrong; whatever in the finding still holds was acted on. **No answer of
Quinn's (Q1–Q7, Q5b) was changed:** where a finding argued against one, the answer stays in the spec and
the concern became a §13 question with a recommendation (§13 Q6, Q11). These entries record the spec
before Quinn answered §13 on 2026-09-30; where one says a question is open, §13's answer now governs.

**R1** (critical; D11, §8, test 31: an approved mail amend vs `ingest`). *Partly refuted:* `ingest`
writes with `WriteOpts::default()` (`ingest.rs:689-696`), so it never skips a human-set field; it
overwrites it, and test 31 as drafted would have failed rather than pinned a skip. The defect stands:
ruling 3 (`cloud-design:566-571`) bars a source from overwriting a hand-set field, and nothing filed the
card. *Changed:* D11 (5), F1, §6.1's conditional `ingest.rs` change, §8's `ingest.rs` row, T8b, tests
31 and 31b; the choice is **§13 Q7**, recommended (a).

**R2** (important; §4.9 live template row, A10). *Accepted.* Q5b binds live mail to the card path. The
live template row is a card unless already equal; A10 now says an LMS's own notification mail is
proposed and that notices are shown, not proposed. No question needed.

**R3** (important; §4.11, §6.2, A12, test 36). *Accepted* (`scheduler.rs:845-922`). The `mail` step runs
in the app after the grades capture and before the child-process loop; the engine chain and `slot_argv`
are unchanged; A12 adds a sentence to CLAUDE.md rather than an arrow; new F6; test 36 rewritten.

**R4** (important; §4.8, D10, test 18). *Accepted.* `mail-pull` withholds a backfill or recover batch
until it closes and delivers it whole in ordering-date order; a mailbox's live claims wait behind its
open batch; recover forwards oldest first. Tests 12 and 18 cover the withholding and a split batch.

**R5** (important; §4.11, §4.8, §0). *Accepted.* An open backfill runs on setup's thread whenever the app
is open, outside the slot cadence and the 30-a-slot cap; a closed batch drains past three rounds and asks
for a run now; §0 and §4.11 say how long a backfill takes (about an hour for a few hundred messages,
days for a large mailbox at the daily ceiling, which R11 imposes).

**R6** (important; §4.6, §4.7, D8). *Accepted.* The `knowbox_seen` row is written with the work row,
after the S3 put; the sweep deletes an expired item's `seen` row so reconcile lists it and recover is
accepted. D8's "cost if wrong" corrected; test 7.

**R7** (important; §4.9, D24). *Accepted.* A past event: backfill archives it `imported-past` (Q5's
"recorded and archived"); live or recovered after setup, nothing is written and it is counted; a future
event is a card in both. §4.9's opening says recover after setup follows the live column and the recover
batches that finish a backfill follow the backfill column. Test 23.

**R8** (important; §4.9 "first-day rules apply unchanged"). *Accepted, with a stated reading.* Q5b names
the first-day rule for live mail; the engine's only vault-day-1 rule is commitments' (`cli.rs:850-852`).
A mail card filed on vault day 1, backfill conflict or live, is snoozed to day 2 and never dropped
(§4.8 step 5, test 22d). The other possible reading, the service's doubled cap on an account's first
two judging days, already applies to `mail_claim` through `charge_call` (`20260923000100`), so both
readings hold and no question is needed.

**R9** (important; §13 Q6, D15, §10). *Accepted.* Q6 is binding, so the old recommendation (gate school
mailboxes on `policy_read`) narrowed it and is withdrawn. §13 Q6 now sets out Q6 and ruling 12 side by
side for Quinn, recommending Q6 as answered; D15 says it follows that answer, and the ruling text carries
a bracketed clause for option (b).

**R10** (important; §7). *Accepted.* §7 now says only bulk and list mail from senders outside school and
the LMS is dropped unread, and every other message, personal mail from people included, is read once by
the model host. The table gains `mailbox_hint` and the SNS headers row.

**R11** (important; §11). *Accepted.* New risk 15. Backfill and recover forward at most 10 a minute and
600 a mailbox a day (provisional), stop at the first non-delivery report or send failure with the named
outcome and the `paused_send` state (§4.2 step 5, D14), and T0 gains S6 to measure the pace. S6 must not
risk the founder's own school account: **§13 Q8** asks where it runs.

**R12** (minor; header, §1, §9, §14). *Accepted.* Base is `199cd1f`; §1 re-verified there (the Gmail
pass, judge-once, `ingest`, approvals, the slot, the migrations and the first-day rows rewritten or
added); §9 and §14 no longer wait for #26; D22 is specified against the merged `gmail-read` (R40).

**R13** (minor; D6). *Accepted.* **§13 Q9** asks Quinn to confirm "offer at sign-in, run at the console's
first open", with the C1 reason; D6 points to it.

**R14** (minor; §5.3). *Accepted.* §5.3 now matches D11: an approved amend card's fields as the human
actor, an approved task card as `agent:approvals`, Undo as the student.

**R15** (minor; §4.9 backfill template row). *Accepted.* A backfill template change to an item a feed
supplies writes nothing (the feed is the authority and runs every slot); one to a note no feed supplies
is direct. Test 22.

**R16** (minor; §4.6 step 4). *Partly refuted:* the 72-hour grace is the device's tolerance for a stale
cached answer (`entitle.rs:29-31`), not a payment grace; every entitled function on the server admits
`active` and `trialing` only (`_shared/entitlement.ts:22`). The loss is real, though. *Changed:* a lapsed
account's mail is deleted unread with **no** `seen` row and `lapsed_since` recorded, and the next
reconcile after the lapse reaches back to it (at most 30 days), so recover brings it in (§4.6 step 3,
§4.11 step 3).

**R17** (minor; D21). *Accepted.* The ruling text in §10 now states the divergence from the Gmail spec's
D15, and **§13 Q10** asks it, so Quinn's signature covers it.

**R18** (minor; A5, A11). *Accepted.* A5 adds the Knowbox proof to ruling 10's *Exit* and amends ruling 9
(`:589`); A11 cites VISION `:211-212`.

**R19** (minor; §14). *Accepted.* §14 tells the plan to split T2, T3, T5, T9 and T10 into tasks of about
80 lines or fewer with disjoint files and a roster label each, names the seams, and gives each shared
`_shared/judge_*.ts` file one owning task.

**R20** (critical; D11, §4.9, test 21: judge-once does not cover `due`). *Accepted* (`write.rs:298`,
`provenance.rs:52-59`). The reconciler decides card or direct itself from `Journal::human_set` (a human
`create` counts) for every field it touches, and `mail.rs` writes cards with its own writer through
`write::create`, after `find_pending_amendment`, never through `write`'s judged set (not through
`propose_amendment` either: its "re-judged … judge-once rule" text would be false for mail, and it
cannot carry D21's fields). New F5; D11 (1)–(3); §4.8 step 5; §6.1; tests 20–22 now use `due`. Widening
`JUDGED_FIELDS_TASK` is named in §8 and §13 Q7 as not needed.

**R21** (important; F1, D11, test 31). *Accepted* (`ingest.rs:626`, `:688-695`; `approvals.rs:792`). F1
is rewritten; D11 no longer claims the approved value holds against the feed by itself. Taken to Quinn
as **§13 Q7** before signing: (a) `ingest` honours ruling 3 for `title` and `due`, (b) `ingest` unchanged
and the card warns. *Refuted in part:* option (a) needs no amendment to VISION commitment 5, because
ruling 3, already signed, carries that exception ("no source overwrites a field the journal shows the
student set by hand"). The approvals arm now stops with ruling 11's named error when `config/actor.yaml`
is bad and never falls back to another actor (D11 (4), §5.3, §8, test 31).

**R22** (important; §4.6 binding and authenticity). *Accepted.* A live forward binds only when the
provider hop's highest ARC set is sealed by the provider, verifies and covers a binding header naming
this mailbox, and SES's SPF passes for a return path of the provider's forwarding shape for this
mailbox; M365 also matches the tenant recorded from the setup canary (`forward_hint`). Authenticity
reads only that hop's ARC-Authentication-Results. T0 S0 records the real headers first. Test 2 adds the
forged-header and attacker-tenant cases; risk 10 rewritten.

**R23** (important; backfill and Junk). *Accepted.* Both backfill searches exclude Junk/Spam and
Deleted/Trash (§4.2 step 5, §4.3 step 7); a backfilled original makes a direct write only when the
provider's own delivery-time record shows an aligned pass, otherwise only cards and notices (§4.6 step
6, §4.9). Tests 4 and 22b; risk 1.

**R24** (important; the saved session). *Accepted, all three parts.* (a) One profile per mailbox,
`mail-session-<address_id>`, deleted on that mailbox's Disconnect (D15, §4.12, §5.4). (b) §7 and D15 say
DPAPI covers the cookies, while the pages and mail the site caches sit in the same folder unencrypted.
(c) Knowlu sign-out deletes every mail profile; start-up sweeps a leftover `mail-setup`; a failed delete
is retried at start-up and named in Settings (§4.13, §5.4, tests 32 and 38). Q6's "deleted on Disconnect
and on account deletion" is kept and only added to.

**R25** (important; re-enabling). *Partly accepted.* Q6 binds "re-enable forwarding if turned off", so
a hidden step still turns forwarding back on when it is found off, and now keeps a heads-up saying so.
Forwarding pointing **elsewhere** is not "turned off": a hidden step never changes it and asks instead
(§4.11 step 2, §4.10, §7). The finding's "ask first" and "recover only while active" argue against Q6
(recovering what was missed is Q6's purpose), so they are **§13 Q11**, recommending Q6 as answered.

**R26** (important; §5.1 constraints). *Accepted* (every constraint re-read). §5.1 names each one:
`models.kind` and `usage_daily.kind` widened with `judgments.kind`; `corrections.judgment_kind`,
`rules.kind`, the eval tables and telemetry's list deliberately not; `promote_rules` and
`backfill_correction_judgments` restricted to `origin <> 'knowbox'`. Test 14 pins each; test 28b pins
that the device never reports a mail label (`labels_to_report` already skips it).

**R27** (important; training export). *Accepted.* The claim schema's `new_item` gains `effort_hours` and
`importance` (an addition inside Q5's fixed schema; its kinds and fields are unchanged), and mail notes
are written `needs_enrichment: false`, as Gmail notes are, so no `device`-origin judgment is ever made
from mail text (D9, §5.1's claim, §5.2, §7, test 28c).

**R28** (important; cost). *Accepted.* (a) The allowance is per account per term on `knowbox_accounts`,
Disconnect cannot renew it, three live addresses and six mints in 30 days; its tokens reach
`monthly_spend` and its count never touches the live cap, under a hard per-account ceiling (D3, D23,
§13 Q1). (b) Charge and `enforce_budget` run before the model; a capped item waits a day, at most three
attempts (§4.7). (c) `POST /canary` only to a hint fitting the provider, at most three a day per address
(§4.10). Test 11, test 6; risk 9.

**R29** (important; `knowbox` auth). *Accepted.* `GET /addresses` and `DELETE /addresses/<id>` are
sign-in only, like google-connect's; the rest stay entitled (§6.3, §4.12, test 16b).

**R30** (important; retention and SNS). *Accepted.* "At most a day" became "until read, otherwise up to
about two days plus AWS's lag" in D2, §5.1, §7 and the ruling text (Q3's one-day rule itself is kept).
The receipt rule's only action is the S3 action with its `TopicArn`; every final path deletes its
object and `inbound/` is deleted after the split (§4.6 step 8, §4.7 step 6); test 16 checks the bucket
is empty.

**R31** (important; "deleted unread"). *Accepted.* The mailbox and the entitlement are found from the
notification's `receipt.recipients` before any GET; unmatched and lapsed objects are deleted without a
fetch (§4.6 steps 2–3; test 2 with a spy S3). The reviser adds what the finding implies: the
notification itself carries the headers, which §7 now says.

**R32** (important; fingerprints and dedup). *Accepted.* Both hashes are HMACs under the account's key;
intake and reconcile use one field, the received minute; the service sends what it holds and the device
diffs, so nothing about unforwarded mail leaves it; `seen` is written with the work row after the put
and deleted on expiry (§4.6 steps 7–8, §4.11 step 3, D8, test 7); §7 says "keyed hash". T0 S2 fixes the
sender form both sides use. Risk 16.

**R33** (important; future Date). *Accepted.* Ordering is by the ordering date, the earlier of the Date
header and the received time (D10, §4.8 step 1, §5.2's `mail_asof`); tests 12 and 19.

**R34** (important; deletion and export). *Accepted.* The account purge calls `delete_knowbox_address`
per mailbox (objects by key first) and names all seven tables; Disconnect deletes delivered claims too;
the export gains the mailboxes and claims and never a token, key or hash (D17, §4.13, §5.1, §6.3, test 14).

**R35** (minor; Gmail confirm). *Accepted.* Exact case-folded equality, refusal on a null hint,
`redirect: manual`, the form action on the allow-list, Google's exact sender with aligned DKIM (§4.6 step
5, test 5's cases).

**R36** (minor; stale citations). *Accepted.* §1's `enrich.rs`, `gmail-read` and `migrations_test.ts`
lines are re-verified at `199cd1f`; the moved assertion checks
`origin not in ('gmail_api', 'events', 'knowbox')`, naming all three origins, so it is stronger, never
looser (§5.1, §8).

**R37** (minor; `field: cancelled`). *Accepted.* A `cancelled` claim is always a `kind: amend` card
proposing `status: active → archived` (`status` is amendable); no new field (§4.9, §5.1, §5.2, test 22c).

**R38** (minor; two desktops). *Accepted.* Before any create, the pass looks for a synced note or card
with the claim's `source_uid` and does nothing if one exists (§4.8 step 4, §9, risk 14, test 22e).

**R39** (minor; scripts, untrusted text, label, Rotate). *Accepted.* A script runs only on the provider's
mail origins (D5, §6.2, test 33); every mail-derived string goes through `h()`, with an `<img onerror>`
test (§5.2, §12.4); one label, `mail`, everywhere (D5, §5.4); Rotate keeps the old token accepted until
the new address's canary arrives (at most 7 days), and Gmail re-confirms (D3, test 2).

**R40** (found by the reviser at `199cd1f`; D22). The drafted `quiet: true, reason: "via_knowbox"` would
not be silent: the merged device reads any unknown quiet reason as `Revoked`, "re-connect from settings",
and returns before reading queued items (`cloudmodel.rs:667-675`). D22 now answers a non-quiet empty read
with the undelivered items and a `via_knowbox` field the device ignores; no device change; T4b, test 16c.

**R41** (important; T19, §6.2, tests 39–41; second review, after Quinn's answers). *Accepted, option A.*
The callers call `availability` directly (`grades.rs:355-356`, `:382`, `:400`; `scheduler.rs:428`), so
no caller test can reach an `Enforced` arm while the callers stay unchanged. The `Enforced` arm is
tested only at the pure `availability_with` seam (test 39); the command and scheduler tests' undated
cases are rewritten to the `Suspended` expectation and every other case is kept (tests 40–41); §6.2,
§8's row, D25 and T19 say so. No answer of Quinn's changes.

**R42** (important; §10 A13). *Accepted, both fixes.* Ruling 12's *Policy read* passage
(`cloud-design:699-703`) and *What it costs* (`:714-717`) still said the MVP's grades proof waits on
UA's read; A13 now quotes them as the fourth and fifth texts with replacements, and "every other
sentence stands" is read with the heading marker.

## Signing sheet
Signing this spec signs the amendment items of §10 and the decisions below, in plain words.
- **A1** Cloud design D12: email has two paths, the Knowbox and Gmail OAuth (a marker).
- **A2** Cloud design §13: email by forwarding is decided, on AWS SES rather than Cloudflare Email Routing.
- **A3** Cloud design §3.1: a new row, email by forwarding, judged in the cloud and set up on the device.
- **A4** Cloud design §5.3: the pilot schools' student mail is Microsoft 365, read through the Knowbox (a marker).
- **A5** Rulings 9 and 10: the MVP gains email by the Knowbox; its exit gains the founder's Knowbox proof.
- **A6** Gmail spec §5: forwarded mail is the Knowbox's; a Gmail address with a Knowbox is not read by OAuth.
- **A7** VISION's sources: email from any mailbox that can forward, and Gmail directly.
- **A8** VISION's "Knowlu reads": Knowlu may change forwarding in the student's own mailbox, and sends nothing to anyone else.
- **A9** VISION's data minimisation: what cannot be narrowed is screened before any model reads it; Knowlu keeps findings, never mail.
- **A10** VISION commitment 5 and ruling 3: the one-time backfill is applied and listed with Undo; later mail is proposed.
- **A11** VISION's build order and Beyond: email by the Knowbox; "Outlook" becomes "the Outlook calendar".
- **A12** VISION's telemetry and CLAUDE.md: no forwarded-mail content in telemetry; the app's mail step before the chain; `agent:knowlu.mail`.
- **A13** Ruling 12, with rulings 10, §11a, the grades spec and VISION: the policy-read gate suspended for both kept sign-ins until 20 paying users; the MVP's grades proof runs on `main` and waits on no read.

Decisions Quinn signs:
1. All of email ships in the MVP; no release carries it, or the suspended grades gate, before privacy bump #1 (D1, D18).
2. Each mailbox forwards everything to a private `@in.knowlu.com` address; AWS holds a message until read, about two days at most (D2–D4).
3. Setup is automatic: the wizard says "Next: connect your school email" and the mail sign-in opens with the console (D5, D6).
4. Mail since the term began is applied and listed with Undo; later mail, and any conflict with you or the LMS, is a card; mail cards show the sender (D7–D11, D21, D24).
5. The LMS feed stops overwriting a title or due date you set by hand and files a card instead (D11, T8b).
6. The saved mail sign-in and the kept Blackboard sign-in are on at every school, no school's policy read, until the 20th paying account; then you rule again (D15, D25, T19).
7. Forwarding found off is turned back on and you are told; never for a mailbox you turned off in Knowlu's Settings (§4.11).
8. Limits: 2,000 backfill judgments and $1 per account per term; canaries only after quiet, 72 hours apart; reconcile once a day; Microsoft's pace measured on a test tenant (D14, D23, S6).

