# Microsoft 365 backfill, redesigned: read in the page, upload from the device

**Date:** 2026-10-10, **revised 2026-10-13** for Quinn's ruling of that day. **Status: decisions ruled 2026-10-11/13/18; live gate closed 2026-10-18** (the header Quinn ruled in C3 Q6, session-5 ledger, 2026-10-19). **Signed by Quinn's delegated go, 2026-10-20 (session-5 ledger).**
Nothing here is built. This note amends the signed email forwarding spec
(`docs/specs/2026-09-30-email-forwarding-design.md`, "the spec") for one routine: "Bring in recent mail" on
school Microsoft 365 (§4.2 step 5). The saved session's recover on Microsoft 365 (§4.11 steps 3 and 4) runs
the same code, so it changes too. Live forwarding, its setup, the canary, Gmail's backfill and the device's
reconcile of claims stay as they are, except where §4.12 says otherwise.

**What changed since v1.** v1 recommended option B: read through OWA's own services and **send** grouped
forwards from the mailbox. Quinn adopted B on 2026-10-11 and the probes ran. P1 to P7 passed as reads.
P9, the first scripted send, was not run: a safety classifier stopped the controller writing a script that
reuses the page's bearer to send mail, and the controller will not hand-script that. On 2026-10-13 Quinn
ruled **C3**: read in the signed-in page through OWA's own same-origin services, and upload each original
message's MIME from the device to Knowlu over Knowlu's own authenticated route, into the pipeline forwarded
mail already uses. **The student's mailbox is never sent from and never written to.** B's send step,
grouping, Sent cleanup and send pace are dropped.

**Authority.** The cloud design and its amendments win. After it come the spec as amended, `VISION.md` and
`CLAUDE.md`'s two overriding rules. Read this note with plan T10.2b, T11.2 and §9.5, and with the progress
ledger's rows dated 2026-10-05 to 2026-10-13 (`.superpowers/sdd/2026-09-30-email-forwarding/progress.md`).

**Rule 1.** Nothing below names a school, tenant, mailbox or person. "The tenant" is the founder's school
tenant the probes read. Every number taken from it is evidence and never a constant.

---

## 1. What the live evidence says

### 1.1 The survey (2026-10-07 to 10), unchanged from v1

| # | Fact | What it means |
|---|---|---|
| L1 | A hidden page (`visibilityState` `hidden`) draws **no message-list rows**. | No design may read OWA's DOM list in the app's hidden window. |
| L2 | Conversation rows hide which folder a child message lives in; expanding did not draw children. | A DOM design cannot hold I1. |
| L3 | Page script cannot run OWA's search (untrusted input). | A DOM search design needs CDP, which D5 forbids. |
| L4 | Search results give a page size, never a total. | No count from the DOM. |
| L5 | Folder names are localised display names. | Folder identity must come from ids. |
| L6 | OWA reads mail through JSON services with a bearer it obtains itself. | The page's own services are the way in. |
| L7 | Sign-in detection, address, forwarding settings: proven live. | Setup up to the canary is unaffected. |
| L8 | Outlook starts forwarding minutes after a Save (`73dd845`). | Unchanged. |

The 2026-10-07 review's findings against the DOM scripts (R-a Deleted/Junk children forwarded; R-b Sent
cleanup deleting a conversation row; R-c the attachment never checked; partly drawn conversations; the
address guessed from the folder pane) all fall away under C3: nothing is forwarded, sent or deleted, and
each item carries its own folder id.

### 1.2 The probes (2026-10-11 to 13; controller, kept scratch session, structure and counts only)

| # | Result | What it settles |
|---|---|---|
| P1 | **Fail.** `GetFolder(inbox)` from page script with cookies only: 401 on `/owa/service.svc` and `/owa/0/service.svc`. No `X-OWA-CANARY` cookie exists. | Cookies alone are not enough; decision 3 is needed. |
| P2 | OWA's own calls (`service.svc` actions, `/ows/api`, `/search/api/v1`, `/api/beta/.../$batch`, `startupdata.ashx`) all carry `authorization` (Bearer) plus `x-anchormailbox`, `x-owa-sessionid`, `x-owa-correlationid`, `x-owa-actionsource`, `x-client-version`, `x-req-source`, `x-tenantid`. `service.svc` request bodies travel in the **`x-owa-urlpostdata` header**. Names only recorded. | The helper's header set and body transport (§4.2). |
| P3 | **Pass.** A closure injected before page scripts kept OWA's own `authorization`, `x-anchormailbox`, `x-owa-sessionid`, `x-tenantid`, `x-client-version` and `x-req-source`; `GetFolder(inbox)` with them answered 200 in 849 ms, page hidden. **But** in a minimized Edge window the page lifecycle **froze**: timers and the fetch never resolved until CDP set the lifecycle active. | Decision 3 works. The design must keep the mail page's lifecycle active, or never wait long hidden (§4.3), and verify it in the app's own WebView2 window (W1). |
| P4 | **Pass.** Deep `FindFolder` from `msgfolderroot`: 37 folders, every parent resolves; `GetFolder` resolves `sentitems`, `drafts`, `deleteditems`, `junkemail`, `outbox`, `conversationhistory`, `syncissues`, `inbox`, `archive` by distinguished id. | Folder selection by identity (decision 6). |
| P5 | **Pass.** `FindItem` with `DateTimeReceived >=` the window start over the 4 allowed `IPF.Note` folders (excluded subtrees removed): 1,118 items, 0 failed calls. | An exact `expected` before any upload (I6). |
| P6 | **Pass, one defect.** Inbox items carry `InternetMessageId`, `DateTimeReceived` to the second, `Size`, `From` with SMTP routing; item classes `IPM.Note` and `IPM.Schedule.Meeting.Request`. **A paging element `IndexedPageItemView` with `__type: IndexedPageView:#Exchange` was ignored**: a request for 5 answered 1,117. | The list's fields exist; the paging form must be fixed (§4.2, W4). |
| P7 | **Pass.** `GetItem` with `IncludeMimeContent` on a synthetic message (sent from a founder-owned Gmail): 200, 15 KB of MIME carrying `Authentication-Results`, `ARC-Seal`, `ARC-Authentication-Results`, `Received`, `Message-ID`, `DKIM-Signature`; the MIME's `Message-ID` equals the item's `InternetMessageId`. | The full original can be read in the page, with the headers intake's authenticity check reads. |
| P8 | **Functional only.** A hand Forward-as-attachment arrives with the original attached; its wire shape could not be captured from the page target. | Moot under C3. |
| P9 | **Not run** (2026-10-13). | B's send route is unproven; C3 needs no send. |
| P10 to P13 | Not run. P11 (send size) is moot. P12 (own address) and P13 (Message-ID survives live forwarding) carry over as W6 and W10. | §6. |

## 2. The bar: what a correct backfill must hold

Restated for C3. Every check in §4.8 names its test.

- **I1. Received mail only.** Never Sent Items, Drafts, Deleted Items or any folder under it, Junk Email,
  Outbox, Conversation History or the sync-issue folders. Never a message the mailbox itself sent. Folder
  identity and the `From` address decide this; a display name never does.
