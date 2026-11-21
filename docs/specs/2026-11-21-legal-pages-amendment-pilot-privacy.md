# Legal pages amendment: pilot privacy (2026-11-21, r2)

**Status:** Quinn's rulings of 2026-11-21, recorded here; r2 answers review r1
(`docs/reports/2026-11-21-pilot-privacy-note-review-r1.md`). Amends `docs/specs/2026-11-17-legal-pages-rewrite.md`
("the rewrite"); where they disagree, this note wins. **Size:** Contract (the privacy page, `PRIVACY_VERSION`,
sync, a migration). **Lands with:** the launch privacy bump (the rewrite's bump B; roadmap PR #86 renumbers bumps
when it merges, and this note follows that numbering then). Quinn: "It's fine if it doesn't exactly reflect the
app right now, because it will by launch." Section 6 is the code that makes it true by then.

## 1. Quinn's rulings

| # | Ruling (Quinn's words, 2026-11-21) | What it means here |
|---|---|---|
| Q-P1 | "I like the 'this computer only' setting." | An **account** setting, *Where your plan is kept*: **Your account** (default, today's behavior) or **This computer only**. |
| Q-P2 | "Make kept sign-ins opt-out. Have it be recommended." | Every kept sign-in is offered at setup as a visible choice, already on and marked **Recommended**, with what it does and what it costs. Off at setup or later in Settings. Worded by kind, never by count or platform. |
| Q-P3 | "Delete forwarded mail as soon as it's processed." | A forwarded message is deleted the moment Knowlu is done with it, success or final failure. Amazon's one-day expiry is only the backstop for a message Knowlu never reached. |
| Q-P4 | "Drop email records when Gmail is disconnected." | Disconnecting Gmail deletes its decision records. Read the same way for a forwarded mailbox (its decision records go with it). **Confirm at the checkpoint.** |
| Q-P5 | "Add the 'who can access your data' section." | §3.5. |
| Q-P6 | "Write the Pilot Privacy Statement." | `site/pilot-privacy.html`, §4. |
| Q-P8 | "Add a disclaimer at the top of the privacy policy stating that it will be prone to changing often until the launch and pilot in January." | §3.0: a notice above the one-line promise. It does not loosen *Changes to this policy*: a material change still asks first. |
| Q-P7 | "Fit the terms of service, privacy policy, and the pilot privacy statement to the new style of the site." | §5. |

**Signatures Q-P1 needs (I8).** It reverses part of the cloud design's signed Ruling 2, which removed the sync
switch because a per-machine promise "the product cannot keep across a student's own machines is not trust".
D1 answers that reason (one account-wide setting the server enforces). Two amendments, for Quinn's signature:
the cloud design's Ruling 2 and VISION commitment 2, both with this text: *"A student may choose **This computer
only** for the account: then no computer's vault is stored in the account, the service refuses to store one,
and judgments still run one call at a time. Each computer then keeps its own plan, and a new computer starts
empty."* **Until both are signed:** C30 does not start, and the This-computer-only text (3.1's clause, 3.2, the
mentions in 3.5 and §4) is held out of the page; the rest of this note publishes without it.

## 1a. Addendum after review r2 (2026-11-21)

- **Q-P9, Quinn: "Once this is done, just publish even if the app doesn't reflect it. Nobody is using it yet,
  so we can just update it before launch in January. Plus, nobody can even agree to the privacy policy yet."**
  This supersedes §1's hold on the This-computer-only text and the rewrite's "A before B, one bump per
  release" for this publication: bump A, bump B and this amendment publish together now, by hand deploy on
  Quinn's word, and the page is corrected again before launch. The two §1 amendments (cloud Ruling 2,
  VISION commitment 2) are still owed for Quinn's signature before C30 is built.
- **Review r2's must-fixes F1–F4 and F7 applied:** kept sign-ins worded by kind everywhere; the four
  remaining vault-in-account sentences qualified (and the terms' matching clause); the zyBooks/VHL replies
  added to the This-computer-only list; "open-weights" dropped from the statement; 3.5's consent promise
  covers "your email or anything found in it"; the sale paragraph points at *Who else touches it*.
- **D-2** (Q-P4) applied as written in D6. **D-3**: the pilot form is "deleted when the pilot ends, or sooner if
  you ask" (page and statement). **D-4**: the mailbox-disconnect sentence is scoped (fingerprints and untaken
  items keep their own times).
