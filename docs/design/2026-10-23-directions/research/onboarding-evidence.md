# Onboarding evidence: long setup, logins, permissions, activation

Research for the five UI directions (2026-10-23). It covers Knowlu's ruled onboarding (spec
`docs/specs/2026-10-23-onboarding-redesign-draft.md`): one long flow in this order: account, school
sign-in, proof, payment, Gmail by Google permission, calendar, homework sites, syllabi confirmed, then
the "Here's your week" reveal.

**How to read this.** Each entry gives the claim, how strong the evidence is, the source, and what it
means for Knowlu's design. Strength scale:
- **A**: peer-reviewed experiment or field study, or a large measured dataset.
- **B**: platform-owner guidance or a published vendor funnel result. These are real data, but the
  method isn't published.
- **C**: practitioner case study, teardown, or a figure reported second-hand.

Where only a secondary source carried a figure, the entry says so. The founder's premise that "longer
onboarding converts better" is weighed honestly in §3. The evidence supports long flows only when each
step pays for itself in the moment.

---

## 1. Motivation and progress (academic)

### 1.1 Endowed progress effect
- **Claim.** People given artificial head-start progress toward a goal are more likely to finish, and
  they finish faster. In the study, an 8-step card was relabelled as a 10-step card with 2 steps
  already done. The effect depends on *perceived* progress. It is moderated by whether a reason is
  given for the head start, and by the "currency" progress is counted in.
- **Strength.** A (Nunes & Drèze, *J. Consumer Research* 32(4), 2006, field experiment, car-wash
  loyalty cards).
- **Source.** https://ideas.repec.org/a/oup/jconrs/v32y2006i4p504-512.html (DOI 10.1086/500480)
- **Implication.** The progress rail should open with real, explained progress already counted: "Account
  created" and "Knowlu installed" are genuine completed steps, so it starts at 2 of 9 instead of 0 of
  7. The head start must have a stated reason. A fake one is a dark pattern and breaks the
  plain-spoken voice. Count progress in the student's currency ("sources connected", "classes found"),
  not in screens.

### 1.2 Goal-gradient (illusory progress, acceleration near the end)
- **Claim.** Effort rises as people near a reward. A 12-stamp card with 2 bonus stamps was completed
  faster than a 10-stamp card. Customers who accelerated toward the reward were retained better.
- **Strength.** A (Kivetz, Urminsky & Zheng, *J. Marketing Research* 43(1), 2006, field data from a
  café loyalty programme and an online rating site).
- **Source.** https://business.columbia.edu/faculty/research/goal-gradient-hypothesis-resurrected-purchase-acceleration-illusionary-goal
- **Implication.** Put the most demanding optional steps (homework sites, syllabus confirmation) *after*
  the student has seen a lot of progress, not before it. Show the remaining distance to the reveal
  concretely ("2 steps until your week"). The reveal is the reward, so name it from step 1.

### 1.3 IKEA effect (labor leads to love, but only when completed)
- **Claim.** People value things more when they helped build them. Their own amateur creations rank
  near experts'. The boost **disappears if the work is not completed or is destroyed**.
- **Strength.** A (Norton, Mochon & Ariely, *J. Consumer Psychology* 22(3), 2012, four experiments).
- **Source.** https://dash.harvard.edu/handle/1/12136084 (DOI 10.1016/j.jcps.2011.08.002)
- **Implication.** Confirming syllabi and class times is the student's "assembly". It makes the week
  feel like *theirs*. But the effect only pays if the work lands. Never let a confirmation be lost
  (resume after quit), and show each confirmed item in the reveal ("the 4 syllabi you checked"). Keep
  the labor to confirming, never typing (ruled: class times are never typed).

### 1.4 Labor illusion / operational transparency
- **Claim.** When a service shows the work it is doing, people value it more and will tolerate a
  longer wait. In the study, users preferred a slower search that visibly worked through sources over
  an instant one with identical results.
