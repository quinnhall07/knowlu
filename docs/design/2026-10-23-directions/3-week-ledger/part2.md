
## 2. Onboarding layout

Window 1280x800. **Two panes from the school step on.**

- **Left pane, the step** (about 560px, a 60-70 character measure): step name, the benefit line, the
  one ask, then the proof. Below the ask, one quiet line of privacy where it matters. At the foot,
  the way forward: the primary button names the next thing ("Continue to payment"), and for a
  skippable step a quiet "Skip" with its cost written beside it, never hidden.
- **Right pane, the ledger** (about 600px): titled "Your week · October 19-25". It is real data or
  plainly pending; it never shows placeholder tasks (notion.md §1.2, "does not carry over").
- **Where you are**: a single line above the step, in words: "Next: payment, then 5 more steps.
  About 7 minutes." The minutes are an honest estimate from the steps left (spec's per-step times).
  No dot stepper, no step numbers.
- Steps 1-2 (welcome, account) are one centred column: there is nothing to put in a ledger yet. At
  the school step the window splits and the ledger slides in from the right with its header only.
- Below 1100px wide the ledger collapses to a docked strip at the bottom (total lines only: "Work
  coming in 22 · estimating", "Open this week ···"), expandable.

---

## 3. Flow map

Each screen has an id. "→" is a transition, with its trigger. Background reads start the moment an
ask is answered and never block the next step (spec §1.5).

### 3.0 Launch and resume

| Id | Screen | Transition |
|---|---|---|
| `R0` | Launch router (no UI; reads `onboarding.json`) | no file → `O1`. File with a current step → that step, ledger rebuilt from the vault and finished jobs (`R1`). Onboarding finished → `M1` |
| `R1` | Resume banner on the saved step | shown once at the top of the left pane; any action dismisses it. Unfinished background jobs re-run silently; their lines show "reading" again |
| `R1-s` | Resume when the school session expired | the step opens normally; the ledger's school lines stay posted; if a later read needs the school, the school line goes to "needs you" with "Sign in to Halverson State again" (opens `O3b`) |
| `R1-p` | Resume at payment | entitlement re-checked first; already paid → advance to `O5` with the ledger's last line "Your trial is on" |

### 3.1 Steps

| Id | Screen | In | Out (trigger) |
|---|---|---|---|
| `O1` | Welcome | first launch | "Start" → `O2` |
| `O2` | Your account | `O1` | Google returns, or the emailed code is accepted → `O2p` (proof inline) → "Continue to your school" → `O3`. Code wrong → `O2-e` inline |
| `O3` | Your school: pick | `O2` | school picked (prefilled from the email domain where it matches) → `O3a`. "My school isn't listed" → `O3f` |
| `O3a` | What you'll need | `O3` | "Sign in at Halverson State" → `O3b` |
| `O3b` | School window open (Halverson State's own page, its host shown in the window title bar: `login.halverson.example`) | `O3a`, `R1-s` | session established → `O3c`. Window closed with no session → `O3-x` |
| `O3c` | Signed in; reading | `O3b` | immediate. Ledger lines appear one by one as "reading". Course list posts first (seconds) → proof line fills → "Continue to payment" enabled at once (student may go on while lines settle) → `O4` |
| `O3-x` | Sign-in didn't finish (window closed, Duo not approved, timed out) | `O3b` | "Sign in again" → `O3b`. Second failure in a row also shows "Use your Blackboard calendar link instead" → `O3f`. "Finish later" → app may quit; resume returns to `O3a` |
| `O3f` | Calendar-link fallback | `O3`, `O3-x` | link pasted and read → `O3c` with fewer lines (the costs post as pending lines) → `O4` |
| `O4` | Payment | `O3c` | "Start 7 days free" opens Stripe Checkout in the browser → `O4w` |
| `O4w` | Finishing in your browser | `O4` | Checkout succeeds → `O4p` → "Continue" → `O5`. Browser tab closed or checkout cancelled → `O4-x`. Tester → `O4-t` instead of `O4` |
| `O4-x` | Checkout didn't finish | `O4w` | "Open checkout again" → `O4w` |
| `O4-t` | Tester: nothing to pay | `O3c` | "Continue" → `O5` |
| `O5` | School email | `O4p` | covered by the school sign-in: forwarding set up and canary sent automatically → proof posts → "Continue" → `O6`. Not covered: one sign-in (same pattern as `O3a-O3b`). "Skip" → pending line → `O6` |
| `O6` | Google: Gmail and Calendar | `O5` | "Continue to Google" → Google warning screen (unverified app) → consent → `O6p`. Partial grant → `O6-g`. Declined or read failed → `O6f`. "Skip" → pending lines → `O8` |
| `O6-g` | Partial grant ("Gmail connected · Calendar not shared") | `O6` | "Share calendar" → consent again for that scope → `O6p`. "Continue without it" → `O7` |
| `O6f` | Gmail by forwarding (fallback only) | `O6` | forwarding verified → `O6p` (Gmail line). "Skip" → pending |
| `O7` | Another calendar (only when no calendar was granted) | `O6-g`, `O6f`, Google skipped | iCal link read → proof → `O8`. "Skip" → pending line → `O8` |
| `O8` | Homework sites (only sites the syllabi or Blackboard mention: zyBooks, VHL) | `O6p`/`O7` | per site: credentials panel → sign-in checked → that site's proof. Wrong credentials → `O8-x` on that site's row. "Skip VHL" → pending line. Both settled → "Continue" → `O9` |
| `O8-x` | That site didn't accept the sign-in | `O8` | "Try again" or "Skip VHL for now" |
| `O9` | Syllabi (only for courses with none found: CS 110) | `O8` | file uploaded → "reading" → proof. "Skip CS 110" → pending line. → `O10` |
| `O10` | Your week: the reconcile | `O9` | every row confirmed, edited, or left blank → "This is my week" → `O11` |
| `O11` | The wait (0 s to 3 min) | `O10` | every first-run job done → `O12`. 3 minutes pass → `O12` with "still running" lines |
| `O12` | Here's your week: the statement | `O11` | "Open Knowlu" → `M1`. Scheduler and start-with-Windows turn on here |

### 3.2 Paths named on the map

- **Duo MFA.** Happens inside `O3b`, on the school's page. Knowlu prepares the student at `O3a`
  ("Have your phone nearby"), waits without a timer while the window is open, and treats a closed or
  timed-out window as `O3-x`, never as an error that blames the student (onboarding-evidence §4.1:
  4.35% of UCI attempts failed at the Duo step). The kept school profile means Duo is asked **once**.
- **Failed school login.** Knowlu never sees the password, so it cannot say "wrong password". It sees
  only "the window closed and no session exists", and says exactly that.
- **Skip paths.** Each optional step's Skip writes a pending line under the perforation saying what it
  would have added. The reveal repeats each skip once, with Connect. Nothing skipped is nudged later
  (spec §1.7).
- **Slow first run.** `O11` waits only for first-run jobs, never backfills, and caps at 3 minutes.
- **Resume after quit.** `R0` → the same step, finished proof intact, ledger rebuilt. A student who
  quit at payment returns to payment (`R1-p`).