- **I2. Only to Knowlu, only for this mailbox.** An original's bytes go only to Knowlu's upload route, under
  the student's own Knowlu session, bound to the account's own mailbox and an open batch of that mailbox.
- **I3. The bytes are the message chosen.** The uploaded MIME's `Message-ID` equals the listed item's
  `InternetMessageId`, and the bytes the device assembled hash to what the page read.
- **I4. Writes nothing in the mailbox.** No send, move, delete, flag, category or read-state change, and no
  rule. The read helper refuses every action outside a read allow-list, at the closure itself.
- **I5. Microsoft's pace.** OWA's service budget is respected (one call at a time, a floor between calls).
  The first throttling answer stops the pass with a named outcome and nothing is retried that hour.
- **I6. A total before the first upload.** Progress shows N of M, in messages.
- **I7. Resumable.** An interrupted pass resumes over the same fixed window (`backfill_since`). Re-running a
  pass uploads nothing the service already holds, and an upload repeated after a lost answer is a no-op.
- **I8. Runs hidden.** No step needs the student's screen, and the page stays able to run (P3) for the whole
  pass, or the pass stops with a named outcome; it never hangs.
- **I9. D5's floor and the privacy rule.** No OS input, no CDP, no debugging port. OWA's token never leaves
  the page; Knowlu's session never enters it. Nothing about a message the service does not hold reaches the
  service except that message itself, uploaded because it is missing.

## 3. The options, as they now stand

| Option | Status | Why |
|---|---|---|
| **A.** Drive OWA's DOM, with a visibility trick and in-process CDP input | **Rejected** (v1; Quinn 2026-10-11) | Fails I1, I3 and I4 by design; needs CDP. |
| **B.** OWA's services in the page, then grouped sends from the mailbox | **Superseded** (Quinn 2026-10-13) | The reads are proven (P3 to P7). The send step is dropped: P9 was not run, and sending from the student's mailbox with a reused bearer is the step no one will script. |
| **C1.** An Outlook inbox rule | Rejected (v1) | Cannot reach mail already in the mailbox. |
| **C2.** The student forwards by hand | **Kept as the fallback** (v1) | Unchecked; cannot hold I1. Shown when the pass reaches a named `not found`. |
| **C3.** B's reads, plus `GetItem` MIME; the device uploads each missing original to Knowlu | **Adopted** (Quinn 2026-10-13) | Holds I1 to I9. Writes nothing in the mailbox, so risks 13 and 15 and T0 S6 no longer apply to M365. |
| **C4.** Graph, IMAP or Knowlu's own Entra app | Out (spec) | Needs a school admin's consent. |

**What C3 costs, from v1's list, and how this note pays it.** A new intake route and binding (§4.5); raw
mail crossing the app process (§4.4, §4.9); the wording of VISION's A8 and spec §7 (§4.9, decision 18).
The authenticity of each original is judged exactly as a backfilled original's is today (§4.5).

## 4. The design (C3)

### 4.1 Shape of one pass

The setup runner's order is unchanged: sign in, step aside (hidden), forwarding, the canary, then the
backfill, which is the recover routine run over the fixed 120-day window. The upload itself does not need
forwarding or the canary; the order stays so the student sees one sequence (decision 21). Only the
routine's Microsoft 365 half changes:

1. **Page able to run.** `m365/alive.js` returns `{visibility, frozen_count, discarded, closure_ready}`
   (§4.3). If `closure_ready` is false the routine waits for OWA's own first service call (a bounded wait in
   Rust, at most 60 s, polling); still false means the named `not found` (`reader`), and the guided step.
2. **Who am I.** `m365/whoami.js` reads the session's own primary SMTP address through the read helper
   (W6 fixes the action) and compares it **in the page** with the kept `mailbox_hint`, passed in. It returns
   `{address_matches: bool}`. A mismatch stops the pass: `the signed-in mailbox is not the one set up`.
3. **Which folders.** `m365/folders.js`: deep `FindFolder` from `msgfolderroot`; keep every `IPF.Note`
   folder **except** the subtrees of the distinguished `sentitems`, `drafts`, `deleteditems`, `junkemail`,
   `outbox`, `conversationhistory` and `syncissues` (decision 6). Returns opaque folder ids only, never
   names; they live in memory for the pass.
4. **How many.** `m365/count.js`: per allowed folder, `FindItem` shallow, restricted to
   `DateTimeReceived` in `[since, until)`, read `TotalItemsInView`. The sum opens the batch as `expected`
   (`POST /batches` with `via: "upload"`, §4.5).
5. **The list.** `m365/items.js` lists each allowed folder, oldest first, by the paging rule of §4.2. Rows:
   `{id, folder, received_at, sender, subject, mid, size, kind}`; `kind` reduces `ItemClass` to `note`,
   `meeting`, `report`, `draft` or `other`. The script drops drafts, reports, `other`, and any row whose
   `From` is the mailbox's own address or a Knowlu sender (canary sender passed as an argument, never a
   literal), counting them `skipped` (decision 7). The routine checks that the rows per folder equal that
   folder's count from step 4; a shortfall is the named `not found` (`list`), never a silent partial pass.