- **Strength.** A (Buell & Norton, *Management Science* 57(9), 2011). Replicated in field settings by
  Buell, Kim & Tsay, *Management Science* 63(6), 2017.
- **Source.** https://ideas.repec.org/p/hbs/wpaper/14-034.html (working paper of the 2011 article);
  https://pubsonline.informs.org/doi/10.1287/mnsc.2015.2411 (2017)
- **Implication.** The "Here's your week" wait (up to 3 min) is an asset if it narrates real work:
  "Reading CHEM 1210 syllabus… found late policy", "Checking 23 assignments against what you turned
  in…". It must be **true, live engine progress**, never a fake ticker. The "proof after each login"
  line ("found 5 classes, 23 assignments, 9 already turned in") is the same mechanism at a smaller
  scale.

### 1.5 Resuming interrupted tasks (Ovsiankina, not Zeigarnik)
- **Claim.** A 2025 meta-analysis of 59 publications found people reliably tend to **resume** an
  interrupted task (the Ovsiankina effect). It found no reliable memory advantage for unfinished
  tasks (the Zeigarnik effect).
- **Strength.** A for resumption, weak for the memory claim (Ghibellini & Meier, *Humanities & Social
  Sciences Communications* 12, 2025).
- **Source.** https://ideas.repec.org/a/pal/palcom/v12y2025i1d10.1057_s41599-025-05000-w.html
- **Implication.** Resume after quit is ruled, and this supports it. The return screen should pick up
  the open loop explicitly ("You're on step 6 of 9: Calendar. Your school and Gmail are already
  connected") rather than restart or summarise. Do not rely on the student *remembering* to come back
  (see the Plaid reminder entry, 2.4).

---

## 2. Logins, permissions, and consent (platform owners)

### 2.1 Prime before the system prompt, with one action
- **Claim.** Apple allows a custom screen before the system permission alert. That screen should
  explain the value, offer a **single action that opens the system alert**, and label it "Continue"
  rather than "Allow", because "Allow" implies permission is already granted. Apple also warns that
  people may read a pre-alert as a delaying tactic. Requests should come in context, when the feature
  is needed.
- **Strength.** B (Apple HIG, Privacy). The exact pre-alert wording was quoted second-hand from the HIG
  section "Displaying custom messaging before the alert", so verify the current text before quoting
  it in product.
- **Source.** https://developer.apple.com/design/human-interface-guidelines/privacy
- **Implication.** Each login step should be built as **benefit line → one primary button that opens
  the real sign-in window → proof**. The button names the action ("Sign in at Riverside State"), not
  "Allow". A secondary "Skip, and lose X" stays visually quiet but present (2.2).

### 2.2 State the data, state the benefit, let people decline, degrade gracefully
- **Claim.** Android's rationale UI should say *what data* is accessed and *what the user gains*. It
  must always offer a cancel option, and a denial should disable only the dependent feature, not the
  app.
- **Strength.** B (Android Developers, "Request runtime permissions").
- **Source.** https://developer.android.com/training/permissions/requesting
- **Implication.** This matches the ruled "optional sources skippable with the cost named". Every skip
  names exactly what disappears ("Without zyBooks, Knowlu can't see participation activities; you'll
  see due dates only from Blackboard"). A skip should never block the reveal.

### 2.3 Unexpected requests get denied
- **Claim.** In-context surveys on Android runtime prompts found 84% of requests accepted overall. The
  main reason for denying was that the request **did not match what the user expected the app to
  need**. People also denied because they assumed they could change it later.
- **Strength.** A (Bonné et al., Google research, in-situ survey, 2017).
- **Source.** https://research.google/pubs/pub46261/
- **Implication.** Order matters. School first is the expected ask for a "what's next for my classes"
  app. Gmail and calendar are less obvious, so their benefit line must connect them to classes
  ("Professors email changes; your calendar has your shifts"). Say plainly that every source can be
  disconnected later in Settings, because that lowers the stakes.

### 2.4 Plaid Link: expectation-setting is the lever for credential steps
- **Claim.** Plaid says "Link conversion is highest when users have the right expectations set going
  into Link". Pre-Link messaging should cover why Plaid is used, what the user gains, how data is
  secured, and **what the user will need to provide**. Plaid also advises keeping copy brief, saying
  that accounts can be unlinked later, using an on-brand, polished flow, pre-highlighting the user's
  institution when it is known, and sending reminders to people who didn't finish. Returning users
  who opted in convert **about 11% higher**. Merging consent and phone-number screens produced a
  statistically significant lift.
- **Strength.** B (Plaid's own docs and blog, from a very large funnel, but the method isn't
  published).
- **Source.** https://plaid.com/docs/link/best-practices ; https://plaid.com/docs/link/messaging/ ;
  https://plaid.com/blog/ten-years-plaid-link/ ; https://plaid.com/docs/link/returning-user/
- **Implication.** This is the closest analogue to school sign-in.
  1. Before the school window opens, say **what they'll need**: "your school email and password, and
     your phone for Duo".
  2. Pre-select the school from the email domain, as Plaid's institution shortcut does.
  3. State the security model in one line: the password goes to the school's own page and Knowlu
     never keeps it.
  4. Collapse adjacent consent screens wherever one decision covers both.
  5. Make sure an abandoned onboarding can be resumed and re-prompted.

### 2.5 Google granular consent: one consent screen can be partially granted
- **Claim.** When an app asks for more than one non-sign-in scope, Google's consent screen shows
  **per-scope checkboxes**. Apps "can't assume users grant all requested scopes". They must check the
  scopes actually granted, disable only the dependent features, explain each permission first, and
  ideally request scopes incrementally, in context.
- **Strength.** B (Google Identity docs; this is mandatory platform behaviour).
- **Source.** https://developers.google.com/identity/protocols/oauth2/resources/granular-permissions
- **Implication.** Quinn ruled one Google client and one consent (spec Q8). The UI must still
  handle **Gmail granted, Calendar unticked** (and the reverse). The proof screen after Google must
  report each scope separately ("Gmail connected · Calendar not shared: [Share calendar]"). The
  benefit line before the consent should name both scopes, so the checkboxes aren't a surprise.

### 2.6 Single sign-on vs handing over a password
- **Claim.** University acceptable-use policies broadly forbid sharing a NetID password (Georgetown,
  UIC, UW, SUNY Cortland). UW-Madison distinguishes this from signing in to outside services *through
  the university's own sign-in*, which is legitimate.
- **Strength.** B (primary policy texts; a sample of schools, not a survey).
- **Source.** https://security.georgetown.edu/aup-primer-2/ ; https://ciso.uw.edu/it-at-the-uw/it-governance-and-policies/appropriate-use ;
  https://kb.wisc.edu/itpolicy/page.php?id=58834
- **Implication.** This is a real worry for a careful student: "am I breaking school rules?". Knowlu's
  school step opens the **school's own sign-in page** in a window and keeps no password (spec §2,
  ruling C). The design should show this visibly: show the school's real URL or host in the window
  chrome, never put a Knowlu-styled password field on a school credential, and use the line "You sign
  in on Riverside State's own page. Knowlu never sees or keeps your password." Homework sites
  (zyBooks, VHL) use stored credentials in Credential Manager. Their copy must say "stored only on
  this computer" and must not over-claim the SSO property for them.

---

## 3. Long onboarding: when length helps (practitioner and NN/g)

### 3.1 Static tutorials hurt; setup that does work is different
- **Claim.** NN/g ran a 70-person test on 4 apps. People who read a deck-of-cards tutorial rated tasks
  **harder** (SEQ 4.92 vs 5.49, p=0.047). They were no more successful (91% vs 94%) and no faster.
  NN/g advises spending the effort on the interface, and preferring contextual help when guidance is
  needed.
- **Strength.** A− (NN/g quantitative usability study, small n, simple apps only).
- **Source.** https://www.nngroup.com/articles/mobile-tutorials/
- **Implication.** The long onboarding must contain **no explanatory slides about the product**. Every
  screen either connects something, confirms something, or shows a result. The length is justified by
  logins that must happen anyway, not by teaching. Teach "how to use Today" on Today itself, in
  context, after the reveal.

### 3.2 Wizards: right pattern for infrequent setup, with a visible map
- **Claim.** Wizards suit novices and infrequent tasks such as setup. Show all steps with the current
  one highlighted and clear labels. Use descriptive next and back labels, not "Next". Make each step
  self-sufficient. Let users exit, save and **resume later**. Put help *beside* the flow, not over it.
- **Strength.** B (NN/g guideline article, synthesis of research).
- **Source.** https://www.nngroup.com/articles/wizards/
- **Implication.** Use a persistent, labelled step map in the student's words (School · Payment · Gmail
  · Calendar · Homework sites · Syllabi · Your week). Primary buttons should name the next thing
  ("Continue to payment"). The "why" lives in a side panel or under a disclosure, never in a modal over
  the sign-in window.