- **Placeholders filled without new facts:** both backup and sign-in-log windows read "30 days", which holds
  on every Supabase plan below Enterprise (C21 still records the exact console values); the postal address
  is offered on request, as the old page did; the terms' legacy-subscriber paragraph is dropped (there are
  no subscribers, so there is nothing to grandfather); the terms say "a desktop app".

## 2. Decisions, each with its reason

- **D1. The setting belongs to the account, and the server enforces it (K1).** It is stored on the account
  row, read by `sync-push`/`sync-pull`, and cached in app data (not the vault, so no vault format changes;
  M8). While it is on, `sync-push` refuses every upload from every computer, so a second computer cannot
  re-create the copy. The engine's `sync` reads the cached setting and exits 0 with a named line
  (`sync: this computer only, nothing sent`); the slot lists it as `sync (skipped: this computer only)`.
- **D2. Turning it on deletes the account's copy**: vault text, journal and grades, by the account-deletion
  purge scoped to those tables. It leaves the database's backups within [BACKUP WINDOW], and the page says so.
  Turning it off uploads the vault of the computer that turned it off; other computers then sync as today.
- **D3. The page never implies the setting makes Knowlu offline.** What still reaches our servers is listed
  in full (3.2).
- **D4. Kept sign-ins keep their risk sentence wherever they are recommended**, the statement included.
- **D5. "Processed" means read and judged, or given up on.** The found items keep the rewrite's retention
  (7 days after the computer takes them, 30 if never): they are results in transit, not mail.
- **D6. Q-P4's mechanics and costs (I3).** A decision record is a `judgments` row with origin `gmail_api` or
  `forward`. Disconnecting Gmail deletes the account's `gmail_api` rows. Forwarded rows gain an `address_id`
  from now on; disconnecting a mailbox deletes its rows, and older forwarded rows that cannot be tied to a
  mailbox are deleted when any mailbox is disconnected. **Learned rules stay** until the account is deleted (they
  are Knowlu's rules for this student, not mail), and the page says so. Cost accepted: after a reconnect Knowlu
  may propose an old email again, and the rules lose their history from that mailbox.
- **D7. Who can access names roles, not people**, so it stays true as the team changes.
- **D8. No new reason for a person to read (K2).** The page's two existing promises (no human reads your tasks
  or your mail without explicit consent) stay, each gaining one exception, a valid legal demand, and 3.5 says
  the same. There is no "to stop an attack" exception. This is a material change and a `PRIVACY_CHANGES` line.

## 3. Text changes to the privacy page (on top of the rewrite's §3, launch-bump variants)

3.0 **Notice at the top** (Q-P8), directly under the date lines and above the one-line promise, as its own
`<p class="notice">`:
> **This policy will change often until Knowlu launches in January 2027, with its pilot.** Each version carries its date above. A change that matters, such as something new collected, a new company or a new use, is still shown to you in Knowlu and asked for before it applies, as *Changes to this policy* says.

3.1 **The one-line promise** (pinned: `site.rs:16`, and `wizard.js`'s `PRIVACY`, compared with a bare one-line
`<p>` by `ui_wizard.rs:270-275`, so that `<p>` keeps no class):
> Your tasks and notes live in a plain-text folder on this machine and, unless you choose This computer only, in your Knowlu account, so every computer you sign in on opens on the same day; that copy is encrypted at rest, beside your account, the judgments made for you and what you correct, and none of it is ever sold or shared.