6. **Diff, on the device.** `POST /reconcile` answers the account key, the held fingerprints and (decision
   8) the held Message-ID hashes. Rust computes `HMAC_k(["msg", <id>])` for each row's Message-ID, in the
   exact form intake computes (`mime.ts`'s `messageKey`; shared vectors, §4.11). A row is held when its
   hash is held; a row without a Message-ID falls back to the fingerprint rule (sender and subject, within
   15 minutes). The key never enters the page.
7. **Read and upload, one original at a time, oldest first.** For each missing row: `m365/mime.js` runs
   `GetItem` with `IncludeMimeContent` on the one id, re-checks the item's folder is still allowed and its
   `InternetMessageId` equals the row's (else `left`), and parks the base64 MIME in the closure's MIME slot
   (§4.4). The device pulls it in chunks (`m365/mime_chunk.js`), checks length, SHA-256 and Message-ID
   (I3), and `POST`s the bytes to `/mail/batches/<id>/originals` (§4.5). The answer is `queued`,
   `duplicate`, or a fixed refusal. The bytes are dropped from memory before the next row.
8. **Close** the batch with `POST /batches/<id>/sent {sent}`: the number of originals the route accepted
   (`queued` plus `duplicate`).

**Stops.** A throttling answer from OWA, a closed or lapsed account, a cap reached (§4.7), a page that has
stopped running (§4.3), or a `not found`: each ends the pass with a fixed word, keeps `backfill_since`, and
resumes on the next run. Nothing is retried within the pass beyond §4.5's upload retry.

### 4.2 The read helper, and decision 3 applied to reads only

Decision 3 (Quinn, 2026-10-11, **ruled**) lets `init.js` keep OWA's own bearer in an in-page closure on the
Outlook hosts only, never returned, logged or stored. Quinn's 2026-10-13 ruling narrows its use to **reads**.
This note enforces that inside the closure, not only in the step scripts:

- **Capture.** On the top frame of `outlook.office.com` or `outlook.cloud.microsoft` only, `init.js` wraps
  `fetch` and `XMLHttpRequest.setRequestHeader` before any page script runs. From OWA's own **same-origin**
  `service.svc` requests it copies the P3 header set (`authorization`, `x-anchormailbox`,
  `x-owa-sessionid`, `x-tenantid`, `x-client-version`, `x-req-source`) into closure variables, replacing
  them each time OWA sends newer ones. Off those hosts, in a subframe, or on a non-`service.svc` path it
  does nothing. It never reads cookies.
- **Use.** The closure exposes exactly one non-enumerable, non-writable function to step scripts,
  `read(action, body)`. It refuses (a fixed word, no request made) any `action` outside the **read
  allow-list**: `GetFolder`, `FindFolder`, `FindItem`, `GetItem` and the one read W6 chooses for the
  address. It sends a same-origin `POST` to OWA's own `service.svc?action=<action>` path with an empty body,
  the JSON body URL-encoded in `x-owa-urlpostdata` (P2), the captured headers, a fresh
  `x-owa-correlationid`, and `x-owa-actionsource` as OWA sends it. `GetItem` is allowed only with a body
  whose shape requests properties (P7's), never an update. The closure answers the parsed JSON or a fixed
  word; it never answers a header.
- **The allow-list is pinned twice.** A Deno test reads `init.js` and fails unless the list is exactly the
  five; a source test fails if any M365 script or `init.js` names a write action (`CreateItem`, `SendItem`,
  `UpdateItem`, `DeleteItem`, `MoveItem`, `CopyItem`, `MarkAllItemsAsRead`, `EmptyFolder`, `UpdateFolder`,
  `CreateFolder`, `UpdateInboxRules`, `SetMailboxForwarding`, any `Set`, `Create`, `Update`, `Delete`,
  `Move` or `Send` prefix).
- **Reach.** Anything in the page's own JS world could call `read`, and OWA's own scripts already hold the
  token and the mail; the function adds no capability OWA lacks, and it can only read the signed-in
  mailbox. Message bodies OWA draws are sanitised HTML without script.
- **Validation.** Every answer is checked against the recorded structure (§4.11; key paths, types, enum
  values) before a script uses it; anything else is the named `not found` with a control name.
- **Paging (P6's defect).** P6 sent the EWS XML element name `IndexedPageItemView` as a JSON member, which
  the JSON service ignored. OWA's JSON form carries the view in the request body's `Paging` member
  (`{"__type": "IndexedPageView:#Exchange", "BasePoint": "Beginning", "Offset": n, "MaxEntriesReturned":
  200}`); W4 records the key paths of OWA's own `FindItem` body and proves the member is honoured. **The
  list never depends on paging alone:** `items.js` asks by received-time slices (`DateTimeReceived` in
  `[a, b)`, starting at 7 days), splits a slice whose answer exceeds 200 rows or reports more in view than it
  returned, and checks the per-folder total against step 4. A server that ignores paging still gives
  bounded, complete, ordered answers.
- **Waits.** No script chains page timers. Each step script makes at most a few calls and returns; the
  floor between calls (250 ms) and every longer wait live in Rust (`Clock`). The authority on a stuck call
  is the runner's own timeout in Rust (`mail.rs` `run`, 90 s a step), because a throttled or frozen page
  may never fire a page timer (P3).

### 4.3 Keeping the page able to run (P3)

P3 found that a minimized Edge window froze the page: no timer fired and no fetch resolved until the
lifecycle was set active. The app's mail window is a different host (a hidden Tauri window over WebView2,
its own user-data folder, `mail.rs` `open`/`step_aside`), and Edge's tab freezing need not apply to it. This
note does not assume either way. It designs for both and decides by W1 in the real app window.

1. **The pass never needs page time while idle.** All waiting is in Rust. The page is asked to work only
   inside a step, and each step finishes in seconds. Throttled timers (Chromium aligns a hidden page's
   timers to once a minute after five minutes hidden) therefore cost nothing.
2. **The page reports its own state.** `init.js` counts `freeze` and `resume` events
   (`document.onfreeze`) and notes `document.wasDiscarded`, in the closure. `alive.js` returns them with
   `visibilityState` at the start of the pass and before every 50th row. A non-zero `frozen_count` since the
   last check, or a discarded page, stops the pass as `paused: the mail page was put to sleep`, and the
   routine reloads the window once before trying again next run.
3. **A frozen page is a timeout, never a hang.** The runner's per-step timeout ends any step whose
   evaluation never completes; two consecutive timeouts with `visibility` reading `hidden` stop the pass
   with the same named outcome.
4. **If W1 shows the hidden window freezes or stalls fetches,** the mail window's own WebView2 environment
   (its own user-data folder, so no other window is affected) is created with the browser arguments
   `--disable-renderer-backgrounding --disable-background-timer-throttling
   --disable-backgrounding-occluded-windows`, through the same `additional_browser_args` seam the debug
   port uses today, in release builds too (decision 19). W2 re-runs W1 with them.
5. **If W2 still freezes,** the routine keeps the WebView2 controller's `IsVisible` true while the native
   window is hidden (`with_webview`, `ICoreWebView2Controller::put_IsVisible`), so Chromium never treats the
   page as hidden; W3 verifies. The native window stays hidden and out of the taskbar, and is never moved
   on screen. If that also fails, the pass runs only while the student's main Knowlu window is open and the
   M365 backfill step says so (the last resort; decision 19).

**The live check in the app window** (W1, then the re-proof): the dev build's hidden mail window, on the
kept scratch profile, reached through the debug build's diagnostic port (`crate::debug_port`, test
harness only, never in a release). The harness evaluates only `alive.js` and the read helper's `GetFolder`,
at 0, 6, 12 and 20 minutes hidden, and records `visibilityState`, `frozen_count`, `wasDiscarded`, the
call's status and its latency. No `Page.setWebLifecycleState`, no `Page.bringToFront`, no focus emulation:
the check measures the window as the release will run it.

### 4.4 How the MIME leaves the page

Only the MIME of an item that passed step 5's filter and step 6's diff, and was re-checked in step 7, ever
leaves the page, and only to the app, over the runner's existing script-result channel (S1a: WebView2's
script-completion value, never IPC; the mail window has no capability grant).

- **The MIME slot.** `mime.js` parks the `GetItem` answer's base64 `MimeContent` in the closure, under a
  random handle, and returns `{handle, b64_len, sha256, mid_matches, folder_ok}`. `sha256` is computed in
  the page over the decoded bytes (`crypto.subtle.digest`). The slot holds **one** original at a time: a new
  `mime.js` call clears the old one, and a slot unread for 5 minutes is cleared on the next call.
- **Chunks.** `mime_chunk.js {handle, index}` returns base64 slice `index` of at most **1 MiB** (so each
  completion value stays near 1 MiB), and the last read clears the slot. Rust appends slices, decodes, and
  checks the byte length, the SHA-256 and that the parsed `Message-ID` equals the row's (I3). Any mismatch
  drops the bytes and counts the row `left`.
