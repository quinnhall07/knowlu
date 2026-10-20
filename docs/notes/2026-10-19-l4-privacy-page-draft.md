# L4 groundwork: the privacy page rewrite, drafted (2026-10-19)

**Status:** draft for Quinn's checkpoint. Docs only: nothing under `site/`, `app/` or `cloud/` is
edited by this note, and `PRIVACY_VERSION` does not move here. The page text in §3 is a draft of
what lane L4 puts on `site/privacy.html`; the plan in §4 is the lane's outline, not its full plan.

**Authority, newest first.** The cloud design's Amendment 2026-10-19 (privacy bump #1 is L4's
interim bump, carrying the sentences the Gmail connect §6, email forwarding §7 (D18), M2 §5 and
grades §11 specs assign to it, with no lawyer read; the lawyer read stays before the Pilot). The
session-5 ledger's rulings of 2026-10-18/19 (`.superpowers/sdd/2026-10-18-session-5/progress.md`):
C3 Q3 (R12b adds D18's upload row and the mailbox-actions sentence after L4 merges, with
`PRIVACY_VERSION` unmoved), #19 close-out §8 Q5 (the registrar sentence rides L4's page, "at schools
Knowlu supports"), L5 §17 Q39/Q42 (syllabus lines on L4's page; no student-facing syllabus task
merges before it). Then the L5 spec §6.1, the four specs' privacy sections, `VISION.md`, and the
current page (version 2026-09-24).

**How to read §3.** Each draft sentence carries a claim tag `⟨cN⟩`. Appendix A (never published)
maps every tag to the file and line that makes it true, or says plainly that no code line does.
Tags on text carried unchanged from the 2026-09-24 page are marked `=` and were not re-verified
here, except where Appendix A says so. Blocks are labelled:

- **[now]** true of `main` plus the email-forwarding branch, which L4 waits for (§4.1).
- **[#19]** worded to be true before and after PR #19 (the registrar) merges.
- **[L5]** syllabus statements, §6.1 items 2-5, which Q42 puts on the page before L5's code exists
  (open question Q2).
- **[after C3]** D18's two additions. Not in L4's edit: R12b adds them after L4 merges (C3 Q3).

## 0. What changes from the 2026-09-24 page, in one list

Sentences on today's page that are false or incomplete once the email branch merges:

1. `privacy.html:77`, "(Gmail is not connected in this version of Knowlu; the app will say when it
   is.)" Gmail connect merged on 2026-09-30 (#26). The parenthesis goes (Gmail spec §6 item 1).
2. `:55`, "to fetch two things — your calendar link and your course list — and then throws the
   session away. It keeps neither the password nor the cookies." False for grades: the Blackboard
   session is kept (`app/src/grades.rs:263-275`). Gains the kept-session exception and the
   registrar's "at schools Knowlu supports, your class schedule".
3. `:20-26`, "These four things live on your PC". The two kept sign-ins (Blackboard, mail) and the
   local-only commitment questions are missing.
4. `:56`, the model bound. It names no event listing or interests (M2 §5), no 2,000-character bound
   for forwarded mail (D18 item 1), no date, not the course codes an email prompt carries or
   forwarded mail's time zone, and not the resolve call's coursework titles.
5. `:48` and `:85`, "What survives is the message id, what we decided it was, and the fields…". The
   judgment record also keeps the email's sender (Appendix A, c40). See Q6.
6. `:60`, "Six companies". Amazon Web Services joins (D18 item 5): seven.
7. `:70`, the email-delivery provider "sees the address a message is going to". It now also sends
   the delivery checks to a connected mailbox, so it sees that mailbox's address too.
8. "What we collect" has no entry for the Google connection itself (Gmail §6 item 2), forwarded
   mail, the forwarding addresses (D18 item 3) or grades (grades §11).
9. "How long we keep it" has no Gmail bullet (Gmail §6 item 4) and no forwarded-mail bullets (D18
   item 4).