3.2 **This computer only.**
- *What stays on your machine*, first paragraph: "Your vault also has a copy in your account" becomes "Unless you chose This computer only, your vault also has a copy in your account". Then a new paragraph after it:
> **This computer only.** You can choose it at setup or in Settings → Data and privacy. Then your vault is not stored in your account at all: our servers refuse to store it from any of your computers, and choosing it deletes the copy already there (it leaves our database's backups within [BACKUP WINDOW]). Knowlu works the same on each computer, but your computers no longer share a plan, and a new one starts empty. Knowlu still sends our servers what it needs to work, all of it listed under *What we collect*: the excerpt each judgment needs, for one call; the judgment that comes back, with the first three words of a title, organizer or sender; the rules Knowlu learns for you; your calendar links; your email, if you connect it, as its own sections describe; your corrections, what you click, your daily counts and your setup answers; and any issue report you choose to send.
- *Your tasks and notes* `<dd>` begins: "Unless you chose This computer only, a copy of your vault: …" (the `<dt>` stays as pinned).
- The forwarded-mail table's "which your account holds" becomes "which your account holds unless you chose This computer only".
- *Export*: after "the plain files on your computer are the same text your account holds", add "(with This computer only, they are the only copy)".
- The sentence pinned at `site.rs:405` stays; add after it: "With This computer only, they stay in the vault on this computer."

3.3 **Kept sign-ins** (the whole bullet, replacing today's; M1, K3):
> **Kept sign-ins.** Some sites sign you out often. So that Knowlu can keep reading them without asking you to sign in again each time, it can keep your sign-in to a site on this computer, in a browser profile encrypted for your account on this computer: your Blackboard sign-in, for grades, and, if you set up email forwarding, a saved sign-in for each mailbox, so Knowlu can check that forwarding is still on, turn it back on and, for Gmail, bring in mail it missed. Knowlu asks at setup, with each kept sign-in already on and marked Recommended. You can turn any of them off at setup or later (**Forget Blackboard sign-in** in Settings, or **Settings → Email** for a mailbox); each is deleted when you turn it off, disconnect it, sign out of Knowlu or delete your account. None keeps a password, and none ever leaves this computer.

followed by today's cache sentence ("A kept sign-in is full access to that account on this computer: …") unchanged
(pinned; the Mac lane rewrites its platform words). Every pin in this bullet that the new text changes
(`site.rs:239-245` "two kept sign-ins" family) is named in the PR; "Settings → Email" stays (`site.rs:272`).

3.4 **Forwarded email.**
- Replace "It waits in Amazon's storage until Knowlu has read it and is deleted then; anything left over is removed by Amazon's one-day expiry, which in practice means up to about two days, plus Amazon's own lag." with:
> It waits in Amazon's storage only until Knowlu has read it, and is deleted the moment Knowlu is done with it, whether or not it was about school, and also when Knowlu gives up on a message it could not read. A message Knowlu never reached is removed by Amazon's one-day expiry, which in practice means up to about two days, plus Amazon's own lag.
- Table, *The whole message*, How long: "deleted as soon as Knowlu is done with it; a message Knowlu never reached, up to about two days in Amazon's storage, plus Amazon's lag". *The record of each decision*: "until you disconnect that mailbox or delete your account".
- *Who else touches it*, Amazon: "holds it until Knowlu is done with it, and a message Knowlu never reached at most about two days."
- *How long we keep it*, forwarded email: the message clause as above, and "Disconnecting a mailbox deletes everything our servers hold about it, the decision records included. The rules Knowlu learned from your mail stay until you delete your account."

3.5 **Who can access your data** (new section, after *Who else touches it*; I6, K2, M4, M5):
> **Who can access your data**
>
> **You.** Your plan is in the plain files on your computer, and *Your rights* below says how to get a copy of what our servers hold.
>
> **People at Knowlu.** The people who run Knowlu can reach our database through its administration tools, because someone has to keep it running. No one at Knowlu reads your tasks, notes, grades or anything found in your email without your explicit consent, unless a valid legal demand requires it. Administrative accounts use two-step sign-in and are given only to the people who need them.
>
> **The companies that run parts of Knowlu**, listed above, each for its one job and nothing else.
>
> **Your school** does not see what Knowlu holds about you.
>
> **Courts and the police**, only with a valid legal demand that names your account. We tell you before we hand anything over, unless the law forbids it, and hand over no more than the demand requires. With This computer only, your tasks and notes are not on our servers to hand over.
>
> **If Knowlu is sold**, the buyer takes on this policy, and we tell you first.
>
> **Nobody else.** No advertiser, data broker or analytics company.

3.6 **The two consent promises gain the legal exception (D8).**
- *Your tasks and notes*: "**No human at Knowlu reads it without your explicit consent**" → "**No human at Knowlu reads it without your explicit consent**, unless a valid legal demand requires it (see *Who can access your data*)".
- *Gmail*: "No human at Knowlu reads your mail without your explicit consent." gains the same clause. The issue-report sentence that follows is unchanged.

3.7 **Gmail disconnect** (`site.rs:428-435` pins both old sentences; that test is replaced):
- The disconnect sentence: "Disconnecting revokes our access at Google and deletes the stored connection, our list of which emails we have already read, any proposals from your mail that had not reached your computer yet, and the record of what Knowlu decided about each email."
- *How long we keep it*, Gmail: "The record of what we decided about each email, including its message id and who it was from, is kept until you disconnect Gmail or delete your account. The rules Knowlu learned from your mail stay until you delete your account."
- *Gmail*, the kept-items sentence after "Message text is discarded": add "until you disconnect Gmail".
- The rewrite's judgments-and-rules entry (its lines 458, 521) gains: "except the judgments about your email, which go when you disconnect that mailbox or Gmail".

3.8 **The pilot application form (I7).** *Mail you send us* (the rewrite's waitlist paragraph) gains:
> If you apply to the pilot on our website, your answers go to a Google Form and are kept in Knowlu's Google account, used only to choose pilot students and to contact you about the pilot, and deleted when you ask. [Quinn: a fixed deletion date, such as when the pilot ends?]

3.9 **Not changed:** the 20-paying-users paragraph, the cache sentence, every [TRUE NOW] item of bump A.

## 4. The Pilot Privacy Statement (`site/pilot-privacy.html`; I5)

Says nothing the policy does not; no model names; no word "AI"; no platform words. Text:

> **Pilot privacy, in plain words**
>
> Knowlu reads your school accounts so it can tell you what's next. Here is what that means for your data. The full [privacy policy](privacy.html) is what counts; this page is the short version.
>
> **Knowlu never sees your school password.** You sign in to your school on its own page. Logins for coursework sites like zyBooks are kept in your computer's own password storage, and our servers have nowhere to store a password.
>
> **You choose where your plan lives.** By default your tasks are in a plain-text folder on your computer and in your Knowlu account, encrypted, so any computer you sign in on shows the same day. Our servers can read that copy, because keeping your computers in step is their job. If you'd rather not keep one, choose **This computer only**: your plan is kept on your computer and not in your account. Knowlu still sends what it needs to work, and the policy lists all of it.
>
> **What the models see.** To estimate how long things take and what matters most, Knowlu sends a short excerpt of each task and calendar event, and of your email if you connect it, to open-weights models run by CoreWeave and DeepInfra. They read it for one request, keep nothing and train on nothing.
>
> **Email is your choice.** It is off until you connect a mailbox. Knowlu reads the start of each message, personal mail included, to tell whether it's about school, and keeps only what is. Forwarded mail is deleted as soon as Knowlu is done with it. Disconnect at any time, and the records of what Knowlu decided about your email go with it.
>
> **Staying signed in is recommended, and up to you.** Knowlu can keep you signed in to Blackboard and your mailbox on this computer, so it doesn't have to ask again. A kept sign-in is full access to that account on this computer, so keep your computer locked. It's on by default, and you can turn it off at setup or in Settings.
>
> **What we measure.** Which screens you open, what you check off, snooze or approve, and when you correct an estimate. Never your words, task titles or course names. Error logs reach us only when you send a report, and you see it first.
>
> **Who can see it.** You. No one at Knowlu reads your tasks or email without your permission, unless the law requires it. Your school doesn't see what Knowlu holds about you. The companies that run parts of Knowlu each do one job; the policy names every one.
>
> **Applying to the pilot.** Your answers on the pilot form are kept in Knowlu's Google account, used only for the pilot, and deleted when you ask.
>
> **Leaving is one button.** Settings → Delete my data removes your plan, your sign-ins and your account. We never sell or share your data.
>
> Questions: support@knowlu.com. A person reads it.

**Publication (I7).** The statement is a legal page: C25's release-tag guard extends to it. Quinn decides
whether it publishes now (describing launch behavior while the pilot runs on today's app) or with the launch
bump; recommendation in the PR.

## 5. Style (Q-P7; M7)

- The three pages use the site's system: `css/tokens.css`, `css/base.css`, its fonts, `index.html`'s `.nav`
  and `.ft` markup with links back to `index.html#…`, and one new `css/legal.css` for long-form reading.
  Red only for links and focus.
- **No script on any legal page.** The header's theme toggle and phone-width menu are dropped there; the
  theme follows the system setting through `tokens.css`.
- `site.css` stays for `signed-in.html` and `subscribed.html`.
- The one-line promise stays a bare one-line `<p>`; every other pinned sentence keeps its text.

## 6. Code that makes it true by launch (new rows for the rewrite's §6)

| # | Change | Size | Contract list? | Agent |
|---|---|---|---|---|
| C30 | **This computer only** (D1, D2): the account column; `sync-push` refuses while on; the purge; the engine's named exit-0 line and the slot's skipped step; setup and Settings rows. Waits for §1's signatures | L | **Yes** (`engine/src/sync.rs`), migration, functions | planner (plan), contract-engineer, cloud-engineer, console-ui |
| C31 | Kept sign-ins as a preselected **Recommended** choice at setup, with the risk line | S–M | No | console-ui, implementer |
| C32 | `mail-process` deletes the stored message when it is done with it, success or final failure | S | No | cloud-engineer |
| C33 | Decision records on disconnect (D6): `address_id` on forwarded judgments going forward; Gmail and mailbox disconnect purges | M | No, but a migration: **Contract** | cloud-engineer, contract-reviewer |
| C34 | Two-step sign-in verified on every administrative account: Supabase, AWS, Cloudflare, Stripe, Google Cloud, OpenRouter, Resend, GitHub (it holds production secrets), the support mailbox host, the domain registrar and DNS, and any tool connected to the production database; recorded with the C21 facts | S (consoles, docs) | No | main session |
| C35 | `pilot-privacy.html` linked from the pilot band, the footer and the privacy page's top; C25 extended to it | S | No | console-ui |

## 7. What must not break

The rewrite's §2; every pin in `engine/tests/site.rs` and the app tests, changed only where this note changes
the sentence, each change named in the PR; the wizard sentence equals the page's; the version tie; one
`PRIVACY_CHANGES` key per build; no word "AI" on the legal pages or in the `PRIVACY_CHANGES` lines; legal pages
publish only on a release tag; `sync` always exits 0.

## 8. Tests

1. Pins for 3.0 (the notice comes before the one-line promise) and 3.1–3.8's new sentences, and that the replaced ones are gone. Named pin changes: `site.rs:16`
   (3.1), the "two kept sign-ins" family at `site.rs:239-245` (3.3), `site.rs:428-435`'s test replaced (3.7),
   `site.rs:361` "AI calls" → "model calls" (the rewrite's C18).
2. `pilot-privacy.html`: exists, no `<script`, no standalone "AI", links to `privacy.html`; carries none of
   "Credential Manager", "Granite", "Qwen", "Windows".
3. The legal pages link `css/tokens.css`, `css/base.css`, `css/legal.css`, and not `site.css`; none carries a
   script; the banned-word list (`site.rs:29`) runs on all three.
4. C35: the homepage footer, the pilot band and the privacy page link `pilot-privacy.html`.
5. The launch bump's `PRIVACY_CHANGES` lines name This computer only, the legal-demand exception, the decision
   records on disconnect and the pilot form.
6. C30–C33: their own plans' tests; C30's include a second profile's push refused while the setting is on.
7. **Live check** before the PR is called ready: the three pages in headless Edge, light and dark, desktop and
   phone widths, by the main session; screenshots in the PR.
8. The lawyer list (the rewrite's §8) gains 3.5, 3.6 and the statement.