- **Size.** An item whose listed `Size` exceeds the per-original cap (decision 16, recommended 25 MB) is
  never read: it is counted `left (too large)` and named in the progress view. The cap sits under intake's
  own `MAX_OBJECT_BYTES` (40 MB, `_shared/s3.ts`) and SES's 40 MB receive limit, so nothing uploaded could
  not also have been forwarded. W9 measures the window's real sizes.
- **On the device.** The bytes live in the routine's memory for one row: never written to disk, never in a
  log, a `Stop`, telemetry or an error. A crash loses them, and the resumed pass reads the row again.
- **Never the other way.** Knowlu's session JWT, the fingerprint key and the upload route's URL never enter
  the page; the page never talks to Knowlu.

### 4.5 The upload route, and how an uploaded original is judged

**Where it lives.** A new route on the existing device-facing `mail` function
(`cloud/supabase/functions/mail/`), behind the same `handler.ts` that authenticates the session JWT and
checks entitlement before any route runs. It is not a new function and not a presigned S3 PUT: the bytes
must be checked before anything is stored, and the function already holds the account's mailbox rows. If
C1 shows the platform's request limit is below the per-original cap, the cap comes down to fit; a presigned
`upload/` prefix is the alternative only then (decision 15).

**Opening an upload batch.** `POST /batches` gains one optional key, `via`: `"send"` (the default, today's
behaviour, Gmail's) or `"upload"`. An upload batch is allowed only for an `m365` mailbox (decision 22 keeps
Gmail on sends for now). Everything else about the batch is today's: BW's window rules, `backfill_since`
fixed by the first backfill, a recover no earlier than it and at most 180 days back, at most ten open
batches. An upload batch's nonce is minted and stored as today but **never answered**, and intake's
`bindRows` excludes upload batches, so no mail sent to the Knowlu address can ever bind to one. A migration
adds `mail_batches.via` (`'send'` or `'upload'`, default `'send'`).

**The route.** `POST /batches/<id>/originals`, `Content-Type: message/rfc822`, the body the original's
bytes, at most the per-original cap (a larger declared or streamed body is a 413 that reads nothing more).
In order, each refusal a fixed sentence that echoes nothing:

1. The batch exists, is the account's (through its mailbox, as `batchSent` does), is `via = 'upload'`, is
   not closed, and has no `sent` count yet. The mailbox is live (not `revoked`, not `off`).
2. **Caps** (§4.7): the batch's accepted uploads stay at or under its `expected`; the mailbox's UTC-day
   upload count and bytes stay under the phase's cap. Over a cap is a 429 with a fixed sentence; the
   device stops for the day.
3. **It is one message.** `parseHeaders` reads a `From` with one address, and a `Date` or a delivery stamp.
   A body whose top-level type is not a message (for example an archive or a bare attachment) is a 422.
4. **Inside the batch's window.** The original's delivery time (`mime.ts`'s `deliveredAt`: the provider's
   topmost `Received` stamp, else `Date`) is at or after the batch's `since`, less one day for clock skew,
   and not after now. Else 422, `outside the batch's window`. This is a check forwarding never had: the
   server, not only the device, holds the backfill window.
5. **Not the mailbox's own mail.** `From` equal to the mailbox's `mailbox_hint` (case-folded) is answered
   `own` and not stored (I1, defence in depth; the device already drops these).