### 3.3 Fields matter more than steps (Baymard)
- **Claim.** In checkout, total form fields hurt performance more than step count. Usability degrades
  noticeably past about 8 steps. In 2024 the average checkout had 5.1 steps and 11.3 fields, and 17%
  of shoppers abandoned over complexity.
- **Strength.** A− (Baymard large-scale usability testing and a quantitative survey; the full
  guidelines are paywalled).
- **Source.** https://baymard.com/blog/checkout-flow-average-form-fields
- **Implication.** A long onboarding is affordable **if each step carries almost no typing**. The ruled
  flow fits that: almost every step is one button plus an external sign-in. Payment is the only real
  form, so use the payment provider's hosted, prefilled fields. Nine steps sits near Baymard's
  degradation point, so each must earn its place with a proof line. Combine steps where one decision
  covers two.

### 3.4 Perceived security of sensitive fields
- **Claim.** Shoppers judge security by visual cues, such as an enclosing border or background,
  reassuring microcopy, and a badge, even when every field is equally encrypted. The reinforcement
  works only if it is **distinct** from the styling of ordinary fields. Trust concerns drive about 19
  to 25% of checkout abandonments, depending on the year's edition.
- **Strength.** B+ (Baymard testing; the abandonment share varies by year).
- **Source.** https://baymard.com/research-articles/perceived-security-of-payment-form ;
  https://baymard.com/research-articles/visually-reinforce-sensitive-fields
