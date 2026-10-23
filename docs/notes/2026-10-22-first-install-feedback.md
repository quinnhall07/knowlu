# First prod install: Quinn's onboarding feedback (2026-10-22)

Quinn installed 0.1.3 from knowlu.com, made a new profile on prod and went through onboarding as a
student would. This note records what they hit, in their words where it matters, and what the
controller checked. It holds no class, grade, schedule or address.

## What Quinn reported

1. **Order of asks.** "We should ask for Gmail access first, then if that's granted ... we don't have
   to do the email forwarding for Gmail." Google consent worked only through the unverified-app
   warning's *Advanced* link.
2. **Outlook forwarding.** Onboarding had Quinn set up Outlook forwarding by hand, "but my email was
   already forwarding to the email it told me to forward it to."
3. **Mail setup summary is wrong.** It read: "Forwarding is set. New mail is arriving. Done: Sign in,
   Step aside, Open forwarding, Test message. Not done: Bring in recent mail, Finish. Mail before
   24 June was not brought in." Bring in recent mail and Finish were reported skipped "even though I
   said 'I've done this'".
4. **"See what came from your email" does nothing** (Quinn suspects a popup is covered).
5. **Class times.** Onboarding allows one meeting time per class, "and that just won't work for me.
   It should be based off of syllabi."
6. **Old classes.** The "what are your classes" step listed past classes.
7. **Blackboard after onboarding.** "Why do I have to connect to Blackboard after all of that?
   Shouldn't that have been done during onboarding?" and: "people should be signing into their
   accounts during onboarding. The fewer the logins the better. If we can use one login for
   multiple uses, that is ideal."
8. **Old assignments.** "Why are old assignments brought in? ... I shouldn't have to micromanage
   assignments as soon as I create the account."

## What the controller checked (structure only)

- Prod has one mailbox: provider `m365`, status `active`, mail received, canary passed,
  `unbound` 0, `unauthenticated` 0, `expired` 0. `backfill_since` is **null**, which agrees with
  item 3: the backfill did not run. `mail_work` 0, `mail_claims` 0, `mail_seen` 1.
- Quinn's M365 forwarding now delivers to the prod mailbox, so staging's `in.knowlu.com` mailbox no
  longer receives Quinn's mail. Forwarding is on (T18c holds); only its target moved.
- Google sign-in first failed with "Unable to exchange external code": prod Auth's Google secret
  was wrong. Quinn added a new client secret and set it on prod; sign-in then worked. Prod's
  `GOOGLE_CLIENT_SECRET` function secret (Gmail connect) was not re-set and may hold the old value.

## Open questions this raises

- Gmail's read scope is a Google *restricted* scope. Past 100 users, or out of Testing, it needs
  Google's verification and a yearly security assessment. Until then every friend must be added as
  a test user and will see the unverified-app warning.
- Items 1, 5 and 7 are onboarding design (spec work). Items 3, 4, 6 and 8 look like bugs or
  thresholds in existing code and need root-causing before anyone designs around them.

## Later the same evening

9. **Old assignments, refined.** Quinn confirmed every live task is due today or later; the
   unwanted ones are work already turned in. The first-run past-due filter worked (59 archived).
   The 28 live tasks are 20 Blackboard gradebook items and 8 zyBooks items, and both sources carry
   completion (a gradebook attempt or score; zyBooks progress), so submitted work can be filtered
   without a new source.
10. **Effort estimates are far too high.** The 20 Blackboard tasks' `effort_hours` come from the
    cloud task judge (`judge_prompts.ts:108`, low confidence): median 8 h, max 24 h, about 155 h
    in all. The prompt gives no anchors by item type, points or weight.
11. **Late work.** "We should be accounting whether or not late work is worth submitting via
    syllabi": the syllabus pass should read each course's late policy, which decides whether a
    past-due item stays live.
12. **Inbox not read.** No vault note came from mail. M365: the backfill is held (item 3). Gmail:
    prod holds an active grant with the read scope, but `gmail-read` answered 500 "read failed" and
    the Google calendar read 502. Prod's function `GOOGLE_CLIENT_ID` is the sign-in (Auth) client,
    not the Gmail-project client staging's functions use, so the likely cause is that client's
    project lacks the Gmail and Calendar APIs.
13. **Updater (0.1.3 → 0.1.4, 2026-10-23).** The update installed and the app reopened on 0.1.4, but
    Settings' "Check for updates" did nothing visible; the banner appeared only after an app
    restart, and "Update now" took two presses. Same family as "sync now": a button that gives no
    feedback.
