# Privacy bump #1: what changed on `site/privacy.html`, for the before-Pilot lawyer read

Written 2026-10-20 by the L4 docs task (T14), from the diff of `site/privacy.html` between `cc8b162`
(the 2026-09-24 page) and L4's bump commit `d4ee7ab` (version **2026-10-20**). Nothing here is new page
text: each item names the sentence by its place on the page and says in a few words what it adds or
changes. The page itself is the packet's text; this note is the index of what moved and which signed
spec required it. The lawyer read of the interim page stays owed before the Pilot (cloud design,
Amendment 2026-10-19; HANDOFF §3), and its version re-prompts everyone through the panel L4 built
(`PRIVACY_CHANGES` in `app/static/console.js` is keyed by version).

Page-wide: the Effective line, the version line and a new "Last changed" line carry `2026-10-20`. The
policy-change section now says a description that names no new category, company or use gets a new
"last changed" date and nothing more, and that reading a kind of material the page says Knowlu does not
read (syllabi are the example) is never a smaller change.

## Gmail connect spec, §6 (Q8, Q9)

- "What stays on your machine / Gmail" section: the parenthesis saying Gmail is not connected in this
  version is gone. Disconnecting is now described as revoking access at Google and deleting the stored
  connection, the list of emails already read and proposals not yet on the computer; the record of each
  decision (message id included) stays until the account is deleted; Calendar and Gmail disconnect together.
- "What we collect": a new entry, "Your Google connection, if you make one" (the permission, stored
  encrypted and never sent to the computer; the Google account's id and email address; which of Calendar
  and Gmail; when connected and when mail was last read; whether it still works).
- "How long we keep it": a new Gmail bullet (the list of emails read for 30 days; a waiting proposal
  until it arrives and then 7 days; both deleted at once by a disconnect or an account deletion).
- Item 5 (Q7 (c)): "who the message was from" is added to what is kept of each decision, in "What we
  collect", the Gmail section and "How long we keep it". This is the Gmail card's sender, which the page
  now states is stored.
- One sentence added to the Gmail section: forwarding the same Gmail address to Knowlu means it is read
  through forwarding only.

## Email forwarding spec, §7 (D18 items 1-7, D25)

1. The Gmail disconnect text above (item 1 of D18's list, with Gmail §6).
2. A new section, "Email you forward to Knowlu": optional, set up in Knowlu; everything that reaches a
   connected mailbox goes to a private Knowlu address, school and personal mail alike; Amazon's mail
   service carries it and its notice carries headers only; it waits in Amazon's storage until read (up to
   about two days with Amazon's lag); mail arriving while the subscription is not active or trialling is
   deleted unread; the rules that set mail aside before a model reads it; the model's host reads subject,
   sender, date and up to the first 2,000 characters of the rest, with course codes and time zone, and
   keeps none of it; attachments are never opened, except the messages Knowlu itself forwards as
   attachments when it brings in older or missed Gmail mail; what is kept is what was found, not the email.
   A table lists each datum, where it lives and how long (the whole message, the headers, the model call,
   the mailbox address and delivery facts, the keyed fingerprint for 180 days, what was found, the record
   of each decision, the forwarding address, the saved mail sign-in).
3. "What we collect": entries for email you forward, and for the forwarding addresses (the private address,
   the mailbox's own address, when mail and delivery checks last arrived, and for Microsoft 365 the school's
   tenant id).
4. "How long we keep it": a forwarded-email bullet (180 days for fingerprints; 7 days after the computer
   takes what was found, or 30 if it never does).
5. "Who else touches it": a seventh company, Amazon Web Services; and the email provider line now also
   names the delivery checks Knowlu sends to a mailbox you forward from.
6. "What stays on your machine": a "Kept sign-ins, if you use them" entry covering the Blackboard sign-in and
   the saved mail sign-in per mailbox (encrypted by Windows for the Windows account, no password kept,
   never leaves the computer, deleted by Settings, sign-out or account deletion), saying that a kept
   sign-in is full access to that account on this computer.
7. The Gmail "what is kept" sentence stays true and gains the sender (above).

Mailbox actions, stated as a list: forwarding on and off; for Gmail, forwarding older mail as attachments
and moving the copies those forwards leave in Sent to Trash, a daily and per-run catch-up of the last
three days, a look-back after forwarding was off or a lapse (up to about four months, or 30 days after a
lapse), and a look in the Inbox for a not-delivered notice; for school Microsoft 365, the page says
Knowlu does not yet bring in older mail; turning forwarding back on if it is found off (never for a mailbox
turned off in Settings, never redirecting forwarding that points elsewhere without asking). "Nothing else
is changed, sent or deleted." Telemetry and training: nothing from forwarded mail enters telemetry or
trains a model; Google's Limited Use policy does not bind forwarded mail, and Knowlu holds it to the same
rules anyway.

Not on this page, by design: D18's Microsoft 365 upload row and the changed mailbox-actions sentence. They
are C3's R12b, after L4, and `engine/tests/site.rs` carries a pin on the "does not yet" Microsoft 365
sentence that R12b deletes with it.

## M2 editing spec, §5 (Q8)

- "What we do not collect", the model-send sentence: "when a campus event is judged, the event's listing
  and up to the first 600 characters of the interests you have told Knowlu" is added to what travels to a
  model. Preferences and grade weights were already named.

## Grades spec, §11 (ruling 12)

- "What stays on your machine": the kept Blackboard sign-in (see D18 item 6 above), kept for grades,
  deleted by "Forget Blackboard sign-in" in Settings.
- "A copy of your vault" and "How long we keep it": the copy now includes the classes and other commitments
  you confirm and, if you connect Blackboard, your grades; grades are kept for as long as the account exists
  and deleted with it.
- "Kept sign-ins at every school, for now" (also D25 of the email spec): until Knowlu has 20 paying users,
  both kept sign-ins are on without any school's own policy being read first (the mail sign-in at every school,
  the Blackboard sign-in at every school with a Blackboard listing); a school with no listing still has no
  grades; the 20th paying account triggers a new decision, which would be a new version of the page.