- **Implication.** Give the credential moments (school window launch, homework-site credentials,
  payment) one consistent, distinct "secure step" treatment: an enclosed panel, a one-line plain
  statement of where the secret goes, and no decorative badges that mean nothing. Reserve that
  treatment for these steps so it keeps its meaning.

### 3.5 Trust signals: design quality and up-front disclosure
- **Claim.** Nielsen's four trust factors still held in a 2016 NN/g study: design quality, up-front
  disclosure, current content, and connection to the wider web.
- **Strength.** B (NN/g qualitative research).
- **Source.** https://www.nngroup.com/articles/trustworthy-design/
- **Implication.** Disclose the price and the "no free tier" **before** the school login, at the
  account step, even though payment comes after the proof. A $9.99 surprise after a student handed
  over their school sign-in reads as a bait and switch. Show the proof first and the charge second
  (ruled), but the price is never hidden.

### 3.6 Value before commitment (Duolingo)
- **Claim.** Duolingo moved sign-up after the first lesson. A Duolingo leader attributed about a 20%
  DAU increase to "simply moving the sign-up screen back a few steps".
- **Strength.** C (a single executive quote, relayed by Taplytics and Growth.Design summaries; the
  method is unpublished).
- **Source.** https://taplytics.com/blog/duolingo-ab-test-onboarding ; Growth.Design, "Duolingo's
  User Retention: 8 Tactics" (https://growth.design/case-studies)