6. **Authenticity**, exactly as a backfilled original's is today: `originalAuthenticity(original,
   mailbox, ...)` from `_shared/mail_bind.ts`, the provider's own verifying topmost ARC set, else its
   delivery stamp (the topmost `Authentication-Results`, authserv-id `mx.microsoft.com` with or without
   version `1`, or none), else, with no stamp at all, Exchange's single `X-MS-Exchange-Organization-AuthAs:
   Internal` on a sole From at the mailbox hint's domain, a name under it, or a parent of it no higher than
   the school's registrable domain, never a public suffix (W8 and Quinn's rulings, 2026-10-18). Nothing the
   device says about the message is read. `unaligned` is kept, as it is for a
   batch original today; `mail-process` reads `aligned` only for vendor templates (screen rule 5 and
   `templateClaims`), so an unaligned original goes to the model like any other.
7. **Dedup and hand off** through the **same** code as intake's steps 7 and 8, moved out of
   `mail-inbound/handler.ts` into a shared module both call: `msg_hash` and `fp_hash` under the account's
   key; a held `msg_hash` answers `duplicate`; else the bytes go to `work/<uuid>` and `mail_hand_off`
   writes the `mail_work` and `mail_seen` rows in one transaction, with the batch's id and phase, and
   `receivedAt` the delivery time clamped to now. A failed put or row write is a 500 with nothing kept.
8. **Count and kick.** `queued` and `duplicate` both add one to the batch's `received` (as a batch send's
   duplicate does today). A `queued` upload kicks `mail-process` once (D8), fire-and-forget.

The answer is `{result: "queued" | "duplicate" | "own", received}`. **Nothing logs** but a fixed phrase,
as in every mail function.

**Authenticity and binding, said plainly (decision 17).** A forwarded batch send was bound to the mailbox by
the outer message's DKIM from the mailbox's own domain and the batch nonce in its Subject; the originals
inside were then judged on their own headers. An upload has no outer message and no SES envelope. C3 binds
instead by **who asks**: the student's own Knowlu session, for a mailbox row that account owns, into an
open upload batch of that mailbox that the same account opened, inside that batch's window. Each original
is then judged on its own headers, exactly as before. What this does and does not prove:

- It proves the upload came from the account holder's device (or from something holding their Knowlu
  session) and lands only in that account. It cannot reach another account.
- It does not prove the bytes came from the mailbox. Neither did the batch send: the student's own mailbox
  could attach any message they composed. A provider ARC seal that verifies, or the provider's delivery
  stamp, is the same evidence under both, and its only power is to let a vendor template read the message
  without the model (screen rule 5). A forged original can therefore only put claims into the forger's own
  vault, as it already could. The cost exposure is bounded by §4.7's caps and the model allowance
  (`mail-process/model.ts`), unchanged.
- **Optional consistency check** (decision 17b): Exchange stamps `X-MS-Exchange-CrossTenant-Id` on mail it
  delivers. Where W8 shows it in `GetItem`'s MIME, an original whose topmost value differs from the
  mailbox's `forward_hint` (the tenant the canary recorded) is answered `foreign` and not stored. It is
  unsigned, so it guards against a wrong mailbox, not a forger. Recommend: adopt if W8 shows the header.

### 4.6 Dedupe with forwarded copies

Live forwarding starts at setup, and the backfill window ends at the pass's `until` (now less 15 minutes),
so the two overlap by design. They meet on `mail_seen`'s `msg_hash`, keyed on the original's Message-ID:

- **On the device, before reading MIME** (§4.1 step 6): exact by Message-ID hash, decision 8. A message
  already forwarded live is never read in full and never uploaded.
- **On the server, after upload** (§4.5 step 7): the same `msg_hash`, so a race (forwarded live while the
  pass ran) costs one upload answered `duplicate`, never a second work item.
- **The form must match byte for byte.** `messageKey` takes the first msg-id's text inside the angle
  brackets, whitespace removed, and refuses one that begins with `[`. Rust implements the same rule and a
  shared vector file (§4.11) holds both to it. A row whose Message-ID the device cannot form is uploaded
  and left to the server's dedup.
- **W10** checks the assumption under all of this: that a message forwarded live by Outlook reaches intake
  with its original Message-ID (old P13).
- Gmail's batch sends and its forwarded copies are unchanged.

### 4.7 Pace, caps, progress, resume

- **Microsoft's pace (I5).** Nothing is sent, so Exchange's sending limits and the outbound spam filter no
  longer apply. What remains is OWA's own service budget. The routine makes one service call at a time,
  at least 250 ms apart, and one `GetItem` with MIME per missing row. A throttling answer (HTTP 429 or 503,
  or a `ResponseCode` of `ErrorServerBusy` or any code carrying `BackOffMilliseconds`) stops the pass as
  `paused: Outlook asked Knowlu to slow down`, and the next run is at least an hour later.
- **Knowlu's caps (decision 16).** Recommended:

  | Cap | Backfill | Recover |
  |---|---|---|
  | One original | 25 MB (larger: `left (too large)`, never read) | 25 MB |
  | One batch | its `expected` | its `expected` |
  | One pass (device) | none | 30 originals (today's `SLOT_CAP`), 10 minutes of the slot |
  | One mailbox, one UTC day (server) | 1,500 originals and 1.5 GB | 150 originals and 300 MB |
  | One window (device, before the first batch) | 3,000 messages: if the 120-day count is larger, the device moves `since` later to the latest date whose count fits, before `backfill_since` is fixed | n/a |

  The model allowance for backfilled mail (`mail-process/model.ts`, `allowance`) is unchanged and still
  governs model spend. A server cap reached is a 429 with a fixed sentence; the pass stops for that UTC day
  and resumes the next (I7). W9's sizes set the byte numbers before release.
- **Total and progress (I6).** `expected` is §4.1 step 4's exact count, in messages. `handled` is held,
  uploaded (`queued` or `duplicate`), skipped or left, in messages. The progress view shows "N of M
  messages", the bytes uploaded, and a time left from the pass's own observed rate. On Gmail the unit stays
  conversations (plan T11.2).
- **Resume (I7).** A resume reads `backfill_since` and diffs again; every original queued earlier is in
  `mail_seen`, so it is held. An upload whose answer was lost is retried at most three times (2, 4, 8 s,
  in Rust), and a retry of one that did land is answered `duplicate`. Each pass opens its own batch and
  closes it with its own `sent`; a pass that dies leaves its batch to the sweep's time-out, as today.
- **The saved session (§4.11).** Recover runs the same pass over its three-day (or lapse) window, capped
  as above, inside the slot's mail step and before the engine chain.

### 4.8 Safety, check by check

| Check | How C3 holds it | Its test (§4.11) |
|---|---|---|
| I1 | Folder ids from distinguished ids, never names; each item's folder checked at list time and again in `mime.js`; `From` against the session's own address on the device and against `mailbox_hint` on the server; drafts and reports dropped. | `m365_items_never_lists_an_excluded_folder`, `m365_mime_leaves_an_item_moved_to_deleted_items_since_the_list`, `m365_the_students_own_message_is_never_uploaded`, `upload_refuses_the_mailboxs_own_mail` |
| I2 | The route binds by session, owned mailbox and an open upload batch; the page never talks to Knowlu. | `upload_refuses_another_accounts_batch`, `upload_refuses_a_send_batch`, `upload_refuses_a_closed_or_counted_batch`, `no_m365_script_names_a_knowlu_host` |
| I3 | Message-ID re-checked in the page; length, SHA-256 and Message-ID re-checked on the device. | `m365_a_message_id_that_changed_is_left_never_uploaded`, `a_chunk_mismatch_drops_the_original` (Rust) |
| I4 | The closure's read allow-list; the write-action source tripwire; W5 (no read-state change). | `init_read_refuses_every_action_off_the_allow_list`, `m365_scripts_and_init_name_no_write_action` |
| I5 | One call at a time, the floor in Rust, stop on the first throttling answer. | `m365_a_throttling_answer_stops_the_pass` (Rust) |
| I6 | `TotalItemsInView`, summed; the per-folder rows checked against it. | `m365_count_sums_allowed_folders_only`, `m365_a_short_list_is_not_found_never_a_partial_pass` |
| I7 | Fixed window; exact diff; server dedup; bounded retry. | `a_resumed_m365_pass_uploads_nothing_held`, `upload_twice_is_one_work_item` |
| I8 | No drawing; page state reported; timeouts named; W1 to W3. | `a_frozen_page_is_a_named_pause_never_a_hang` (Rust), live W1 |
| I9 | Token in the closure, never answered; Knowlu's JWT and key never in the page; bytes never on disk or in a log. | `no_m365_script_result_carries_a_bearer_or_jwt_shape`, `init_wrapper_is_inert_off_the_mail_hosts_and_in_subframes`, `upload_logs_only_fixed_phrases` |

### 4.9 Privacy, D5 and VISION

- **No new class of data leaves the device.** An uploaded original is the same whole message live
  forwarding would have sent for the same mailbox, chosen by the same rule (received mail in the
  mailbox's own folders). It is read by the same processors (our function's memory, S3 `work/`, the
  pinned model host for what the screen keeps) and kept as long (deleted when processed, else the bucket's
  one-day `work/` expiry; the keyed hashes 180 days). The path differs: device to Knowlu's API over TLS,
  instead of Microsoft to SES to S3 `inbound/`. Amazon SES and SNS never see an uploaded original, so its
  sender, recipients and subject are in no notification. The sub-processor list is unchanged.
- **New on the device:** the app process holds one original in memory at a time, never on disk. It is the
  student's own mail, on their own computer, already in the same mail profile's cache (spec §7's saved
  sign-in paragraph).
- **Nothing about an unheld message reaches the service** except the message itself, uploaded because it
  is missing, as forwarding would have. `POST /reconcile` still carries only a window; its answer adds the
  held Message-ID hashes (decision 8), which are the account's own and expose nothing new.
- **The mailbox:** nothing in it changes. No mail is sent, so none reaches anyone (VISION commitment 3
  holds trivially for the backfill). `GetItem` is expected to leave read state alone; W5 proves it.
- **D5.** Quinn's 2026-10-11 amendment (decision 2, ruled) already covers this: "in-page only: the page's
  DOM and the page's own same-origin services, under the page's own sign-in". The floor is unchanged: no
  CDP, no debugging port, no OS input. Decision 19's browser arguments are none of these.
- **VISION's "Knowlu reads" bullet (A8)** says Knowlu writes, in the mailbox, "older or missed mail
  forwarded to Knowlu's own address (whose Sent copies it removes)". For Microsoft 365 that is no longer
  true, and the change is a reduction: Knowlu then writes less in the mailbox. Proposed text (decision 18):
  "... older or missed mail forwarded to Knowlu's own address (whose Sent copies it removes) **or, for
  school Microsoft 365, read on this computer and uploaded to Knowlu without changing the mailbox** ...".
- **Spec §7** changes in three places: the mailbox-actions sentence (for Microsoft 365: "reads older or
  missed mail through Outlook's own services and uploads it from this computer to Knowlu; it sends and
  deletes nothing", with "(Gmail)" marking the forward-and-delete clause); the data table gains its own row,
  "| An older message read on your computer (Microsoft 365) | uploaded from your computer to our function's
  memory | for the length of that call |", leaving the S3 row as it is (signed 2026-10-13 with this split row); and
  the lawyer packet's automated-access line becomes "reading through Outlook's own services with the saved
  sign-in; no mail is sent". `PRIVACY_VERSION` still does not move here.
- **Telemetry:** none new, and no new row shape.
- **Review level.** The read closure is credential-adjacent and the MIME path carries mail content, so
  both get Opus review (`contract-reviewer`), as R4 asked for scripts that change a mailbox.

### 4.10 What retires

| What | Fate |
|---|---|
| `m365/search.js`, `m365/list.js`, `m365/forward.js`, `m365/sent_cleanup.js`, their tests and synthetic pages | Deleted; dropped from `mail.rs`'s script list |
| `recover.rs`'s M365 span narrowing, conversation reading, forward path and its use of the send pace | Replaced by the upload path; Gmail's half untouched |
| The send pace (10 a minute, 600 a UTC day) and Sent cleanup **for M365** | Gone for M365; kept for Gmail (`mail_steps.rs` `Routine`, `CLEANUP_*`) |
| T0 S6 (test-tenant send pace) **for M365** | Not needed; still Gmail's gate |
| Risks 13 (Sent cleanup) and 15 (outbound burst) **for M365** | Closed for M365 |
| v1's decisions 4 (Sent copies) and 5 (grouping, send pace); probes P8 to P11 | Void |
| D13's guided placeholder for the M365 backfill step | Removed when C3 ships |
| **Not retired:** M365 forwarding scripts, the canary, Gmail's scripts and batch sends, intake's batch-send path | Unchanged |

### 4.11 Tests (test first)

- **Deno, step scripts and `init.js`.** A fake `fetch` (and fake `XMLHttpRequest`) routes by `action` and
  serves answers built from the **recorded structure**: key paths, types and enum values from the probes,
  synthetic values only, no real address, subject or id (rule 1). Tests assert on the requests each script
  makes (action, folder ids, restriction, slices, paging member) and on every refusal and `not found`.
  `init.js` tests: capture only on the two hosts' top frame and only from same-origin `service.svc`;
  `read` refuses every action off the allow-list without a request; no answer, error or property ever
  carries a captured header; the MIME slot holds one original, clears on the last chunk and after 5
  minutes; `freeze` and `resume` are counted.
- **The structure is pinned.** A test fails if a script reads a key path the recording lacks. The
  recording is a fixture of this lane (`app/assets/mail/m365/testdata/owa_shapes.json`), not one of the
  frozen references; a live change is caught by the smoke and re-recorded.
- **Shared vectors.** `cloud/supabase/functions/_shared/testdata/message_key_vectors.json`: Message-ID
  header values (angled, unangled, folded, with comments, `[`-led, absent) and the expected `messageKey`
  and `HMAC_k` under a fixed test key. The Deno `keys.ts` test and a Rust test in `recover.rs`'s M365 path
  both read it, so the two forms cannot drift. Not a frozen reference.
- **Rust (app).** In the M365 path: exact-id diff and fingerprint fallback; chunk assembly with length,
  SHA-256 and Message-ID checks; the size cap; per-folder total check; throttling, cap (429), lapsed and
  timeout stops; the frozen-page pause; resume; upload retry. `mail_client.rs`: `open_batch` with `via`,
  `upload_original` against a server on `127.0.0.1:0` (no egress), with a body never logged or put in an
  error. A test that no M365 code path writes an original to disk (the routine takes no path argument and a
  source test finds no `fs::` in it).
- **Cloud (Deno).** The upload route: every refusal in §4.5's order; ownership; `via`; caps per batch and
  per UTC day, compare-and-set under concurrency; one-message and window checks; the `own` and optional
  `foreign` answers; authenticity from the original alone; duplicate on a second upload; `received`
  counting; the kick; fixed-phrase logging only. Intake's tests stay green after the hand-off code moves to
  the shared module, and a new one shows `bindRows` never offers an upload batch. `POST /reconcile`
  answers held Message-ID hashes in its window. The migration test covers `via` and the day counters.
- **Tripwires.** The write-action list (§4.2); no `Input.`, `CallDevToolsProtocolMethod`, `SendInput`,
  `setWebLifecycleState` or `remote-debugging` in `app/src/mail*` or `app/assets/mail/**` (spec test 34,
  widened); no Knowlu host or `/functions/v1` in `app/assets/mail/**`.
- **Live.** W1 to W10 before the build (§6); C1 and the T18c re-proof after it.

### 4.12 What this changes in the signed spec, plan and VISION

| Where | Change |
|---|---|
| D5 | Already amended (decision 2, ruled 2026-10-11): in-page only, floor unchanged |
| §4.2 step 5 | M365: count by the services, upload missing originals, no sends; Gmail unchanged |
| §4.6 | A second way in: the upload route (§4.5), binding by session and upload batch; steps 6 to 8 shared |
| §4.11 steps 3 and 4 | M365 lists by the services and uploads; the slot cap counts originals; `/reconcile` answers Message-ID hashes |
| §5.1 | `mail_batches.via`; the per-mailbox UTC-day upload counters |
| §6.3 | `POST /batches` takes `via`; new `POST /batches/<id>/originals` |
| §7 | The mailbox-actions sentence, the whole-message row, the lawyer packet line (§4.9) |
| Risks 13 and 15 | Closed for M365 |
| VISION A8 | Amended text (decision 18), Quinn signs |
| Plan T10.2b, T11.2 | The M365 half is replaced by §7's tasks; Gmail's half unchanged |
| Plan §9.5 | Rows for v1, the 2026-10-13 ruling and this revision |
| Spec test 34 | Widened as in §4.11 |

## 5. Decisions for Quinn

v1's numbers are kept so the ledger's references stay true. **Ruled** decisions are marked with the date;
**void** ones are kept for the record. Decisions 11 (its extension), 16, 17, 18 and 19 are needed before
the build starts; the rest can follow W1 to W10.

1. **The mechanism.** **Ruled 2026-10-11 (B), superseded and ruled 2026-10-13: C3.** M365 DOM backfill
   scripts retired; C2 kept as the fallback.
2. **D5's wording.** **Ruled 2026-10-11:** "in-page only: the page's DOM and the page's own same-origin
   services, under its own sign-in"; never CDP, a debugging port or OS input.
3. **The bearer closure.** **Ruled 2026-10-11; narrowed to reads 2026-10-13.** This note enforces "reads
   only" inside the closure with a five-action allow-list (`GetFolder`, `FindFolder`, `FindItem`,
   `GetItem`, and W6's address read). **Confirm** that the enforcement point is the closure itself.
4. **Sent copies.** **Void** (no sends).
5. **Grouping and send pace.** **Void**; decision 16 replaces it.
6. **Which folders.** Every `IPF.Note` folder under the root except the subtrees of Sent Items, Drafts,
   Deleted Items, Junk Email, Outbox, Conversation History and the sync-issue folders, by distinguished id.
   Archive and the student's own folders included. P4 and P5 show it resolves. **Recommend: yes.**
7. **Which items.** Messages and meeting messages in; drafts, reports, the student's own mail and Knowlu's
   own canaries out, counted `skipped`. **Recommend: yes.**
8. **Exact reconcile.** `POST /reconcile` also answers held Message-ID hashes; the device matches on them
   first and falls back to fingerprints. P7 showed the MIME's Message-ID equals the item's; W10 confirms
   live forwarding keeps it. **Recommend: yes** (it is now what keeps a forwarded message from being read in
   full on the device).
9. **The progress unit on M365.** Messages, exactly. **Recommend: yes.**
10. **Recover too.** The saved session's daily recover on M365 uses the same pass, under decision 16's
    caps. **Recommend: yes.**
11. **R5 widened for probes.** **Ruled 2026-10-11** for P1 to P13. **New ask:** extend the same scope to W1
    to W10 (all read-only in the mailbox; W7 and W8 read synthetic messages the controller sends from a
    founder-owned account to the school mailbox, as P7 did; W10 is a count-only staging query) and to C1
    (synthetic uploads to staging, after the build). **Recommend: yes, scoped exactly so.**
12. **C3.** **Ruled 2026-10-13: adopted.**
13. **While C3 is built.** **Ruled 2026-10-11:** the M365 backfill step is shown as the guided step; the
    T18c proof continues on what works.
14. **Gmail hidden.** G1 checks whether Gmail draws rows hidden. **Recommend: run G1 now** (it is
    independent of C3), and redesign nothing for Gmail yet.
15. **Where the upload lands.** A route on the existing `mail` function (`POST /batches/<id>/originals`),
    checking bytes before storing them, rather than a presigned S3 PUT. **Recommend: yes**; revisit only if
    C1 shows the platform's request limit is below the per-original cap.
16. **Caps.** 25 MB an original (larger is `left`, never read); per mailbox per UTC day, 1,500 originals and
    1.5 GB in backfill, 150 and 300 MB in recover; 30 originals a slot; a first backfill window of at most
    3,000 messages, the device moving `since` later to fit before `backfill_since` is fixed. The model
    allowance is unchanged. **Recommend: yes,** with the byte numbers revisited after W9.
17. **Binding and authenticity for uploads.** Bind by the student's Knowlu session, a mailbox their account
    owns, and an open upload batch of it, inside its window; judge each original on its own headers exactly
    as a backfilled original is today (`originalAuthenticity`); refuse the mailbox's own mail. Accept that
    this proves the uploader, not the mailbox, as the batch send never proved the attachments either (§4.5).
    **Recommend: yes.** **17b:** also refuse an original whose `X-MS-Exchange-CrossTenant-Id` differs from
    the canary's recorded tenant, if W8 shows the header. **Recommend: yes, if W8 shows it.**
18. **The text.** VISION A8's bullet and spec §7 as drafted in §4.9; the lawyer packet line. **Ruled
    2026-10-13: signed, with the upload as its own data-table row** (§4.9). The VISION and spec §7 edits
    land in the build's commits, so they never describe behavior the branch does not have.
19. **Keeping the page running.** W1 decides. If the hidden app window freezes or stalls, the mail window's
    own WebView2 environment gets `--disable-renderer-backgrounding --disable-background-timer-throttling
    --disable-backgrounding-occluded-windows` in release builds (W2); if that fails, the controller stays
    "visible" while the window is hidden (W3); the last resort runs the pass only while Knowlu's main window
    is open. **Recommend: yes, in that order,** and no change at all if W1 passes.
20. **An original over the cap.** Counted `left (too large)` and named in the progress view; never cut down
    to its text and headers. **Recommend: yes** (a cut original would fail its own authenticity check and
    make a second, divergent intake path).
21. **The order of setup.** The upload does not need forwarding or the canary, but the backfill keeps
    waiting for the canary in this build (a7f1ca0's rule), so setup reads as one sequence. **Recommend:
    keep the order now.** **Open for the Pilot:** many school tenants block automatic external forwarding;
    C3 could serve them as an "upload-only" mailbox (the daily recover alone, by the saved session). Not
    in this build; Quinn's call when there is evidence.
22. **Gmail stays on sends.** `via: "upload"` is M365-only. Gmail's own "Show original" page could later
    feed the same route; not in this build. **Recommend: yes.**

## 6. Remaining live checks (controller; read-only in the mailbox)

**Rules for every check.** The kept scratch proof profile's M365 session. W1 to W3 run in the **dev app's own
hidden mail window**, reached through the debug build's diagnostic port (`crate::debug_port`, harness only,
never in a release); the rest may use that window or the Edge probe profile. No `Page.bringToFront`, no
focus emulation, no `Page.setWebLifecycleState`, no input events. **No write action** is ever called; every
script goes through the read allow-list. Each check returns only the fields its pass condition names: counts,
booleans, statuses, key paths and types, never values from a real message. W7 and W8 read only synthetic
messages the controller sends from a founder-owned account. A failed check stops the checks that depend on
it.

| # | Check | Pass condition |
|---|---|---|
| W1 | **The app's hidden window keeps running.** Open the hidden mail window as the slot does (`open_hidden`), signed in. At 0, 6, 12 and 20 minutes hidden, run `alive.js` and one `GetFolder(inbox)` through a harness copy of the read closure. | Every call answers 200 within 5 s; `frozen_count` 0; `wasDiscarded` false. Record `visibilityState` each time. A freeze or a stall means W2. |
| W2 | (Only if W1 fails.) W1 again with decision 19's browser arguments on the mail window's environment. | As W1. A failure means W3. |
| W3 | (Only if W2 fails.) W1 again with the controller kept visible while the window is hidden. | As W1, and the window is never on screen or in the taskbar. |
| W4 | **Paging.** Record the key paths (no values) of OWA's own `FindItem` request body from its `x-owa-urlpostdata` while the inbox loads; then `FindItem` on the Inbox with that paging member, `MaxEntriesReturned` 5. | 5 rows, and the answer's range fields (`TotalItemsInView`, `IncludesLastItemInRange`) present. If paging is still ignored, the slice rule (§4.2) stands alone; record that. |
| W5 | **Reading changes nothing.** On an unread synthetic message: `IsRead` and the Inbox's unread count before; `GetItem` with MIME; both again after. | `IsRead` still false; unread count unchanged. |
| W6 | **The session's own address.** Find the read action that answers the primary SMTP address (`GetMailboxByIdentity` or the configuration answer OWA already loads); compare it in the page with the kept `mailbox_hint`. | `{matches: true}`, and the action named for the allow-list. |
| W7 | **MIME through the result channel.** A synthetic message with a 3 MB attachment; `mime.js` then `mime_chunk.js` in 1 MiB slices into a harness that reassembles in memory. | Length, SHA-256 and Message-ID all match; the slot reads empty afterwards. Nothing written to disk. |
| W8 | **Authenticity of Exchange's MIME.** Run `originalAuthenticity(…, 'm365', …)` locally (Deno, controller machine) on W7's synthetic MIME and on P7's. Record which arm answered (ARC verified, or the delivery stamp), and whether `X-MS-Exchange-CrossTenant-Id` is present (boolean). | `aligned` for both. If the ARC arm fails on Exchange's regenerated MIME, record it: the stamp arm then carries every M365 upload. **2026-10-18:** the stored MIME carries no ARC set; external mail's stamp reads `mx.microsoft.com 1`, which the stamp arm then refused (fixed: that authserv-id is read); intra-tenant mail carries no stamp and no ARC, only `X-MS-Exchange-Organization-AuthAs: Internal`. Quinn's ruling: such an original is `aligned` when exactly one AuthAs field reads `Internal` and its sole From is on the hint's domain (Exchange strips these fields from external mail). |
| W9 | **The window's bytes.** Over the allowed folders and the 120-day window: the sum of `Size`, the largest `Size`, and the count over 25 MB. Three numbers. | Recorded; they set decision 16's byte caps. |
| W10 | **Live forwarding keeps the Message-ID** (old P13). For W7's synthetic message, which forwarding delivered live to staging, a count-only staging query on `mail_seen` for its `msg_hash`. | Count 1. |
| G1 | **Gmail, hidden** (v1, unchanged). | Row count recorded. |
| C1 | **After the build, staging only.** Upload W7's synthetic original to a recover batch of the scratch profile; upload it again; upload a synthetic 26 MB body. Count-only queries. | First `queued` and one `aligned` work item; second `duplicate`; the large one 413; nothing logged but fixed phrases. |

Order: W1 (then W2, W3 if needed), W4 to W9 in any order, W10 after W7, G1 any time. **The build starts
when W1 (or W2 or W3), W4, W5, W6 and W8 pass.** W9 sets numbers; W10 decides decision 8's emphasis, not
whether C3 proceeds. C1 and the T18c re-proof come after the build.

## 7. Tasks (the plan writes them in full)

No task touches a file on the contract list (`engine/src/...`, `app/src/{credentials,account,updates}.rs`,
the oracle, sync and entitlement tests, `engine/tests/fixtures/**`). R2 goes to `contract-engineer` anyway,
under CLAUDE.md's "No agent fits?": a silent error there could leak a credential or student mail.

| Task | Files | Agent | Why |
|---|---|---|---|
| R0 Live checks | none (a report under `docs/reports/`) | main session (controller) | live sessions and staging are the controller's |
| R1 Recorded structure | `app/assets/mail/m365/testdata/owa_shapes.json` (synthetic values) | main session | written straight from R0's output |
| R2 Read closure, allow-list, MIME slot, lifecycle counters | `app/assets/mail/init.js` | `contract-engineer` | credential-adjacent; a silent error leaks a token or mail |
| R3 Failing script and `init.js` tests | `app/assets/mail/m365_read_test.ts`, `app/assets/mail/init_test.ts` | `test-writer` | §4.11, red first |
| R4 The scripts | `app/assets/mail/m365/{alive,whoami,folders,count,items,mime,mime_chunk}.js`; delete the four retired scripts and their tests | `implementer` | specified; checked by R3 |
| R5 Shared vectors | `cloud/supabase/functions/_shared/testdata/message_key_vectors.json` and the Deno test reading it | `test-writer` | one fixture both sides read |
| R6 Migration | `cloud/supabase/migrations/<new>_mail_upload.sql` (`mail_batches.via`, per-mailbox UTC-day upload counters and their compare-and-set RPC), `migrations_test.ts` | `cloud-engineer` | `cloud/` |
| R7 Upload route, shared hand-off, reconcile hashes | `cloud/supabase/functions/mail/{routes,handler}.ts` (or a new `upload.ts`), `_shared/mail_handoff.ts` (moved from `mail-inbound/handler.ts`), `mail-inbound/handler.ts`, `mail-inbound/index.ts` (`bindRows` excludes upload batches), tests | `cloud-engineer` | `cloud/`; touches intake |
| R8 Device client | `app/src/mail_client.rs` (`open_batch` with `via`, `upload_original`), `app/tests/mail_client.rs` (new, so R9 keeps `app/tests/mail.rs` to itself) | `implementer` | specified HTTP against a local listener |
| R9 The routine | `app/src/mail/recover.rs` (M365 path), `app/src/mail_steps.rs` (M365 read pace and caps), `app/tests/mail.rs` | `implementer` | orchestration through existing seams; uses R8 behind a trait |
| R10 The window | `app/src/mail.rs` (script list; decision 19's arguments or controller visibility, only as W1 to W3 decide) | `implementer` | small, specified by the live result |
| R11 Progress and Settings copy | `app/static/**` (the backfill step's "N of M messages", bytes, the named pauses; the guided placeholder removed) | `console-ui` | `app/static` |
| R12 Docs | spec amendment, VISION A8 text, privacy bump list, `docs/reference/app.md`, `cloud/supabase/README.md`, plan §9.5 | `docs-keeper` | after Quinn signs 18 |
| Review R2 | `init.js` | `contract-reviewer` | credential-adjacent |
| Review the rest | R4 to R11 | `reviewer` | cheaper agents' work before a push |
| Re-proof | T18c's M365 backfill and a recover slot | main session, with Quinn | live |

Parallel: R2, R3, R5, R6 and R8 at once; R4 after R1 and R3; R7 after R5 and R6; R9 after R4's interfaces
and R8 (fake scripts and a fake client meanwhile); R10 after R0; R11 any time after the wording is
agreed; R12 last.

## 8. Open risks

1. **Microsoft reshapes OWA's services.** A shape change is caught by validation and becomes a named
   `not found`, then the guided step; never a wrong read or an upload of the wrong message (I3). The live
   smoke before each release catches it.
2. **The read closure is the most sensitive code in the lane.** It gets `contract-engineer`,
   `contract-reviewer` and tripwires. It can only read the signed-in mailbox.
3. **A hidden window that freezes** in some WebView2 runtime or Windows power mode W1 did not see. The pass
   names it and resumes later; decision 19's ladder handles the systematic case.
4. **OWA's service budget** under a 1,000-message backfill. One call at a time with a floor, and a stop on
   the first throttling answer; W1 to W9 add a few hundred calls at most.
5. **Exchange's regenerated MIME may not carry a verifying ARC set** (W8). The delivery stamp arm then
   carries authenticity, as it already does for a batch original.
6. **Upload size against the platform's request limit** (C1). The per-original cap comes down to fit, or
   decision 15 is revisited.
7. **Upload-only cost exposure.** Anyone holding a student's Knowlu session could upload into that account.
   Decision 16's caps and the model allowance bound it to that one account.