10. "Your rights", Delete: the forwarding-off pre-step and the mail sign-ins are not named.
11. No section for email forwarding (D18 item 2), and no "kept sign-ins at every school, for now"
    paragraph (D25, A13; HANDOFF's Pilot row).

What stays word for word: the one-line summary sentence (pinned by `engine/tests/site.rs:15` and
matched against `console.js`'s `PRIVACY` by `app/tests/static_assets.rs:1006-1019`; see Q1), the
sections on who we are, selling, age, security, Do Not Track, and every sentence the three site
tests pin (`engine/tests/site.rs:43-94`, `app/tests/static_assets.rs:1666-1690`).

## 1. VISION and CLAUDE.md check

- **Commitment 2** (`VISION.md:30-33`): "the privacy page says so". The account copy, grades
  included, is named with "encrypted at rest, not end-to-end" kept (§3.4).
- **Grades** (`VISION.md:136`): "named on the privacy page, and never enter telemetry". §3.4 names
  them; the telemetry entry is unchanged and grades are never added to it.
- **"Knowlu reads"** (`VISION.md:169`, A8): the mailbox actions in §3.8 are the A8 list, and no
  more: forwarding on at setup, older mail and missed mail forwarded with Sent copies removed
  (Gmail; "missed" is the daily pass over the last three days, reaching back further after forwarding
  was off or the subscription lapsed, c42a), forwarding turned back on and announced, never for a mailbox turned
  off in Settings, nothing sent to anyone else. Microsoft 365's backfill and its recover pass are
  both held until C3, and the page says so rather than describing a routine that does not run.
- **Telemetry** (`VISION.md:170-172`): nothing from forwarded mail or the Gmail API, never grades.
  Stated in §3.8.
- **Data minimization** (`VISION.md:184`): the screen before any model is described as what it is,
  a reduction ("set aside"), not a scrub.
- **Rule 1, no single-user assumptions.** The page names no person, school, tenant or vault. "At
  schools Knowlu supports" and "at every school Knowlu has a Blackboard listing for" are the only
  school references, and neither names one.
- **Rule 2, frozen references.** Nothing here touches `engine/tests/fixtures/**`.

## 2. Decisions this draft takes, and why (each open to Quinn in §5)

- **D-a. Microsoft 365 today is described as it runs.** Forwarding is set up and kept on; older
  mail is not brought in, and neither is mail that arrived while forwarding was off: the slot
  returns before reconcile and recover for a held provider (`backfill_held_for_c3`, Appendix A
  c33). The page says "Knowlu does not yet bring in older mail, or mail that arrived while
  forwarding was off". C3's upload row waits for R12b.
- **D-b. Sent copies go to Gmail's Trash.** `sent_cleanup.js` presses Gmail's Delete, which moves a
  copy to Trash (c31). The draft says "moves to your Trash", not "deletes" (Q7).
- **D-c. The backfill window is the code's**, 120 days before setup (c29), said as "up to about four
  months before you set it up", not "the term's start" (Q9).
- **D-d. The syllabus lines carry a "not in this version" parenthesis** while L5's surfaces are off
  `main` (Q2), on the 2026-09-24 page's own Gmail precedent. Removing it is L5's go-live, and is
  material under the recommendation in Q2: it moves `PRIVACY_VERSION` and re-prompts, as Gmail's
  removal does here.
- **D-e. The one-line summary sentence is kept word for word** (Q1).

## 3. The draft page text

Section order follows the 2026-09-24 page; one section is new (§3.8). `V` is the new version date,
set in L4's last commit (§4.3). Text in *italics with brackets* is an instruction to the page's
editor, not page text.

### 3.0 Header

Effective `V`. This page is version **`V`**, the version Knowlu writes down beside your name on the
day you accept it. ⟨c1⟩ *[If Q3 is ruled as recommended, add a second line: "Last changed `V`."]*

*[The summary sentence and "That is the whole policy in one line…" stay word for word. ⟨=⟩]*

### 3.1 Who we are, and how to reach us

*[Unchanged. ⟨=⟩]*

### 3.2 What stays on your machine  [now]

Knowlu is a desktop app first. These things live on your PC and are yours. Your vault also has a
copy in your account, described below, so that every computer you sign in on shows the same day;
everything else in this list never leaves this machine. ⟨c2⟩

- **Your vault** — *[the 2026-09-24 bullet, unchanged ⟨=⟩]*, then: Two things in it stay on this
  computer and are never copied to your account: the questions Knowlu asks you about which calendar
  events are your classes and other commitments, before you answer them, and the list of repeating
  events it builds those questions from. Only what you confirm or decline is copied. ⟨c3⟩
- **App data** — *[unchanged ⟨=⟩]*.
- **Coursework logins** — *[unchanged ⟨=⟩]*.
- **Kept sign-ins, if you use them.** If you connect Blackboard for grades, Knowlu keeps your
  Blackboard sign-in on this computer, in a browser profile Windows encrypts for your Windows
  account, so it can read your grades without asking you to sign in every time. ⟨c4⟩ **Forget
  Blackboard sign-in** in Settings deletes it. ⟨c5⟩ If you set up email forwarding, Knowlu keeps a
  saved sign-in for each mailbox the same way, so it can check that forwarding is still on, turn it
  back on and, for Gmail, bring in mail it missed. ⟨c6⟩ It is on unless you turn it off in **Settings → Email**,
  and it is deleted when you turn it off, disconnect that mailbox, sign out of Knowlu or delete your
  account. ⟨c7⟩ Neither keeps a password, and neither ever leaves this computer. ⟨c8⟩ A kept sign-in
  is full access to that account on this computer: Windows encrypts its sign-in cookies for your
  Windows account, but the pages and mail the site keeps in its own cache sit in the same folder
  without that encryption, and a program running as you could use either. ⟨c9⟩
- **Your backup folder** — *[unchanged ⟨=⟩]*.
- **[L5] Your syllabi.** Every syllabus file you give Knowlu, and everything read from it that is
  not in a suggestion or approved: office hours, the course's policies in the syllabus's own
  words, and the dates not yet suggested. (Knowlu does not read syllabi in this version; the app
  will say when it does.) ⟨c10⟩

**Kept sign-ins at every school, for now.** Until Knowlu has 20 paying users, both kept sign-ins
are on without waiting for any school's own policy on them to be read first: the saved mail
sign-in at every school, and the Blackboard sign-in at every school Knowlu has a Blackboard listing
for. A school Knowlu has no listing for still has no grades. ⟨c11⟩ When the 20th paying account
exists, we decide this again, and a change would be a new version of this page. ⟨c12⟩

### 3.3 What we collect, and why  [now]

By category, with the reason it exists. If a category is not on this list, we are not collecting
it. ⟨=⟩

- **Your account** — *[unchanged ⟨=⟩]*.
- **Your subscription** — *[unchanged ⟨=⟩]*.
- **Your tasks and notes** — *[the 2026-09-24 entry, unchanged ⟨=⟩, with one sentence added after
  its first:]* That includes the classes and other commitments you confirm and, if you connect
  Blackboard, your grades, kept with your tasks so that every computer you sign in on shows the
  same ones. ⟨c13⟩
- **Your calendar links** — *[unchanged ⟨=⟩]*.
- **Your coursework logins** — *[unchanged ⟨=⟩]*.
- **Your Google connection, if you make one** *(new)*. The permission Google gave us, stored
  encrypted and never sent to your computer; the Google account's id and email address, so the app
  can show which account is connected; which of Calendar and Gmail you allowed; when you connected
  and when Knowlu last read your mail; and whether the connection still works. It is deleted when you disconnect or delete your
  account. ⟨c14⟩
- **Email you forward to Knowlu, if you set it up** *(new)*. Every message that reaches a connected
  mailbox, whole, held only until Knowlu has read it; then what Knowlu found in it about school.
  Its own section is below. ⟨c15⟩
- **Your forwarding addresses** *(new)*. The private Knowlu address each connected mailbox forwards
  to, stored encrypted, and that mailbox's own address, so mail can be matched to it and the
  delivery checks below can reach it; and the facts that show forwarding works: when mail and
  delivery checks last arrived, and, for Microsoft 365, your school's tenant id. All of it goes
  when you disconnect that mailbox or delete your account. ⟨c16⟩
- **What you do in Knowlu** — *[unchanged ⟨=⟩]*.
- **Your corrections** — *[unchanged ⟨=⟩]*.
- **Diagnostic logs** — *[unchanged ⟨=⟩]*.
- **Gmail, if you connect it** — Message metadata and text, **held in memory for the length of one
  judgment and then discarded**. ⟨=⟩ What survives is the message id, who it was from, what we
  decided it was, and the fields that ended up on the card you saw. ⟨c17⟩ Its own section is
  below. ⟨=⟩

### 3.4 What we do not collect

- **No birthdate.** *[Unchanged ⟨=⟩]*
- **No card number.** *[Unchanged ⟨=⟩]*
- **[#19] No campus password.** Knowlu never asks for your university sign-in, and there is nowhere
  in the app to type one. When you sign in to your school, it happens on your school's own page, in
  a window Knowlu opens, and Knowlu never sees what you type there. ⟨=⟩ Once you are signed in it
  uses that window's own session, on your machine, to fetch your calendar link, your course list
  and, at schools Knowlu supports, your class schedule, and then throws the session away. It keeps
  neither the password nor the cookies. ⟨c18⟩ The one exception is the Blackboard sign-in for
  grades, which Knowlu keeps on this computer as described above, and still never the
  password. ⟨c19⟩
- **No more of your text sent to a model than a judgment needs.** What travels to the model is the
  note's title and up to the first 1,200 characters of its text, your stated preferences and grade
  weights; ⟨c20⟩ when a campus event is judged, the event's listing and up to the first 600
  characters of the interests you have told Knowlu; ⟨c21⟩ for an email read through Gmail's
  permission, its subject, its sender, its date, up to the first 1,200 characters of the message
  and the codes of your courses; ⟨c22⟩ for email you forward to Knowlu, the same with up to the
  first 2,000 characters, and your time zone; ⟨c23⟩ and, to match what an email says to your coursework, the titles and due dates of the
  course items it might mean. ⟨c24⟩ All of it is sent for the length of that one call, and neither
  we nor the model's host keep a record of what was sent. ⟨=⟩
  *[L5, appended to this bullet:]* For a syllabus, one call sees at most 13,500 characters: the
  course's code, section and term; the lines that say when and where the class meets and when
  office hours are, with email addresses, phone numbers and links removed; the lines of the
  schedule that carry a date; the lines that say how the course is graded; and the paragraphs that
  state the late-work, makeup and attendance policies. The instructor's contact details are never
  sent, and the model's host is one already named below. ⟨c25⟩

*[L5, a new bullet after it:]* **What Knowlu never does with a syllabus:** share it, pool it across
students, or keep a copy on our servers. ⟨c26⟩

### 3.5 Who else touches it  [now]

Seven companies, each doing one job, and only two of them ever have any content of yours read by a
model. ⟨c27⟩

- **Supabase**, **Stripe**, **Cloudflare**, **OpenRouter**, **CoreWeave** — *[unchanged ⟨=⟩]*.
- **DeepInfra** — reads your email to make those judgments, *[rest unchanged ⟨=⟩]*. *[It reads
  Gmail and forwarded mail alike ⟨c28⟩; no wording change is needed.]*
- **Amazon Web Services** *(new)* — receives the email you forward to Knowlu and holds it until
  Knowlu has read it, at most about two days. ⟨c29a⟩

*[The CoreWeave-and-DeepInfra paragraph is unchanged ⟨=⟩. The closing paragraph becomes:]* One more,
doing nothing with your data but delivering it: the provider that sends our account and billing
email, and the delivery checks Knowlu sends to a mailbox you forward from, sees the address a
message is going to. ⟨c30⟩ *[Its last two sentences are unchanged ⟨=⟩.]*

### 3.6 We do not sell or share your personal information

*[Unchanged ⟨=⟩.]*

### 3.7 Gmail  [now]

*[First paragraph unchanged ⟨=⟩.]*

Connecting Gmail is optional, it is off until you turn it on, and you can disconnect it in Settings
at any time. ⟨=⟩ Disconnecting revokes our access at Google and deletes the stored connection, our
list of which emails we have already read, and any proposals from your mail that had not reached
your computer yet. The record of what Knowlu decided about each email, including its message id and
who it was from, stays until you delete your account. Google holds Calendar and Gmail as one
permission, so disconnecting one disconnects both. ⟨c31a⟩ If you also forward that Gmail address to
Knowlu, Knowlu reads it through forwarding only. ⟨c32a⟩

*[The Limited Use paragraph and its bullets are unchanged ⟨=⟩, except the last bullet, which
becomes:]* Message text is **discarded when the judgment is done**. What is kept is the message id,
so the same email is not proposed to you twice, who it was from, the decision, and the fields on
the card. ⟨c17⟩

*[The testing-mode paragraph is unchanged ⟨=⟩.]*

### 3.8 Email you forward to Knowlu  [now; new section, after Gmail]

Email forwarding is optional, and you set it up in Knowlu. Each mailbox you connect forwards
everything that reaches it to a private Knowlu address: school mail, personal mail, all of it.
⟨c15⟩ It passes through Amazon's mail service; Amazon's notice to us carries the message's headers
(sender, recipients, subject), never its text. It waits in Amazon's storage until Knowlu has read
it and is deleted then; anything left over is removed by Amazon's one-day expiry, which in practice
means up to about two days, plus Amazon's own lag. ⟨c29⟩ It is read in our server's memory. ⟨c34⟩
While your subscription is not active or in its trial (it has lapsed, been cancelled, or has a
payment overdue), mail that arrives is deleted unread. ⟨c34a⟩

Before any model reads a message, Knowlu's own rules set aside mail such as bulk and mailing-list
mail from senders outside your school and its learning tools, and mail the rules Knowlu has learned
for you mark as noise, ⟨c35⟩ and mail Knowlu recognises by its shape is read by Knowlu's own rules
without a model. ⟨c35a⟩ Every other message, including personal mail from a friend, a doctor or a
bank, has its subject, sender, date and up to the first 2,000 characters of its text, with the
codes of your courses and your time zone, read by the model's host, which keeps none of it,
so that Knowlu can tell whether it is about school. ⟨c23⟩ What Knowlu keeps is what it found about
school, not the email. ⟨c36⟩ Attachments are never opened, except the messages Knowlu itself
forwards as attachments when it brings in older or missed Gmail mail, each of which is read as an
email. ⟨c37⟩

| What | Where | How long |
|---|---|---|
| The whole message | Amazon (its mail service and storage), then our server's memory | deleted when read; otherwise up to about two days in Amazon's storage, plus Amazon's lag ⟨c29⟩ |
| The message's headers (sender, recipients, subject) | Amazon's notice to our server | for the length of that call, never logged ⟨c38⟩ |
| The subject, sender, date and up to 2,000 characters of each message the rules neither set aside nor read themselves, with your course codes and time zone | the model's host, one call | not kept ⟨c23⟩ |
| The titles and due dates of the course items an email might mean | the model's host, one call | not kept ⟨c24⟩ |
| Your mailbox's own address, when mail and delivery checks last arrived, and (Microsoft 365) your school's tenant id | our database | until you disconnect it or delete your account ⟨c16⟩ |
| A keyed fingerprint of each message, so it is never read twice | our database | 180 days ⟨c39⟩ |
| What was found: a one-line reason, a quote of at most 140 characters, the sender's name and address, and the date | our database until your computer takes it; then your vault, which your account holds | 7 days after your computer takes it (30 if it never does); in your vault until you delete it ⟨c36⟩ |
| The record of each decision (no message text) | our database | until you delete your account ⟨c40⟩ |
| The forwarding address | our database, encrypted | until you disconnect it or delete your account ⟨c16⟩ |
| The saved mail sign-in, one per mailbox | your computer only, in Knowlu's app folder | until you turn it off, disconnect that mailbox, sign out of Knowlu, or delete your account ⟨c7⟩ |

*[After C3 merges, R12b inserts this row after "The whole message", in D18's signed words:]* "| An
older message read on your computer (Microsoft 365) | uploaded from your computer to our
function's memory | for the length of that call |" *[R12b may put "our function's memory" in the
page's plainer "our server's memory" only if Quinn says so: D18's words are signed (Q10).]*

**What Knowlu does inside your mailbox,** with you there at setup or, later, with the saved sign-in
you allowed:

- It turns forwarding on, and off again when you disconnect. ⟨c41⟩
- **Gmail:** it forwards older mail, from up to about four months before you set it up, to your
  Knowlu address as attachments, ⟨c42⟩ and moves the copies those forwards leave in your Sent
  folder to your Trash, where Gmail removes them as it does anything else there. ⟨c31⟩ Then, about
  once a day, and at every scheduled run once forwarding has stopped or your subscription has
  lapsed, it forwards the same way any mail from the last three days that did not reach Knowlu.
  After forwarding was off, or after your subscription lapsed, it looks further back: to when forwarding was last on, at most
  about four months, or up to 30 days back after a lapse. ⟨c42a⟩ While it does, it looks in your Inbox for a notice that one of those forwards was not delivered,
  and stops if it finds one. ⟨c43⟩
- **School Microsoft 365:** Knowlu does not yet bring in older mail, or mail that arrived while
  forwarding was off; only mail forwarded after setup reaches Knowlu. ⟨c33⟩ *[Removed by R12b after C3 merges, which then adds D18's sentence:]*
  *"for Microsoft 365, reads older or missed mail through Outlook's own services and uploads it from
  this computer to Knowlu; it sends and deletes nothing".*
- If it finds forwarding turned off, it turns it back on and then tells you so. It never turns
  forwarding back on for a mailbox you turned off in Knowlu's Settings, and it never changes
  forwarding that points somewhere else without asking you. ⟨c44⟩

Nothing else is changed, sent or deleted. ⟨c45⟩ Knowlu also sends each mailbox a delivery check now
and then, to make sure forwarding still works. ⟨c46⟩ None of this sends anything to another person.
⟨c45⟩

**Telemetry and training.** Nothing from forwarded mail ever enters telemetry. ⟨c47⟩ Judgments made
from forwarded mail are never used to train or improve a model and never pooled with another
person's. ⟨c48⟩ Google's Limited Use policy does not bind mail that arrives by forwarding, because
it does not come through Google's API; Knowlu holds it to the same rules anyway. ⟨c48⟩

### 3.9 How long we keep it  [now]

*[The six 2026-09-24 bullets are unchanged ⟨=⟩, except "Your tasks and notes", which gains "and your
grades" after "the current text of each" ⟨c13⟩. Two bullets are added after it:]*

- **Gmail** — our list of the emails we have read, for 30 days, so the same email is not read
  twice; a proposal waiting to reach your computer, until it does, and then for 7 days.
  Disconnecting Gmail, or deleting your account, deletes both at once. The record of what we decided
  about each email, including its message id and who it was from, is kept until you delete your
  account. ⟨c49⟩
- **Email you forward to Knowlu** — the message itself, until Knowlu has read it, and at most about
  two days in Amazon's storage, plus Amazon's lag; ⟨c29⟩ the fingerprints that stop a message being
  read twice, 180 days; ⟨c39⟩ your mailbox's own address, its forwarding address and the facts
  that show forwarding works, until you disconnect it; ⟨c16⟩ what was found, 7 days after your computer takes it, or 30 if it never does,
  and then only in your vault. ⟨c36⟩ Disconnecting a mailbox deletes everything our servers hold
  about it except the decision records. ⟨c50⟩

### 3.10 Your rights, wherever you live  [now]

- **Know and access** — *[unchanged ⟨=⟩]*.
- **Export** — *[the 2026-09-24 bullet, with its list extended:]* …the account, the subscription,
  the consent records, which calendar links exist, your connected mailboxes and what was found in
  your mail that has not yet reached your computer, your events, your corrections and your issue
  reports. ⟨c51⟩ *[Rest unchanged ⟨=⟩.]*
- **Correct** — *[unchanged ⟨=⟩]*.
- **Delete** — **Settings → Delete my data**. *[Then, new:]* If you forward email to Knowlu, it
  first tries to turn forwarding off in each mailbox with its saved sign-in, and tells you if it
  could not, so you can turn it off yourself; that never stops the deletion. ⟨c52⟩ It removes the
  vault, its snapshots in your backup folder, your coursework logins in Credential Manager, both
  kept sign-ins and the rest of the app's data on this machine, and the account and everything
  under it on our servers. ⟨c53⟩ *[Rest unchanged ⟨=⟩.]*

### 3.11 Unchanged sections

*[Age, Security, Do Not Track: unchanged ⟨=⟩. Security's "Portal passwords are the clearest case of
all…" stays true: neither kept sign-in holds a password (c8).]*

**Changes to this policy.** *[Unchanged ⟨=⟩, unless Q3 is ruled as recommended; then its last
sentence becomes:]* Smaller changes, such as a clearer sentence, a corrected address, or a
description that names no new category, company or use, get a new "last changed" date on this page
and nothing more. Reading a kind of material this page says Knowlu does not read, such as syllabi,
is never a smaller change. *[This is Q3 (ii)'s narrow form, the recommendation. If Quinn rules Q3
(i) instead, the last sentence is the broad form: "Turning on a use this page said was not yet on
is never a smaller change."]*

## 4. The L4 lane, outlined

A sketch for the planner who writes L4's plan; it fixes the order, the owners and the tests, not the
code. HANDOFF names `contract-engineer` as L4's next agent; the lane has five owners because it
touches the contract list, the cloud, the page and the console.

### 4.1 Ordering

1. **After the email merge.** §3.2, §3.3, §3.8 and §3.9 describe the email-forwarding branch. A page
   that names forwarding on a `main` without it would describe behaviour the code lacks, and the
   cited lines (Appendix A, tag `EF`) are read off that branch, uncommitted work included. L4's
   first task re-reads every `EF` line on `main` after the merge.
2. **Before #19, C3's R12b, and L5's student-facing tasks.** #19's slot is after L4 and L7 (p3
   close-out plan, "L4 → L7 → #19 → L1 R11 → L5"); R12b runs "after L4 merges" (C3 Q3); Q42 keeps
   every L5 surface off `main` until this page is there. The [#19] sentence is true either way.
3. **Before any release from that `main`.** The Pilot release gate (HANDOFF's Pilot section; Gmail
   Q8 (a1), email D18 and D25) holds every release from a `main` carrying the Gmail row, forwarding or
   the suspended grades gate until bump #1 merges. L4 is bump #1 (Amendment 2026-10-19).
4. **Before production's `GOOGLE_CLIENT_ID`.** Gmail spec §6's ordering: the page reaches
   `knowlu.com` no later than the secret is set (L9's checklist).
5. **The re-consent screen ships in the same release as the version move** (R-PS-4; Pilot gate).
   No non-founder account exists before both.

### 4.2 Tasks

| # | Task | Agent, and why | Files (disjoint) | Before |
|---|---|---|---|---|
| L4.0 | Re-verify Appendix A on `main` after the email merge; fix line numbers, flag any claim no longer true | `docs-keeper`: read-and-record, no code | this note | email merge |
| L4.1 | Failing pins for the new page | `test-writer`: tests first, CI-checked | `engine/tests/site.rs` (new fns only) | L4.0 |
| L4.2 | The page edit, §3 less [after C3] | `console-ui`: owns `site/` | `site/privacy.html` | L4.1 |
| L4.3 | `PRIVACY_VERSION` and `PRIVACY_BUMP_1` to `V` | `contract-engineer` (xhigh), then `contract-reviewer`: `account.rs` is on the contract list and the version is a consent-log contract | `app/src/account.rs` (one line), `app/tests/grades.rs` (one line) | L4.2, same commit |
| L4.4 | Cloud: read and record a later privacy version | `cloud-engineer`: `cloud/` | `cloud/supabase/functions/account/{handler,index,handler_test}.ts` | L4.0 |
| L4.5 | App: the version check and the accept command | `contract-engineer` (xhigh), then `contract-reviewer`: `account.rs` | `app/src/account.rs`, `app/tests/account.rs`, `app/src/main.rs` (one handler line) | L4.4 |
| L4.5b | Scheduler: the slot's cloud steps skip with a named reason until the yes (Q4) | `implementer` (high): fully specified, checked by a scheduler test; not on the contract list | `app/src/scheduler.rs`, `app/tests/scheduler.rs` (new fns only) | L4.5 |
| L4.6 | Failing static pins for the screen | `test-writer` | `app/tests/static_assets.rs` (new fns only) | L4.5 |
| L4.7 | The re-consent screen | `console-ui`: `app/static` | `app/static/index.html`, `app/static/console.js` | L4.6 |
| L4.8 | Docs: HANDOFF's bump-#1 lines, `docs/reference/app.md`, the lawyer-packet delta | `docs-keeper` | those files | L4.3, L4.7 |
| L4.9 | Whole-lane review; Quinn reads the page | `reviewer` (report in `docs/reports/`), then Quinn | none | all |

`app/src/main.rs`: L6 owns it while L6 runs (L5 spec, Q42 bullet). L4.5's one `generate_handler!`
line is handed to L6's owner or waits for L6, as L5's tasks do. L4.5b's two scheduler files are
not disjoint from the email branch (which edits `app/tests/scheduler.rs`, merged before L4.0) or
from L6's scheduler edits: L4.5b starts on a `main` carrying the email merge, and, if L6 is still
running, its edit is handed to L6's owner or waits for L6, as the `main.rs` line does. L4.2 and L4.3 land in **one
commit** (`account.rs:21-23`: the constant and the page's date move together); the controller or
`integrator` assembles it.

### 4.3 Behaviour and tests, per task

**L4.1 (pins, red first).** New tests in `engine/tests/site.rs`, each failing on today's page:
- banned: "Gmail is not connected in this version", "to fetch two things", "Six companies",
  "These four things".
- present: "at schools Knowlu supports, your class schedule"; "Kept sign-ins at every school, for
  now"; "Amazon Web Services"; "Forget Blackboard sign-in"; "Settings → Email"; "2,000 characters";
  "up to the first 600 characters of the interests"; "Google holds Calendar and Gmail as one
  permission"; "who it was from"; "Knowlu does not yet bring in older mail, or mail that arrived
  while forwarding was off; only mail forwarded after setup reaches Knowlu" (R12b deletes this pin
  with the sentence, after C3); "any mail from the last three days that did not reach Knowlu";
  "up to 30 days back after a lapse"; "mail that arrives is deleted unread"; "the codes of your courses"; "your school's tenant id"; "180 days".
- If Q3 is ruled as recommended ((ii)): "Reading a kind of material this page says Knowlu does not
  read, such as syllabi, is never a smaller change". If ruled (i): "Turning on a use this page said
  was not yet on is never a smaller change".
- Today's pins stay as they are (`site.rs:17-94`, `static_assets.rs:1006-1019, 1666-1690`).

**L4.2 (page).** §3, minus every *[after C3]* block; the [L5] blocks only as Q2 is ruled. No
`<script>` (site.rs:22). Effective date and version line read `V`. The table in §3.8 needs a table
style in `site/site.css` if none exists (console-ui checks; that file joins L4.2's list).

**L4.3 (the bump).** `PRIVACY_VERSION = V` (`account.rs:25`); `PRIVACY_BUMP_1 = Some(V)`
(`app/tests/grades.rs:183`), which must be later than `2026-09-24` (`:206-207`). Green:
`the_privacy_version_constant_is_the_published_pages_date`, the date-and-bump tests,
`record_consent`'s request test (`app/tests/account.rs:1325` pins a literal `2026-09-17` body, which
is a fixture string, not the constant: leave it). `V` is the date of L4's merge commit.

**L4.4 (cloud).** Today `POST /account/consent` writes nothing once any `tos` row exists
(`handler.ts:173`), so an existing account can never record a later privacy version, and no route
returns the recorded one. Behaviour wanted:
- `GET /account/consent` answers the account's recorded `privacy_version` (null when none).
- A request carrying a `privacy_version` later than the recorded one writes one `privacy` consent
  row and moves `accounts.privacy_version`, and nothing else (no `tos`, no `age_18` restamp: the
  guard in `index.ts:121-124` stays).
- An equal or older version is a silent 200, as a repeat sign-in is today; a non-date string is a
  400 (today's `MAX_VERSION` bound stays).
- Only if Q4 is ruled (a): forwarded-mail intake (`mail-process`) leaves alone the mail of an
  account whose `accounts.privacy_version` is older than the server's current version, with a
  named outcome, never an error. That adds `cloud/supabase/functions/mail-process/*` and a way for
  the server to know the current version to L4.4's files; the plan settles both.
Tests (Deno, red first): the three cases above; the first-sign-in path unchanged; the export
(`/account/export`) shows the new row.

**L4.5 (app).** A command (name for the plan) that reads the recorded version and answers whether
`PRIVACY_VERSION` is newer, and one that records it. Network down: answers "unknown", never
"accepted". Tests in `app/tests/account.rs` against a `127.0.0.1:0` server (CLAUDE.md's rule).

**L4.7 (screen).** Behaviour depends on Q4. Under the recommendation: shown at launch, over the
main window, for a signed-in profile whose recorded version is older; it names what changed in
three or four lines, opens the page through `open_policy("privacy")`, and records the yes. Until
the yes, the scheduler's cloud steps skip with a named reason (contract: a step that cannot run is
named, never run-and-failed). L4.6 pins the screen's ids, its link and the skip wording.
The skip itself is `app/src/scheduler.rs` work: a task **L4.5b** for `implementer` (high), after
L4.5, test first in `app/tests/scheduler.rs` (`run_slot_end_to_end` stays ignored).

**File overlap, named.** L4.3 and L4.5 both edit `app/src/account.rs`. They are not disjoint, so
one `contract-engineer` dispatch does both, in two commits (the bump with L4.2's page; the commands
after L4.4), and `contract-reviewer` reads both.

## 5. Open questions for Quinn, each with a recommendation

**Q1. Does the one-line summary change?** It is pinned in three places (`site.rs:15`, the page,
`console.js`'s `PRIVACY`). It stays true: forwarded mail and grades are "beside your account" or in
the vault copy. But the next line, "That is the whole policy in one line", reads thinner once a
section on forwarded mail exists. *Recommend:* keep both word for word; the new sections carry the
detail. Changing the sentence means a `console.js` edit and the wizard's copy too.

**Q2. The syllabus lines before L5's code exists.** Q42 puts §6.1 items 2-5 on L4's page before any
L5 surface reaches `main`; this note's rule is that every sentence is true of the code. *Recommend:*
carry items 2-5 with "(Knowlu does not read syllabi in this version; the app will say when it
does.)", the 2026-09-24 page's own Gmail precedent, true today. The alternative, leaving the lines
out, breaks Q42's gate.

*Then the choice for Quinn: what removing the parenthesis is.* Starting a use the page said was off
is "a new use for what we already hold" (`privacy.html:120`), and this draft treats Gmail's
identical parenthesis that way: its removal is part of bump #1 (§0 item 1, Gmail §6 item 1, §4.1
item 3). If L5 removed the syllabus parenthesis as a "Last changed" edit, syllabus text would reach
a model under a consent given when the page said it would not.
- **(a) Recommended: the removal is material.** L5's go-live task removes the parenthesis, moves
  `PRIVACY_VERSION` (and the page's Effective and version lines) to its own date, and every account
  re-accepts through L4's screen before syllabus reading starts. Cost: L5's go-live becomes a
  contract-list edit (`account.rs`, `contract-engineer` then `contract-reviewer`) and a re-prompt
  for every account then existing; L5's plan carries both, and Q42's gate gains "and the version
  moves with it". A third cost, from Q4: every device call that reads a syllabus (a slot step or a
  console command) waits on the same not-yet-accepted check as L4.5b's skip, so syllabus reading
  never runs on the old consent. Server-side gating (Q4 (a)) is not needed for it: the device
  fetches, extracts and sends each excerpt (L5 spec G3, `2026-10-05-syllabus-class-times-design.md:184-186`),
  and nothing reads a syllabus without the device asking (see Q4 (b)'s tripwire).
- **(b) Q3's smaller-change rule excludes it.** Same effect, stated as a rule on the page rather
  than a one-off: §3.11's added sentence. Under Q3 (ii), the recommendation, it reads "Reading a
  kind of material this page says Knowlu does not read, such as syllabi, is never a smaller change";
  under Q3 (i), "Turning on a use this page said was not yet on is never a smaller change". (a) and
  (b) are compatible; the draft carries both, and Q3 shows what the two wordings do to R12b.
- **(c) Not recommended: a "Last changed" edit only.** Cheapest; it is the gap the review found.

**Q3. C3 Q3 against the page's own change rule.** C3 Q3 has R12b change the page with
`PRIVACY_VERSION` unmoved. But the page promises every change a new date (`privacy.html:120`), and
the test pins the page's Effective date to `PRIVACY_VERSION` (`static_assets.rs:1673-1674`), so a
new date forces a new version and a re-accept. *Recommend:* L4 separates the two: "This page is
version `V`" (the consent version, pinned to `PRIVACY_VERSION`) and a new "Last changed" date that
a non-material edit moves alone; the Changes paragraph says so (§3.11), and says that reading a
kind of material the page says Knowlu does not read, such as syllabi, is never a smaller change
(Q2 (b), in the narrow form that (ii) below recommends). Q2's removal is then
material and moves the version. The Effective and version lines stay tied to `PRIVACY_VERSION`, so
neither `account.rs` nor the existing test changes; L4.1 adds one pin that the "Last changed" date
is a real date no earlier than `V`.

*A consequence to rule on with it: R12b.* The page's M365 bullet says Knowlu "does not yet bring
in older mail, or mail that arrived while forwarding was off". R12b turns that on, uploading
older and missed mail from the device. Under the broad sentence ("Turning on a use this page said
was not yet on…") that is a use the page said was not yet on, so R12b would move
`PRIVACY_VERSION`, against C3 Q3's ruling that it does not. Two ways through: **(i)** R12b moves
the version and re-prompts, and C3 Q3 is amended (consistent with Q2 (a); costs a contract-list
edit in R12b and a second re-prompt); §3.11 then carries the broad sentence. **(ii)** C3 Q3
stands, on the reasoning that bringing in mail that already reaches a connected mailbox is the
same use (forwarded mail read for school) by a new path, not a new use; the page's M365 bullet then
says "does not yet" as a delay, and §3.11 carries the narrow sentence, which names its case
outright: "Reading a kind of material this page says Knowlu does not read, such as syllabi, is
never a smaller change." The page says Knowlu reads forwarded mail, whole (§3.8), so Microsoft 365's
older mail is not "a kind of material this page says Knowlu does not read"; syllabi are (§3.2's
parenthesis). *Recommend (ii):* forwarded mail is already disclosed whole, and D18's upload row was
signed for exactly this. §3.11 and the L4.1 pin are drafted in (ii)'s form, with (i)'s sentence
beside them as the alternative.

**Q4. What the re-consent screen does before a yes.** The page says "Carrying on using Knowlu is not
how we take consent for a materially different use" (`:120`), so the new uses must not run on the
old consent. *Recommend:* a blocking panel at launch with **Accept** and **Read the page**; until
Accept, the vault, today's page and every local edit work as usual, and the slot's cloud steps
(`sync`, `coursework`, `ingest`, `grades`, `judge`, `mail`) skip with "privacy update not accepted".
No decline button: deleting data stays in Settings. *Cost:* L4.5b in the scheduler. *Alternative:*
the panel only, with steps running (cheaper; weaker against the page's promise).

*The skip covers the device only.* Forwarded mail is processed on the server: Amazon's intake and
the `mail-process` cron keep reading an account's mail whatever its recorded privacy version, so
on the server the new uses do run on the old consent until the yes. Choose one:
- **(a) Intake checks the version.** `mail-process` leaves alone an account whose
  `accounts.privacy_version` is older than the current one (a conditional bullet in L4.4, §4.3).
  Cost: the server needs its own notion of the current version, and mail it leaves alone is lost
  at Amazon's one-day expiry or held longer than the page says.
- **(b) Quinn accepts the gap for founder accounts.** Per Q5 only accounts that existed before `V`
  are ever on an older version, and those are Quinn's own; every later account accepts `V` at
  sign-up. *Recommend (b)*, recorded in the plan as a ruling, with a tripwire scoped to what the
  device cannot gate: if a later version bump turns on, or changes, a use that runs on the server
  without the device asking (forwarded mail's intake and `mail-process` are today's), and any
  non-founder account predates that bump, (a) is built first. Syllabus reading (Q2 (a)'s bump) does
  not fire it: the device fetches, extracts and sends each excerpt (L5 spec G3), so the device's
  not-yet-accepted check, a cost Q2 (a) now lists, covers it. L4.0 also lists any other server-side
  job that reads an account's mail on a schedule, so the ruling names them all.

**Q5. Re-consent for testers.** Testers who sign up after L4 accept `V` at sign-up and never see the
screen; only Quinn's accounts are re-prompted. *Recommend:* fine as is; no ask beyond confirming
it, since the Pilot gate wants the screen to exist, not to have fired.

**Q6. The judgment record keeps the email's sender.** For a Gmail email the record's fields carry
the full "From" (name and address) as the rules' `source` feature (Appendix A c40), kept until
the account is deleted; promoted per-account rules can key on it. Today's page does not say so.
*Recommend:* disclose it now ("who it was from", §3.3, §3.7, §3.9), which also pre-states Gmail
Q7 (c)'s sentence. The alternative is a code change to keep only the sender's domain.

**Q7. "Deletes the Sent copies" or "moves them to Trash".** D18's words say deletes; the script
presses Gmail's Delete, which moves to Trash (`sent_cleanup.js:16-17`). *Recommend:* the page says
Trash (§3.8). D18's VISION and spec wording is untouched by L4.

**Q8. Microsoft 365 today.** The page says Knowlu does not yet bring in older Microsoft 365 mail,
or mail that arrived while forwarding was off. *Recommend:* yes; it is the code (c33: no backfill
and no recover pass for a held provider) and matches C3 Q1's console copy. R12b replaces it.

**Q9. Four months, not "the term's start".** VISION and the spec say the backfill reaches the term's
start; the code's window is 120 days before setup (c42). *Recommend:* the page says "up to about
four months before you set it up"; a VISION wording fix is the docs lane's, not L4's.

**Q10. D18's "our function's memory".** D18's signed row says "our function's memory"; the page
elsewhere says "our server's memory". *Recommend:* R12b uses D18's words exactly, as ruled; no
change.

## 6. Could not settle here

- **Amazon's one-day expiry** is an S3 lifecycle rule set in the AWS console, not in the repo;
  `sweep.ts:5-9` relies on it. The "about two days" sentence (c29) has no code line. *Owner:* the
  controller reads the rule (counts and names only) at L9, before the page ships. *Status
  2026-10-20: still open.*
- **A lapsed mailbox may never read `active` again: RESOLVED on the merged tree** (L4 T1, 2026-10-20).
  The lapsed-mailbox fix (migration `20261019000100_mail_last_lapsed_at`, ledger rows 133, 135, 143)
  makes the old reading untrue. What the code says now: intake moves a mailbox into `lapsed` and keeps
  an earlier `lapsed_since` (`mail-inbound/store.ts:97-113`); the next entitled `mail-pull` or
  `gmail-read` moves `lapsed` to `quiet` (a canary ever arrived) or `pending`
  (`_shared/mail_unlapse.ts:7-9`: "never `active`, since nothing proves the forward survived the
  lapse"), and a canary through the current token lifts it to `active` (`mail-inbound/store.ts:183-192`);
  `lapsed_since` "stays until the device reports the gap covered" (`mail-pull/handler.ts:32-33`), and
  only that report clears it (`mail/store.ts:323-328`), which the app sends after a whole pass
  (`mail/slot.rs:71-80, 253-260`). So the Gmail pass is due at every slot only until it covers the gap,
  the hourly delivery check reaches the mailbox again once it reads `quiet` (`mail-canary/handler.ts:50`,
  `SENDABLE` is `active, quiet`), and a lapse still reaches back at most 30 days
  (`mail/recover.rs:91, 126`). The §3.8 sentence ("or up to 30 days back after a lapse") is true
  either way and needs no change. `last_lapsed_at` is a timestamp on the mailbox row, which "the facts
  that show forwarding works" covers (ruled: no new data class). *Owner:* none; closed.
- **Windows encrypting the kept sign-ins' cookies** (c4, c9) is WebView2's behaviour, not a Knowlu
  line; the grades and email specs assert it. No test proves it. *Owner:* L4.9's reviewer decides
  whether the sentence needs a source note. *Status 2026-10-20: still open.*
- **`EF` line numbers** were read from the email-forwarding worktree at commit `5f71623`
  (`5f716238753d1a9ddb02a692e40a3b538d649b35`) with uncommitted changes then in `mail/backfill.rs` among
  the cited files. **Re-read on the merged tree 2026-10-20 (L4 T1; lane branch cut from `ba7eb0b`):**
  every cited line was mapped through `git diff -U0 5f71623 HEAD` and read at its new number; moved
  ones are updated in Appendix A. Still open: nothing for the code. The email spec's §7 citations had
  to be re-found by content (§7 is `:967-1012`).
- **The registrar code** (#19) is unmerged; c18's "at schools Knowlu supports" holds with zero
  supported schools until it merges, and was not checked against #19's branch. *Status 2026-10-20:
  still unmerged (`app/src` has no registrar module).*
- **The re-consent route's shape** (L4.4) is a design choice for the L4 plan; nothing on `main`
  returns a recorded version today. *Status 2026-10-20: still open; T2 settles it.*
- **Unchanged sentences** (`⟨=⟩`) were true on 2026-09-24 and were not re-read line by line here.
  *Status 2026-10-20: still so.*
- **Facts T1 settled for later tasks (2026-10-20).** (a) *Scheduled jobs that read an account's mail*
  (the Q4 (b) tripwire's list), all in `20261002000200_mail.sql`: `knowlu-mail-process` (`:605`, every
  minute, the `mail-process` function), `knowlu-mail-canary` (`:631`, hourly at :11, `mail-canary`, which
  reads `mail_addresses` to send the delivery check) and `knowlu-mail-sweep` (`:657`, daily 07:47, the
  `mail-process` function in `sweep` mode). `mail-inbound` and `mail-pull` run on a delivery and a
  device call, not on a schedule. No other cron reads mail text: `knowlu-promote-rules`
  (`20260911000500_rule_promotion_fix.sql:251`) reads decision records, which keep a sender (c40), and
  the rest (billing, tombstone, issue, Google-state, Gmail-queue and `gmail_seen` sweeps, `sync_prune`)
  read none. (b) *Cascades:* `judgments.account_id` and `usage_daily.account_id` each `references
  public.accounts(id) on delete cascade` (`20260911000100_judgment_service.sql:69` and `:153`).
  (c) *Models:* `20261020000100_mail_max_tokens_repin.sql` moves only the two mail rows' `max_tokens`
  (1280, 1536); the model, host, route and price are as `mail.sql:343-360` pinned them, so c23, c27 and
  c28 stand.

## 7. Fidelity ledger

| Source requirement | Where here | Task | Check |
|---|---|---|---|
| Amendment 2026-10-19: bump #1 = L4, no lawyer read | header, §4.1 item 3 | L4.3 | `PRIVACY_BUMP_1 = Some(V)` |
| Gmail §6 item 1: drop `:77`'s parenthesis; disconnect text under Q9 (a)(ii) | §3.7 | L4.2 | L4.1 ban and pin |
| Gmail §6 item 2: the connection entry | §3.3 | L4.2 | reviewer, against `google.sql:7-25` |
| Gmail §6 item 4: the Gmail retention bullet | §3.9 | L4.2 | 30 and 7 days re-read (c49) |
| Gmail §6 item 5 / Q7 (c): "who the message was from" | §3.3, §3.7, §3.9 (Q6) | L4.2 | c17, c40 |
| Gmail §6 ordering: page before prod `GOOGLE_CLIENT_ID` | §4.1 item 4 | L9 checklist | controller |
| D18 items 1-7 | §3.4 (1), §3.8 (2), §3.3 (3), §3.9 (4), §3.5 (5), §3.2 (6), §3.7 (7) | L4.2 | L4.1 pins |
| D18 upload row and mailbox-actions sentence, after C3 (C3 Q3) | §3.8 [after C3] | R12b, not L4 | Q3, Q10 |
| D25/A13: kept sign-ins at every school, for now | §3.2 | L4.2 | pin |
| M2 §5: interests and event listing | §3.4 | L4.2 | pin "600 characters of the interests" |
| Grades §11: kept session; grades the account holds | §3.2, §3.3, §3.9 | L4.2 | pins; c4, c13 |
| #19 §8 Q5: registrar sentence, "at schools Knowlu supports" | §3.4 | L4.2 | pin |
| L5 §6.1 items 2-5 | §3.2, §3.4 [L5] | L4.2 (as Q2 rules) | Q42's gate in L5's plan |
| `privacy.html:120`: a new use is not consented by carrying on | §3.11, Q2, Q3 | L4.2; L5's go-live (Q2 (a)) | L4.1 pin; L5's plan moves the version |
| R-PS-4: the re-consent screen with the version move | §4.2 L4.4-L4.7 | L4.4, L4.5, L4.5b, L4.7 | §4.3 tests |
| `privacy.html:120` on the server: forwarded mail before the yes | Q4 | L4.4 (Q4 (a)) or a ruling (Q4 (b)) | Quinn |
| `account.rs:21-23`: constant and page date in one commit | §4.2 | L4.2 + L4.3 | `static_assets.rs:1666-1674` |
| Task: M365 guided until C3 (no backfill, no recover) | §3.2, §3.8 | L4.2 | c6, c33; L4.1 pin |
| Task: Gmail forward-and-delete backfill and recover | §3.8 | L4.2 | c31, c42, c42a, c43 |
| VISION commitments 2, 3, "Knowlu reads", telemetry, minimization | §1 | all | reviewer |

## Appendix A. Evidence for each claim (never published)

`main` = the main checkout at `C:\Users\danie\GitHub\knowlu`, read 2026-10-19. `EF` = the
email-forwarding branch, read (never written) in its worktree on 2026-10-19 at `5f71623` plus
the uncommitted changes §6 lists (only `mail/backfill.rs` among the cited files). **Re-read
2026-10-20 (L4 T1) on the lane branch, cut from `ba7eb0b` (after the email merge `6fd44ec`):**
every `EF` line below now names a line of that tree, found by mapping each old number through
`git diff -U0 5f71623 HEAD` and reading the code at the new number; a `main` row was re-read
where the merge touched its file (`index.html`, `gmail-read/handler.ts`, `judge_prompts.ts`,
`account.rs`) and left alone where the file is the same on both sides. The email spec's §7
citations were re-found by content (the old numbers no longer land in §7, which is `:967-1012`). A claim
that moved has its new numbers in place; the one claim the merged code no longer makes is in
§6 (the lapsed mailbox). "No code line" means the claim rests on a spec or on platform
behaviour, said so in §6.

| Tag | Claim | Evidence |
|---|---|---|
| c1 | version line equals the constant | main `app/src/account.rs:21-25`; `app/tests/static_assets.rs:1666-1674` |
| c2 | only the vault is copied; the rest stays | main `app/src/grades.rs:271-272` (kept session never synced); EF `app/src/mail_session.rs:1-12` (profiles under the data folder) |
| c3 | commitment questions and the series list stay local | main `engine/src/commitments.rs:27` (`LOCAL_CARD_KINDS`); the tripwires CLAUDE.md names (`sync_keeps_every_local_card_kind_local`) |
| c4 | Blackboard session kept in a browser profile | main `app/src/grades.rs:263-275, 584`; encryption: no code line (§6) |
| c5 | Forget Blackboard sign-in deletes it | main `app/static/index.html:212` (the "Forget Blackboard sign-in" button; was `:203` before the email merge); `app/src/grades.rs:285-294` |
| c6 | a saved sign-in per mailbox, its uses; missed mail for Gmail only | EF `app/src/mail_session.rs:4-5`; `app/src/mail/slot.rs:1-18`; `app/src/mail/backfill.rs:9-14`; Gmail only: `mail/slot.rs:184-189` (returns before reconcile and recover when `backfill_held_for_c3`), `mail/backfill.rs:16-18` |
| c7 | on by default; deleted on off, disconnect, sign-out, deletion | EF `mail_session.rs:88-89` (the `keep_session` field), `:234` (`keep_session: true`); `mail/commands.rs:176-192` (off: `forget_all`, `:189`), `:346-350` (disconnect: `forget`); `account.rs:668-684` (sign-out), `:1274-1283` (deletion) |
| c8 | no password kept; never leaves | main `grades.rs:271-272`; EF `mail_session.rs:4-9`; no upload path names either folder |
| c9 | cookies encrypted, cache not | email spec §7 (`2026-09-30-email-forwarding-design.md:1001-1003`; was cited `:897-899`); no code line (§6) |
| c10 | [L5] syllabus files stay local | L5 spec §6.1 item 2; no code yet (Q2) |
| c11 | both sign-ins on everywhere, for now | main `app/src/grades.rs:51` (`Suspended`); cloud design `:683`, `:740-741`; EF `mail_session.rs:88-89, 234` (no school check) |
| c12 | decided again at 20 paying users | cloud design `:683` (review trigger); policy, no code line |
| c13 | commitments and grades sync | main `engine/src/ids.rs:24` (`NOTE_FOLDERS` ends `commitments`, `grades`); `engine/src/sync.rs:268` |
| c14 | the Google connection's fields | main `cloud/supabase/migrations/20260911000200_google.sql:7-25` (`google_sub`, `secret_id` into Vault, `email_hint`, `scopes`, `connected_at`, `last_read_at`, `status`) |
| c15 | forwarding sends everything, whole | EF `mail.sql:41-57` (one row per mailbox, provider); email spec §7 `:969-970` (was cited `:865-866`) |
| c16 | forwarding address encrypted; mailbox address and forwarding facts kept; all go on Disconnect | EF `mail.sql:46-54`, `:55-56` (`forward_hint`, e.g. the M365 tenant id), the row's other columns, all in `mail.sql:42-78` and kept until Disconnect: `status`, `lapsed_since`, `reenabled_at`, `confirm_until`, `backfill_since`, the timestamps `last_received_at`, `last_canary_sent_at`, `last_canary_ok_at`, the counters `canaries_today`, `unbound`, `unauthenticated`, `expired`, `unbound_own_since_live`, `unbound_foreign_since_live`, and `last_lapsed_at` (added by `20261019000100_mail_last_lapsed_at.sql:13`, a timestamp the export lists, `account/mail.ts:111`); `:371-384`; `cloud/supabase/functions/mail-canary/query.ts:21` (checks go to the mailbox address) |
| c17 | the record keeps who it was from | main `cloud/supabase/functions/_shared/judge_pipeline.ts:81-92`; `_shared/judge_rules.ts:35-37, 65-79`; `gmail-read/handler.ts:380, 398` |
| c18 | discard-everything window; [#19] class schedule | main `app/src/lms_link.rs:17-44, 177-180`; `docs/plans/2026-10-18-p3-registrar-close-out-plan.md:92, 855` |
| c19 | the one exception | main `app/src/grades.rs:263-275` |
| c20 | task prompt bounds | main `_shared/judge_prompts.ts:15-17, 163-173` |
| c21 | event listing and 600 characters of interests | main `judge_prompts.ts:177-191`; `engine/src/events.rs:444`; `app/src/commands.rs:289-292` (M2's editor) |
| c22 | Gmail email prompt, with course codes | main `judge_prompts.ts:194-213` (`:194`, `:206`: "Known course slugs") |
| c23 | forwarded mail: 2,000 characters, course codes, time zone, DeepInfra | EF `_shared/judge_prompts.ts:237, 352-378` (`:361-374`: known course codes and the time zone, "nothing else"); `mail.sql:343-360`; `mail-process/handler.ts:21`. Not "once": a `model failed` item (no answer from upstream, network or clock) is refunded and retried the next UTC day, up to its third attempt (`mail-process/model.ts:12-13, 19-21`; `handler.ts:21-22`), and the host may have had the text before the failure, so §3.8 says "read by", not "read once by" |
| c24 | resolve: titles and due dates | EF `judge_prompts.ts:390-410` |
| c25 | [L5] syllabus call bounds | L5 spec §6.1 item 3, Q38 (`:909`, `:1658`); no code yet |
| c26 | [L5] never shared or pooled | L5 spec §6.1 item 5; no code yet |
| c27 | seven companies; two model readers | main `migrations/20260916000100_provider_swap.sql:29-46`; EF `mail.sql:348-358` |
| c28 | DeepInfra reads forwarded mail too | EF `mail.sql:348-358` |
| c29, c29a | Amazon holds it until read, about two days | EF `mail-process/sweep.ts:5-9`; `mail-process/handler.ts:28-30`; the lifecycle rule: no code line (§6) |
| c30 | the delivery provider sends the checks | EF `mail-canary/index.ts:2-6` (Resend, `billing-jobs`' key) |
| c31 | Sent copies to Trash | EF `app/assets/mail/gmail/sent_cleanup.js:1-17` |
| c31a | Gmail disconnect deletes; record stays; one grant | main `migrations/20260929000200_gmail_disconnect_purge.sql:12-18, 33-38`; `google-connect/disconnect.ts:4-7`; `google-connect/handler.ts:95-129` |
| c32a | a forwarded Gmail address is read by forwarding only | EF `gmail-read/handler.ts:7-9, 196-200, 299-309` |
| c33 | Microsoft 365 backfill and recover held; mail during a forwarding gap not brought in | EF `app/src/mail_steps.rs:86-95`; `mail/backfill.rs:16-18, 70-82`; `mail/slot.rs:184-189` (returns before reconcile and recover) |
| c34 | read in memory, text never written | EF `mail-process/handler.ts:32-34` |
| c34a | lapsed: deleted unread | EF `mail-inbound/handler.ts:12-15, 111-114, 470-475` (no entitled mailbox left: deleted unread, no `mail_seen` row); entitled means `active` or `trialing` (`_shared/entitlement.ts:19-22`; `past_due`, `canceled` and `none` are not) |
| c35 | the screen before any model ("such as": not the full list) | EF `mail-process/handler.ts:10-16` (also screens Microsoft bulk score >= 7, the student's own mail outside a batch, and unaligned live mail, which intake deletes) |
| c35a | mail recognised by shape read by rules, no model | EF `mail-process/handler.ts:17-20` (template-only claims) |
| c36 | what is kept, and for how long | EF `mail.sql:159-175`; `sweep.ts:12-13, 80`; `_shared/judge_validate.ts:197-199` (140-character quote) |
| c37 | attachments never opened, backfill and recover exception | EF `mail-process/handler.ts:8-9`; the exception: `mail-inbound/handler.ts:24-26, 433-459` (a batch send's `message/rfc822` attachments each become one work item of the batch's phase, backfill or recover; the outer text and every other attachment dropped unparsed) and `_shared/mail_bind.ts:476` (a backfilled or recovered original); email spec §7 `:984` (was cited `:880`) |
| c38 | headers in Amazon's notice, never logged | email spec §7 `:971, 983` (was cited `:879`); `mail-inbound/handler.ts:208` (the one `log`, `console.log` of a fixed phrase; every call at `:195-196, 280-510` is a fixed phrase or a count, none takes a header; `confirm.ts:197-209` logs a URL's shape, `hopShape`) |
| c39 | keyed fingerprints, 180 days | EF `mail.sql:114-128`; `sweep.ts:79` |
| c40 | decision records kept until account deletion; sender in Gmail's | main `judge_rules.ts:35-37, 75-76`; `gmail_disconnect_purge.sql:12-14`; EF `mail.sql:211-213` |
| c41 | forwarding on at setup, off at Disconnect | EF `app/assets/mail/gmail/forwarding_set.js`; `mail/commands.rs:294-317, 350-401` (the off half) |
| c42 | older Gmail mail, 120 days, as attachments | EF `mail/backfill.rs:1-5`; `gmail/forward.js:1-3` |
| c42a | Gmail recover: about daily, the last three days; further back after forwarding was off (to last known on, at most 120 days) or a lapse (at most 30 days); lapsed mail deleted unread; forwarded and Sent copies trashed the same way | EF cadence: `mail/slot.rs:43-53, 60-68` (`Daily`, a 20-hour gap, every `active` mailbox; `quiet`, `off`, `lapsed` due at once); `mail/slot.rs:190` (last known on), `:202-212` (`lapsed_since`, the window, phase `Recover` at `:211`); window: `mail/recover.rs:87-91` (`WINDOW_DAYS` 3, `LAPSE_DAYS` 30), `:120-132` (`window`: `since.min(l.max(now - LAPSE_DAYS * DAY))`, so a lapse reaches back at most 30 days, and `k.max(now - BACKFILL_DAYS * DAY)`), `app/src/mail_steps.rs:35` (`BACKFILL_DAYS` 120); lapsed mail: `mail-inbound/handler.ts:12-15, 111-114, 470-475` (deleted unread); the pass waits for entitlement: `app/src/scheduler.rs:530-532`; "the same way": `mail/recover.rs:1-5, 14-18` (one routine for backfill and recover, forwards as attachments, Sent copies removed), `:1170-1181, 1484, 1580` (the Inbox look), `:1606` (`sent_cleanup.js`); `mail.sql:83-88` (`mail_batches` phase `recover`). **The lapse fix (read 2026-10-20, ledger rows 133, 135, 143):** a lapsed mailbox now leaves `lapsed` on the next entitled `mail-pull` or `gmail-read` (`_shared/mail_unlapse.ts:17-33`: `quiet` if a canary ever arrived, else `pending`; `mail-pull/handler.ts:29-33, 260-262`; `gmail-read/handler.ts:299-307`), or on a canary through the current token (`mail-inbound/store.ts:183-192` sets `active`, `lapsed` included), and `lapsed_since` is cleared only by the device's `covered` report (`mail/routes.ts:28-31`; `mail/store.ts:323-328`: `lapsed_since` and `last_lapsed_at` must both still match and the status be neither `lapsed` nor `revoked`). The device sends `covered` after a whole pass over a mailbox that is not `lapsed` whose window began at or before the later of `lapsed_since` and 30 days back (`mail/slot.rs:71-80`, `:253-260`). The wording stays an upper bound on purpose and is true either way: a lapse still reaches back at most 30 days (`recover.rs:126`), and after `covered` the window is the usual three days again. `last_lapsed_at` is one more timestamp on the mailbox row (`20261019000100_mail_last_lapsed_at.sql:13`): it falls under "the facts that show forwarding works" (c16), no new data class |
| c43 | the Inbox look, stop on a report | EF `gmail/bounce.js:1-19`; `gmail/forward.js:15-16` |
| c44 | turned back on and announced; never if turned off; never "elsewhere" | EF `mail/slot.rs:11-17, 341-354`; `app/src/mail.rs:780-793` |
| c45 | nothing else changed or sent; no one else (the list above it includes recover, c42a) | email spec §7 `:994-999` (was cited `:893-895`); EF `gmail/sent_cleanup.js:10-12`, `gmail/forward.js:23-30, 339-342` (one recipient, Knowlu's) |
| c46 | the delivery check | EF `mail.sql:631-655` (hourly job); `mail-canary/index.ts:2-6` |
| c47 | no forwarded mail in telemetry | EF `mail.sql:211-213` (telemetry's kinds not widened) |
| c48 | not trained on, not pooled | EF `mail.sql:205-224, 249, 281, 299` |
| c49 | Gmail retention | main `migrations/20260911000900_final_review_fixes.sql:48, 72`; `gmail_disconnect_purge.sql:12-14, 33-34` |
| c50 | Disconnect deletes all but the decisions | EF `mail.sql:371-384` |
| c51 | export includes mailboxes and undelivered findings | EF `cloud/supabase/functions/account/index.ts:68-70`; the mailbox columns the export lists: `account/mail.ts` (`last_lapsed_at` at `:111`) |
| c52 | Delete turns forwarding off first, never blocks | EF `app/src/account.rs:1378-1416`; email spec §4.13 |
| c53 | Delete removes both kept sign-ins | EF `account.rs:1274-1284` (the data folder holds `lms-session`: main `grades.rs:271-273`) |