- **Implication.** This supports the ruled order **school sign-in → proof → payment**. The student sees
  their own classes and turned-in work before paying. An account must come first, because the vault
  and the kept profile need an owner, but keep that step minimal (Google or an emailed code, with no
  password).

### 3.7 Long quiz funnels (Noom, Headspace), and the honest version of "longer converts"
- **Claim.** Noom's onboarding runs to roughly 100 screens. Teardowns credit it with three things:
  explaining *why* on the same screen as each sensitive question, an early progress indicator, and
  interleaved proof (stats, stories). Headspace reportedly gained about 10% trial conversion by
  letting users pick several goals. No primary conversion data is published for either.
- **Strength.** C (RevenueCat and Paddle teardowns, a second-hand Headspace figure).
- **Source.** https://www.revenuecat.com/blog/growth/why-your-onboarding-experience-might-be-too-short ;
  https://www.paddle.com/studios/shows/fix-that-funnel/noom ; https://tearthemdown.substack.com/p/headspace
- **Implication.** The evidence for "longer converts" comes from flows where each screen
  **personalises the outcome and shows it** (investment, then reflected value). It does not support
  length as such. For Knowlu, the personalisation *is* the data: each step's proof line is the
  equivalent of Noom's "here's what we learned about you". Without a proof line, a step is pure
  friction.

### 3.8 Activation: setup → aha → habit, and the metric to watch
- **Claim.** Reforge frames activation as three moments: setup, aha (first real value) and habit. Most
  onboarding designs only for setup. Lenny's survey of about 500 products gives a median activation
  rate of 25% (average 34%). A good activation milestone predicts at least 2x retention.