- "No campus password": the exceptions are now named (the two kept sign-ins, neither keeps the password), and
  "Sessions live in Windows Credential Manager" is narrowed to the Knowlu sign-in, with the kept sign-ins named
  as the exception.

## Registrar close-out (#19), §8 Q5

- "No campus password": what the sign-in window fetches is now "your calendar link, your course list and, at
  schools Knowlu supports, your class schedule" (the registrar read; the wording is Q5's).

## L5 syllabus spec, §6.1 (items 2-5), ahead of L5's code (Q42)

- "What stays on your machine": your syllabus files are held on this computer, and the opening paragraph says
  a syllabus leaves only as the lines described in "What we do not collect". The entry carries the
  parenthesis "Knowlu does not read syllabi in this version; the app will say when it does." That
  parenthesis is **true today and is removed at L5's go-live**, which is a new version and a re-prompt
  (draft Q2 (a)); L5's plan carries that bump.
- "What we do not collect": the bound on one syllabus call (at most 13,500 characters: the course's code,
  section and term; the meeting and office-hours lines with addresses, numbers and links removed; dated
  schedule lines; grading lines; late-work, makeup and attendance policy paragraphs; the instructor's contact
  details never sent) and "What Knowlu never does with a syllabus: share it, pool it across students, or
  keep a copy on our servers."

## Friends-testers spec (L8), Q7

- "What we collect": two new entries. "Daily counts": daily counts of AI calls and cloud requests, never
  what they were about, to keep the cost in bounds; in the export and deleted with the account. "Free
  tester access, if Knowlu gives it": the dates, a short note on why and an early-end reason, kept until 30
  days after it ends; a log of when it was given, extended and ended, kept at most three years; both in the
  export and deleted with the account; Knowlu looks at the daily counts and their cost to pay for them.

## Other sentences changed for accuracy (no spec item; L4's own, from the code)

- "What stays on your machine / Your vault": two things in it stay on this computer and are never copied to the
  account: the questions Knowlu asks about which calendar events are your classes (before you answer) and the
  list of repeating events they come from. Only what you confirm or decline is copied (the local card kinds in
  `commitments::LOCAL_CARD_KINDS`).
- The opening paragraph no longer says "these four things": it lists the kept sign-ins as staying on the
  computer, and app data as leaving only in an issue report you choose to send.
- "Export" no longer claims the file holds everything meaningful: it lists what it holds (now including
  connected mailboxes and unplaced findings, daily counts and free tester access) and says the record of each
  decision, the Google connection's details and Gmail proposals not yet on the computer are not in the file yet.
  A cloud follow-up for `exportAll` is open in the ledger.
- "Delete": it first tries to turn forwarding off in each mailbox with its saved sign-in and tells you if it
  could not (that never stops the deletion); it removes both kept sign-ins.
- "Security": the Knowlu sign-in lives in Credential Manager; the kept sign-ins are a browser profile on the
  computer and are the exception.

## For the lawyer packet beyond the page (C3 plan Q7, ruled 2026-10-19)

- The packet keeps **one automated-access item**, scoped by provider: the mailbox actions done with a saved
  session (for Microsoft 365, reading through Outlook's own services with the saved sign-in, no mail sent;
  Gmail's forward-and-delete as before), with the research on Microsoft's and Google's terms on automated
  access. It is a before-Pilot item and blocks no merge. The page does not yet carry C3's Microsoft 365 upload
  sentences (R12b).
- With it, the email spec's packet list: email forwarding to Knowlu and AWS, the saved mail sign-in, and the
  suspended university-policy reads (both kept sign-ins on at every school until 20 paying users).
- P5's facts and the terms (`docs/reports/2026-09-10-c1-quinn-packets.md`, HANDOFF §4's Pilot row) go with
  this note.