- **Strength.** B− (a practitioner framework; Lenny's survey is self-reported).
- **Source.** https://www.lennysnewsletter.com/p/what-is-a-good-activation-rate ; Reforge framework as
  summarised at https://echai.ventures/glossary/aha-moment
- **Implication.**
  - **Setup** is the onboarding.
  - **Aha** is the reveal, when "what's next" is right and turned-in work is already gone.
  - **Habit** is the student opening Today the next day. Design the reveal to hand off to habit:
    end with tomorrow's first item and when Knowlu will refresh ("Updated again at 12:00").
  - The candidate activation metric is "opened Today on day 2 after the reveal".

### 3.9 Waiting: percent-done past 10 seconds
- **Claim.** Use a looped animation for waits of 2 to 10 seconds and a percent-done or step-count
  indicator for waits of 10 seconds or more. Avoid static "Loading…". People shown a moving indicator
  were willing to wait about 3x longer than people shown nothing.
- **Strength.** B (NN/g, citing a University of Nebraska–Lincoln study).
- **Source.** https://www.nngroup.com/articles/progress-indicators/
- **Implication.** The reveal's wait of up to 3 minutes needs a **countable** indicator. Use sources
  done out of sources total, with per-source lines (labor illusion, 1.4). Don't use a spinner. When
  the time cap is hit, name what is still running (ruled) and let the week show without it.

---

## 4. The failure side

### 4.1 Duo / MFA friction at school sign-in
- **Claim.** At UC Irvine, logs from 2,559 users over 2024–25 showed each Duo push adds about 8
  seconds, and 4.35% of authentication attempts failed because the Duo step was not completed. 43.9%
  of surveyed users had at least one Duo failure. At BYU (4,275 respondents), most found Duo easy and
  secure, but about half had been locked out at least once, and half would prefer the school not use
  it. Students and faculty felt more negative about Duo than staff did.
- **Strength.** A (UCI: NDSS 2026 paper, real logs) and A− (BYU: large survey).
- **Source.** https://arxiv.org/abs/2602.01489 ; https://par.nsf.gov/biblio/10110669
- **Implication.** The school step will fail for some students on the first try for reasons outside
  Knowlu's control: a phone that isn't nearby, a push that timed out, a lockout. Design for it:
  1. Before opening the window, say "Have your phone ready for Duo".
  2. Detect a closed or timed-out window and offer "Try again". Do not show an error state that blames
     the student.
  3. Allow "Finish this later". The step resumes and the rest of onboarding can't proceed without
     school, so say so plainly.
  4. Never ask for MFA again during onboarding. The ruled single kept school profile exists for this.

### 4.2 Students' privacy worries about their own data
- **Claim.** In EDUCAUSE's 2022 survey of 820 undergraduates, only 46% trusted their institution's
  ethical use of data and 54% trusted its ability to safeguard it. Over a third were very or extremely
  concerned about **ed-tech companies using student data for profit**. In 2020, only 22% said they
  understood how their institution used their data. A Future of Privacy Forum survey of about 1,500
  students found 71% wanted the right to control how their data is used.
- **Strength.** A− (large surveys; wording differs between studies).
- **Source.** https://www.educause.edu/ecar/research-publications/2022/student-data-privacy-and-security-a-call-for-transparent-practices/introduction-and-key-findings ;
  https://www.edtechmagazine.com/higher/article/2021/10/educause-2021-what-do-college-students-really-think-data-privacy
- **Implication.** "Knowlu makes money from your data" is the default suspicion to defuse, and $9.99
  with no free tier is the answer: "You pay us, so we don't sell your data". Give it one line at the
  payment step. The vault-on-your-machine fact ("your schoolwork stays as plain files on this
  computer") is a differentiator, so show it during onboarding with a "Show me the folder" link,
  not just in a privacy page. State what goes to the cloud ("task text, to judge effort; never your
  passwords") so "think in the cloud" is disclosed, not hidden.

### 4.3 Credential-sharing distrust (the aggregator analogue)
- **Claim.** In fintech, a PYMNTS consumer survey found 58% of people with a connected third-party
  app were "uncomfortable" sharing bank credentials, and 42% believed it raised fraud risk. A
  regulator submission notes people adopt these tools more readily **when a trusted party recommends
  them**.
- **Strength.** C (an industry survey whose method wasn't checked; stakeholder submissions).
- **Source.** https://pymnts.com/?p=1188525 ; https://treasury.gov.au/sites/default/files/2024-04/c2023-436961-mfaa.pdf
- **Implication.** Where a friend referred the student (the friends MVP), name them on the welcome
  screen. Keep the security statement to one specific sentence per credential step rather than a wall
  of reassurance, because over-reassurance reads as nervous.

### 4.4 Partial grants and skips as a normal state
- **Claim.** About 1 in 6 Android permission requests are denied (10 to 23% depending on type).
  Google's granular consent makes partial Gmail or Calendar grants routine.
- **Strength.** A / B (see 2.3 and 2.5).
- **Implication.** The reveal and Today must look *complete* with some sources missing. Show a quiet
  "Not connected: Calendar · Connect" line, never a broken widget or a red badge.

---

## 5. Notes on evidence quality
- The strongest evidence (A) supports the *mechanisms*: endowed progress, goal-gradient, the IKEA
  effect requiring completion, labor illusion, resumption, Duo failure rates, and student privacy
  attitudes.
- Platform guidance (Apple, Android, Google, Plaid) is B. It is authoritative about practice but
  rarely publishes numbers. Plaid's +11% for returning users and the "expectations" claim are the
  most directly transferable funnel facts.
- The "long onboarding converts" case studies (Noom, Headspace, Duolingo) are C. Use them for
  *patterns*, not as proof that length itself converts. The NN/g tutorial study is direct evidence
  that length spent on *explaining* hurts.
- Figures I could not verify against the primary source are marked as second-hand above. Re-check
  them before quoting any in product copy or a pitch.
