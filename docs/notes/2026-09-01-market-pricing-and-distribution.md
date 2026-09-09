# quinn-ops — Market, Pricing & Distribution

**Provenance:** written by Quinn in a separate session, September 2026; filed here verbatim on
2026-09-01 because nothing about this project may live only in a chat transcript. It is the
companion to `2026-09-01-product-and-business-plan.md` and **supersedes parts of it** — its §0 is
the authoritative list of what is now obsolete.

**Reading order for the two documents:** this one's §0 first, then the other document's §12
(the rulings Quinn gave on handing it over), then either body. Where the three conflict, the
precedence is **§12 rulings > this document > the first document's body**, because that is the
order they were written in.

**What was checked on ingestion is in §12 at the end of this file**, including three places where
this document recommends something the repo already does, one factual refinement from operating
experience, and the architectural obligations the new pricing model creates.

*Companion to the Technical & Business Planning Document. September 2026.*

**This document supersedes parts of the first one.** See §0 for what's now obsolete.

---

## 0. What this supersedes

| First doc | Status | Replaced by |
|---|---|---|
| §9.3 — $4.99/mo, $39.99/yr, no free tier | **Obsolete** | §7: free tier + $9.99/mo + $69.99/academic year |
| §9.2 — "paid acquisition doesn't work at $4.99" | Still true, reasoning strengthened | §5 |
| §10.2 — pilot adoption 0.1–5% of campus | **Too pessimistic as a ceiling** | §3: Coursicle reaches 80–90% at some campuses |
| §10.3 — Year 1 projections | **Recalculate** at new price | §9 |
| §3.2 — three-tier scraper escalation | Still correct, but tier 1 is bigger than assumed | §4: ICS feeds handle the primary LMS case |
| §2.4 / §3 — Blackboard REST as the UA connector | **Deprioritized** | §4: ICS first, REST only if needed |
| §11 — open decisions | Several now closed | §11 |

Everything else in the first document stands.

---

## 1. Competitive landscape

### 1.1 Three clusters

**Cluster 1 — Auto-filling student planners (direct competition).** Shovel, DormWay, Coursicle, Wick, IntelliPlan. Converged on an identical pitch: LMS sync plus AI syllabus parsing, so the app fills itself in.

**Cluster 2 — Manual student planners.** MyStudyLife, myHomework, Power Planner. Large install bases, cheap, but require typing everything in.

**Cluster 3 — General AI schedulers.** Motion ($19–29/seat/mo annual), Reclaim (free–$18), Sunsama ($20), Akiflow ($34). Priced for professionals, no LMS awareness. Motion has no email awareness, no triage, and users report feature bloat as it expanded into a "SuperApp."

### 1.2 Source caveat

Much of the findable comparison content is competitor-written. DormWay and Coursicle both run "honest picks" blog posts that rank for these searches and conclude with themselves. They concede real weaknesses, which makes them useful, but they are SEO plays. Verify feature claims independently.

### 1.3 The two findings that matter most

**Your coursework layer is table stakes, not a differentiator.** "LMS sync + AI reads your syllabus" is the baseline claim of at least five products.

**The category is commoditized to free.** DormWay, Coursicle, Wick, and IntelliPlan are all free for students. Shovel is the notable paid exception (~$9.79/mo or $35/yr; some sources report tiers near $20/mo) with no free planner, only a trial.

Charging for coursework tracking means beating free on quality. Charging for what nobody else does is a far easier fight.

### 1.4 Where quinn-ops is actually unoccupied

Every competitor found is **coursework-only**. None ingest email for obligations. None surface opportunities/events with a preference model learned from accept/reject decisions. None track contact cadence or "you're waiting on a reply." None touch client work or content commitments. None are local-first. None have an approvals-vs-autonomy model or back-traceability.

**Strategic read: coursework is the entry ticket and must be at parity, but it cannot be the pitch.**

---

## 2. Competitor profiles

### 2.1 Coursicle — the closest analogue

**Origin.** Joe Puccio, incoming UNC Chapel Hill freshman, summer 2012. Spent ~7 hours planning his schedule, got into 1 of 5 classes. Wrote a script to text him when a seat opened. First version literally captured automated screenshots of course pages; once he got into everything, he rearchitected it to notify on change. Friends asked for access. Co-founded with Tara Aida (friend from a 10th-grade science camp, then at Harvard), who built a companion scheduling site.

**Timeline — the most important data in this document:**

| Period | State |
|---|---|
| 2012 | First script, personal use |
| 2012–2015 | **UNC only, free, ~3 years** before requests came from other schools |
| 2015 | Products merge into Coursicle |
| May 2016 | Both founders graduate, go full-time |
| 2016 | Upstart accelerator (StartCo), $43,800 |
| ~2016–17 | ~100 schools, ~19,000 users |
| Today | 900 colleges, 2.6M courses, 284,000 web users |

Roughly **four years from first script to business, six-plus to meaningful scale.** Puccio has described logging ~100 hours/week.

**Funding.** Essentially none. $43,800 accelerator. Otherwise bootstrapped, running entirely on revenue, avoiding VC deliberately as a mission choice.

**University deals: none, by design.** Puccio: *"We collect information from hundreds of schools, we just have to put in the right links. And we can do that without the university's help."* Class data is publicly available and relatively standardized, so no institutional participation is required.

This is the teach-once-then-run-deterministically scraper pattern from the first doc, validated at 900-school scale by a two-person team.

**Backend.** Started on two physical servers in their dorm rooms — which broke down as course catalogs multiplied, and required physically transporting the machines and managing shutdowns over breaks. Moved to DigitalOcean using **GitHub Student Developer Pack** credits, which they cite as a major cost factor as students. Today: 10–15 Droplets plus Snapshots, Backups, Spaces, a Reserved IP, private networking, load balancers. Boring, cheap infrastructure.

**Growth.** *"The majority of our growth over the years has been word of mouth between students."* Two accelerants: they picked up schools as competitor MyEdu wound down (disbanded March 2017), and saw a boom at colleges where **academic advisors** started recommending it after hearing about it from advisees.

**Pricing.** $4.99 **per semester** (~$10–15/year), with one class trackable free. Earlier attempted textbook affiliate commissions. Aida: *"We don't want to charge students for this — being able to do something they should already be able to do."*

**Penetration.** At some colleges, **80–90% of students use Coursicle.** Single-campus saturation is achievable — over years.

**Product notes.** No account required; device-local persistence. Reads six LMS platforms. iOS, Android, and web. Also does class registration and open-seat alerts — an acute, once-a-semester panic with a clear success condition.

### 2.2 Shovel — the institutional path

**Origin: an audience first, not a product.** Jim Siverts ran HowToStudyInCollege.com. Co-founder Petr Placek was a Czech hockey player at Harvard (economics) who struggled with a course; he later worked as an assistant project manager on a NYC skyscraper, coordinating hundreds of workers and material deliveries. **The Cushion is construction critical-path scheduling applied to coursework.** Third co-founder Branko Gvoka, a Rice CS student, built the real-time planner.

Founded 2017, Massachusetts, ~11 employees as of May 2025.

**Distribution strategy: through the institution.** Pursued **LTI certification** to run inside Canvas, Brightspace, Moodle, and Blackboard. Slower, requires school buy-in, but puts you in front of every student by default.

**Product.** Syncs Canvas, Brightspace, Moodle, Google Classroom read-only (~every 24 hours), plus AI syllabus upload with a review screen. The Cushion: available study time minus estimated time needed per task, recomputed continuously, surfaced as an ahead-or-behind indicator.

**The critical weakness.** Reviews consistently cite that Shovel **requires upfront setup to enter every task**, and its mobile app states setup must be done on the web app first because syllabi carry too much information for a phone. **Shovel makes the human supply the time estimates** — the load-bearing input is punted to the user.

That directly violates quinn-ops's "if the user is typing something the system could have found, the system has failed." **This is the clearest product opening in the category** (see §4.5).

Only paid competitor; no free tier.

### 2.3 DormWay

Free for students. Syncs Canvas, Blackboard, Moodle read-only. AI reads uploaded syllabi for assignments, exam dates, **the grading breakdown, and the late policy** — surfacing stakes, not just dates. One "Do Now / Up Next / Due Soon" timeline. Web, iPhone, iPad, Mac; no Android. Positions on an explicit AI boundary: "we use AI for dates, not essays," and never sells student data.

The grading-weight extraction is the input a defensible ranking function needs, and it comes from the syllabus because no API exposes it.

### 2.4 MyStudyLife

Claims 24M students across 197 countries. Two founders; team distributed across the UK, Canada, UAE, Pakistan, Bangladesh, Indonesia. Free with MSL+ subscription (~$4.99/mo, which is where Canvas sync lives). Coverage from NYT and Forbes.

**Not standing still:** has added Scout, an AI study coach, camera-based schedule scanning from a photo of a timetable or syllabus, and iCal sync. The manual incumbents are automating.

### 2.5 The rest and the graveyard

- **Wick** — free, syncs Canvas/Moodle/Blackboard/D2L, reads syllabi, texts reminders.
- **IntelliPlan** — free, Canvas/StudentVue/Schoology, includes an AI tutor.
- **myHomework** ($4.99/yr) and **Power Planner** ($4.99 one-time) — manual, cheap, established. Power Planner is the GPA-projection pick.
- **MyEdu** — disbanded March 2017.
- **StudyBlue** — acquired by Chegg for $20.8M in 2018, shut down end of 2020.

Category churn is high.

---

## 3. Realism assessment

**Encouraging:**
- Bootstrapping works. A two-person student team reached hundreds of schools on $43,800 and no VC.
- No university agreements needed if you stay on public or user-authorized data.
- Infrastructure is cheap and boring — DigitalOcean droplets, student credits.
- Single-campus saturation of 80–90% is proven achievable.

**Sobering:**
- **Timeline is years, not semesters.** Three years at one school before expansion requests arrived.
- **Coursicle's product has a structural advantage yours lacks.** Registration is an acute, universal, once-a-semester panic with a clear success condition. Daily planning is a habit — harder to establish, easier to churn out of.
- Both successful cases had co-founders. You are currently solo.
- Median subscription app earns **$492/month**. Top 10% capture **94.5%** of all subscription revenue. **57.7% of new apps never reach $1,000 in total.**

---

## 4. LMS access — the technical core

### 4.1 The problem

UA runs **Blackboard Ultra** (all ~4,000 courses transitioned by Spring 2026). UK runs **Canvas** (migrated 2016, retired Blackboard). Your pilot campus and first expansion campus are on different platforms from different vendors.

### 4.2 Blackboard REST — real gate, but scalable

Uses OAuth 2.0. A developer registers one application in Anthology's portal for an OAuth key/secret plus an Application ID. Critically: **one Application ID works across all institutions** — the app identifies the institution by FQDN, and there is no need to install a separate ID per Learn instance.

But each institution's admin must register that ID via their Learn Admin REST API Integrations tool before it functions.

Correct configuration would be **three-legged OAuth with End User Access = Yes**: students sign in with their own Learn credentials, and each user's access is limited to their own permissions.

**Net: one registration, then a repeatable per-campus ask.** Slow but not restart-from-scratch.

### 4.3 Canvas personal access tokens — do not use

Students *can* generate them from account settings, which is presumably how some competitors work. Universities are actively shutting this down:

- **Texas A&M**: token access "is not available for students or development purposes"; sharing with third-party vendors creates security risks and may violate acceptable-use procedure; revocation can happen at any time.
- **Cornell**: decommissioned manual token issuance April 2024; instructs students not to provide tokens to any individual, vendor, application, or bot; anyone who has shared one must delete or regenerate immediately.
- **UW**: strongly prefers developer keys and OAuth over direct API access.

**The disqualifying detail:** a Canvas token is equivalent to the student's username and password, and Cornell's stated risk is that a *student's* token can expose FERPA-protected data belonging to **other students** — course enrollment, discussion posts, grades from group assignments.

This sharpens the first doc's FERPA flag into something specific. The "paste your Canvas token" pattern collects other people's protected records through your user, at schools that increasingly prohibit it by policy. **Off the table.**

### 4.4 The ICS feed — the path nobody in the competitive set is taking

**Both Blackboard Ultra and Canvas expose a student-generated ICS calendar feed URL.**

- **Blackboard**: Calendar → Calendar Settings → Share Calendar → copy link (ends in `.ics`). Covers **all** the student's courses; spans one year back and forward.
- **Canvas**: iCal feed link in the Calendar sidebar.

Both are documented, vendor-supported, user-facing features built so students can pipe coursework into Google or Apple Calendar.

**Why this is close to ideal:**

| Property | Effect |
|---|---|
| Zero admin approval | Works at UA on day one without contacting OIT |
| Zero credential handling | User pastes a URL. No keychain, no OAuth, no password-equivalent token |
| Scoped to one student | Cannot reach other students' data — the exact FERPA exposure that kills the token path |
| Same connector for both LMSes | The UA/UK platform split largely dissolves at the ICS layer |
| Plain fetch + parse | Tier-1 case from the first doc's escalation ladder. No browser, no model |

**Limitation:** ICS gives due dates and schedule only — **not** grades, grading weights, submission status, or completion state. So it does not serve mechanic #6 (completion detection) or the GPA-floor piece.

### 4.5 The resulting two-source architecture

**ICS feed for the live due-date stream + syllabus PDF for the stakes and the deadlines that never reach the LMS.**

Neither requires institutional permission. Together they cover most of what's needed without touching a credential. DormWay independently reached the same conclusion on the syllabus half.

**Revised build order:** ICS parser is connector #1, not Blackboard REST. It unblocks UA immediately, works at UK unchanged, avoids credential and FERPA exposure, and defers the UA OIT conversation until there are users and a reason to ask.

### 4.6 The Shovel opening

Shovel's Cushion is mechanically simple — available study time minus estimated time needed, recomputed live. The sophistication is in doing it continuously.

**But Shovel makes the user enter the estimates.** That is the load-bearing input, punted to the human, and it is their biggest friction point.

Derive estimates instead, from: assignment type, credit hours, historical completion time on similar past items, and grading weight (a 15% exam and a 2% homework warrant different budgets). **This is where to point ranking work first.**

---

## 5. Ecosystem & integrations

### 5.1 What earns an integration

**Does this source create an obligation?** Not "do students use it." Spotify, dining menus, campus transit — heavily used, no obligations. Handshake creates obligations. Coursework vendors create obligations. That filter kills most of the wishlist.

### 5.2 Read-only by default

"Approvals, not autonomy" plus "the AI never produces anything sent externally" means you are overwhelmingly consuming, not writing. **"quinn-ops only ever reads"** is a landing-page sentence, and it reduces OAuth scope requests and legal exposure simultaneously. Rare writes (a calendar block on the user's own calendar) stay behind explicit approval.

### 5.3 Where obligations actually live

The LMS is **not** where most deadlines are:

- Instructors post some deadlines to the LMS calendar, some only in the syllabus PDF, some only in an announcement, some only verbally.
- A large share of homework lives in **third-party coursework platforms** — MyLab/Mastering, McGraw Hill Connect, WebAssign, Cengage, ALEKS, zyBooks, Gradescope, Top Hat. These rarely sync deadlines back to the LMS and mostly have no public API. **This is the differentiator; no LMS-only tool catches them.**
- The syllabus PDF at semester start is the densest obligation source in a student's life, and it is a document, not a feed.

### 5.4 Integration tiers

| Tier | Sources |
|---|---|
| **Launch-critical** | LMS via ICS (both campuses), campus calendar via M365/Google Workspace, syllabus PDF ingestion |
| **High value, early** | Coursework vendors (top 3–5 by campus prevalence), SIS (Banner at UA) for schedule/registration dates, personal email |
| **Opportunities** | Handshake, campus engagement platforms (Anthology Engage, CampusGroups, Presence), department/research listservs |
| **People/work** | Google/Outlook contacts, GitHub or Linear, GroupMe or Discord (where club coordination actually happens) |
| **Excluded** | Banking, health portals, social feeds — no obligations, extra regulatory regimes |

### 5.5 Considerations

- **Platform risk is asymmetric.** Canvas/Blackboard can restrict API access; Handshake and LinkedIn are hostile to automated access. **Avoid LinkedIn entirely** given the litigation history.
- **Connector count is a support burden.** Every integration breaks at 2am before a deadline. Ten solid connectors beat forty flaky ones. Let failure-rate-per-source telemetry drive what gets added.
- **Be a good citizen on exit.** ICS export, journal in a user-synced folder. Philosophically consistent and noticed by a privacy-skeptical audience.
- **Onboarding is where breadth becomes liability.** Minimum viable connection set (LMS + calendar) produces a useful morning view immediately; everything else offered later.

---

## 6. Distribution: consumer vs. institutional

### 6.1 Three tiers, not two

A **technical permission is not a vendor relationship.** This distinction was previously collapsed.

| Tier | What it is | Cost |
|---|---|---|
| **1. Consumer** | Word of mouth, ICS feeds, no institutional contact | Free |
| **2. Technical permission** | Ask UA OIT to register your Blackboard Application ID so students who choose your app can connect | Small ask. No money changes hands, no procurement, no site license. Closer to approving a browser extension than buying an LMS |
| **3. Institutional sales** | Site licenses, LTI certification, RFPs | Reshapes the company |

### 6.2 Why tier 3 conflicts structurally

- **FERPA posture flips.** From "student using a personal tool on their own data" to a service provider acting on the school's behalf — contracts, DPAs, breach obligations, possibly audit rights. A one-way door per school.
- **Local-first contradicts procurement.** Schools want SSO, admin visibility, account disabling, data residency answers. Blind-relay encryption means you **cannot** produce a student's data — you would be pitching a security review on your inability to satisfy it.
- **HECVAT and VPAT are real gates.** Long security questionnaires and accessibility conformance documentation. Note UA's own Blackboard Ultra messaging emphasized accessibility compliance with federal guidelines.
- **LTI is a third client.** A web app in an iframe — a new surface, not a port, and you just committed to desktop-only.
- **Sales cycles are 6–18 months**, tied to fiscal years and committee calendars. Consumer growth is product plus word of mouth. One is doable between classes; the other is a job.
- **Channel conflict.** If UA site-licenses it, what happens to UA students already paying you?

### 6.3 The advisor channel — cheapest institutional-adjacent play

Coursicle saw a boom at colleges where **academic advisors** heard about it from advisees and recommended it to other students. Institutional distribution without an institutional relationship. Advisors are a small, reachable, non-procurement population who each talk to hundreds of students, and recommending a tool requires no one's approval.

**At UA you can walk into that conversation.** Highest leverage per unit effort on this list, and fully compatible with the architecture, pricing, and legal posture.

### 6.4 Sequencing — note the asymmetry

Consumer-first leaves the institutional door open. Institutional-first would force a fundamentally different (non-local-first) product, and that cannot be walked back.

1. **Now:** consumer + word of mouth, ICS feeds, no institutional contact.
2. **Now, cheap:** advisor channel. Test during pilot.
3. **When users ask for grades/completion detection:** the Blackboard app ID request. Having real student users makes this much easier to ask.
4. **Only if numbers demand it:** true institutional sales. Decide deliberately with revenue data; do not drift into it.

Steps 1–3 are compatible and should all happen. Step 4 in parallel with step 1 is two full-time jobs for a solo student founder.

---

## 7. Cost structure

### 7.1 Payment processing is now your largest cost — larger than AI

At $4.99 billed monthly, Stripe-style fees take **$0.44, ~8.8% of revenue**. AI is $0.10–0.50. The flat $0.30 per transaction dominates at low price points.

### 7.2 Billing cadence — biggest single lever, costs nothing

| Structure | Annual revenue | Fees | Effective rate |
|---|---|---|---|
| $4.99/month | $59.88 | ~$5.34 | 8.9% |
| $19.99/semester (×2) | $39.98 | ~$1.76 | 4.4% |
| $29.99/academic year | $29.99 | ~$1.17 | 3.9% |

Beyond fees, semester/annual billing matches how students think, lines up with financial aid disbursement, and sidesteps the summer problem where usage drops but monthly billing continues.

### 7.3 You already avoid the biggest tax in the category

Desktop-only, direct download means **no Apple or Google 15–30% cut** — $0.75–1.50 per user per month that mobile-first competitors pay and you don't. **Larger than your entire AI bill.**

### 7.4 Cloudflare R2 for model distribution

Model downloads are the one bandwidth-heavy workload. R2 charges **$0.00/GB egress at any volume**; S3 charges $0.09/GB after the first 100GB free. Storage: $0.015/GB-month vs $0.023.

10,000 users × 1.5GB model = 15TB egress: **~$1,350 on S3, $0 on R2.** R2 implements the S3 API so tooling works unchanged.

Caveat: R2 charges for operations, so it is wrong for workloads with huge numbers of tiny requests. Model distribution is few requests, large files — its best case.

Cloudflare Workers (100k requests/day free; $5/mo for 10M) and Pages suit the manifest endpoint and marketing site.

### 7.5 Credits you qualify for right now

**GitHub Student Developer Pack** includes DigitalOcean credits — Coursicle explicitly cites using exactly this, and still runs on DigitalOcean today. Also check DigitalOcean's startup program and inference-provider startup credits.

### 7.6 Syllabus deduplication — the biggest smart-data lever

Every student in PH 106 section 003 has **the same syllabus**. Parsing 200 identical uploads independently means paying for 200 identical extractions.

**Parse once, cache the derived structured course data** (assignment list, due dates, grading weights, late policy), serve to everyone else in that section.

At a campus with ~4,000 courses and 42,000 students, this collapses extraction volume by orders of magnitude. **Economics improve with density** — a genuine network effect on cost that rewards exactly the campus-concentration strategy.

Guardrails: cache the *derived course data*, never the PDF or anything student-specific. A syllabus is the professor's course document, not a student's record — but make it an opt-in "share course data with classmates" setting anyway. Same shape as extraction-script sharing: derived artifacts travel, personal data doesn't.

### 7.7 Smaller inference optimizations

- **Batch API tiers** typically ~50% cheaper; ingestion is a scheduled background job with no latency requirement.
- **Prompt caching** on repeated system prompts and schemas.
- **Rule-promotion loop** means per-user inference cost *declines* as the ruleset matures. Unit economics improve with age.

### 7.8 Revised cost per active user

**~$0.15–0.60/month**, mostly processing fees, AI minor, infrastructure a rounding error.

---

## 8. Pricing (supersedes first doc §9.3)

### 8.1 Research findings

**Conversion by price point:**
- Median download-to-paid: **1.4% low-priced, 2.0% mid, 2.8% high** (RevenueCat 2026).
- Adapty's cut is starker: **9.8% median for high-priced vs 4.3% for low-priced.**
- Realized LTV: **$62.19/user/year high-priced vs $10.69 low-priced — a 5.8x gap from pricing alone.**
- Framing: a $2.99/mo app feels like a toy, $9.99 feels like a tool, $14.99 feels like a professional solution.

**Paywall model:**
- Hard paywalls: **10.7% D35 trial-to-paid vs 2.1% freemium** (~5x). Revenue per install at D60: **$3.09 vs $0.38** (8x).
- But year-one retention is **27% vs 28%** — statistically negligible. Paywall type affects conversion *speed*, not loyalty.
- Freemium remains correct when free users drive word of mouth. That is your situation.
- Trial models (ChartMogul, 200 products): opt-in 8.9%, opt-out/CC-required 31.4%, freemium 5.6%. Opt-out converts better per signup but cuts signup volume 50–70%.

**Plan duration:**
- **Low-priced apps retain best on annual: 36%, vs 23% high-priced and 26% mid-priced.** Cheap apps are where annual works best. *(This reverses earlier caution about annual commitments.)*
- Annual retention ~44.1% vs ~17% monthly.
- **59% choose annual when offered a 30–40% discount.**
- Counterweight: monthly wins when pre-PMF, trust is low, users want flexibility, or the use case is short-term — which describes an unproven product sold to AI-skeptics.
- Category convention: Productivity draws **77% of revenue from monthly plans.**

**Trials and Day 0:**
- Trials of 17–32 days: **42.5%** conversion; under 4 days: **25.5%.**
- **55% of trial cancellations happen on Day 0. 82% of trial starts happen the same day as install.**
- **30% of annual subscribers cancel in month one; month one accounts for 35% of all annual cancellations.**
- Education-category apps already trend long: 80%+ of trials run 5–9+ days.

**AI penalty (worse than previously noted):**
- AI apps generate **41% more revenue per customer** but **AI monthly plans retain 36% worse over 12 months.** RevenueCat's read: a problem emerging after AI apps hit mainstream and users had longer to assess them. Note the penalty attaches to *monthly* plans — another argument for annual.

**EdTech seasonality and churn:**
- B2C edtech retention averages ~40% annually; ARPU $10–50/month.
- **Q3 (July–Sept) is peak; Q1 is trough with engagement 30–40% below peak.**
- Pause-instead-of-cancel is the standard seasonal remedy. One edtech case study cut churn **25% → 14%** via a personalized cancellation flow, retaining more than half of students who entered it.
- **Notification guilt is real:** one platform sending 14 emails/month found more notifications increased guilt and accelerated churn; cutting to 4/month helped. Learning subscriptions trigger dual shame — financial waste *plus* personal failure.

**Market concentration:**
- Median subscription app: **$492/month.** Top 10% capture **94.5%** of subscription revenue. **57.7% of new apps never reach $1,000 total.**

### 8.2 Why the price goes up

**The free tier absorbed the affordability constraint.** The original $4.99 was driven by "students are broke and AI-skeptical, nobody can be priced out." That is now satisfied by a permanent free tier — no student is priced out, because coursework tracking is free forever.

The paid tier is therefore free to be priced on value rather than accessibility. Two different jobs, previously collapsed into one price.

**Freemium self-selects.** Anyone upgrading has already installed, connected sources, used it a semester, and decided it's worth money — a far warmer buyer than someone hitting a cold paywall. This matches the finding that higher-priced apps convert better because their users are more intent-driven.

**The paid features skew to a specific person.** Email ingestion, opportunity filtering, contact cadence, client work, content cadence — a median sophomore has none of these. **The overcommitted student does:** research assistant, club officer, freelancing, job hunting, three group projects. More desperate *and* higher willingness to pay.

**Comparables for that person are not free student planners.** They are Motion ($19–29), Sunsama ($20), Akiflow ($34), Reclaim (up to $18). B2C edtech ARPU runs $10–50/month. At $4.99 you were priced below the floor of your own category.

**Asymmetry:** lowering a price later is easy; raising one is painful. Grandfather clauses, betrayal feelings, an audience anchored low. Launch higher and discount if needed — "$4.99 for your first semester" works as an acquisition tactic without permanently anchoring.

### 8.3 Recommended structure

| Tier | Price | Purpose |
|---|---|---|
| **Local-only** | Free, permanent | Word-of-mouth engine. Costs ~$0 in inference |
| **Academic year** | $69.99 (Aug–May) | Anchor. ~30% off monthly equivalent (~$7.78/mo), inside the band where most choose annual |
| **Monthly** | $9.99 | Flexibility for low-trust users; covers pre-PMF |

### 8.4 The free/paid boundary

Drawn where cost is *and* where competition is. Coursework is commoditized to free — do not fight that. Charge for the domains nobody else offers.

| | Free (permanent, local) | Paid |
|---|---|---|
| Coursework via ICS + syllabus | ✓ | ✓ |
| Morning view, ranking, capacity | ✓ | ✓ |
| Single device | ✓ | ✓ |
| Multi-device sync | | ✓ |
| Email ingestion | | ✓ |
| Opportunities, people, work, content domains | | ✓ |
| Cloud proposal generation | | ✓ |

Everything free is free **because it costs nothing to run**; everything paid costs real money. That story survives scrutiny.

### 8.5 Three churn mechanics that matter as much as price

1. **Don't bill for summer at all.** Auto-pause June–August. Kills the worst churn cliff, saves inference cost during near-zero usage, and is a differentiating honesty signal. Nobody else does this.
2. **Put pause in the cancellation flow**, not just settings — that's where the 25%→14% result came from.
3. **Front-load value ruthlessly.** Month one drives a third of annual cancellations; Day 0 drives most trial cancellations.

### 8.6 Trial

14–21 day opt-in trial, no credit card. Opt-out converts better per signup but cuts volume 50–70%, which fights the reach goal.

---

## 9. Revised projections

At $9.99/mo or $69.99/academic year, assuming a paid mix averaging ~$7.50/month effective, Stripe fees, and $0.15–0.60/user AI+infra:

**Pilot — University of Alabama (42,360 students; 35,622 undergrad):**

| Scenario | Paid users | Gross/mo |
|---|---|---|
| Weak (0.1%) | ~42 | ~$315 |
| Plausible (0.5%) | ~210 | ~$1,575 |
| Strong (2%) | ~850 | ~$6,375 |
| Exceptional (5%) | ~2,100 | ~$15,750 |

Note these are *paid* users. Free-tier users will substantially exceed these counts, and that is the point — they are the distribution channel.

**Year 1 (UA + UK, ~81,000 combined students):**

| Scenario | Paid users | Gross/mo | Net after fees + AI |
|---|---|---|---|
| Conservative | 1,500 | $11,250 | ~$9,900–10,500 |
| Moderate | 5,000 | $37,500 | ~$33,000–35,000 |
| Optimistic | 15,000 | $112,500 | ~$99,000–105,000 |

**Steady state (~19.6M US undergraduates):**

| Penetration | Paid users | Gross/mo |
|---|---|---|
| 0.05% | ~9,800 | ~$73,500 |
| 0.5% | ~98,000 | ~$735,000 |
| 2% | ~392,000 | ~$2,940,000 |

**All contribution margin, not profit.** Excludes engineering time, support, marketing. Every number is downstream of one figure not yet in hand: the actual organic referral rate.

---

## 10. Onboarding, cross-platform, and experimentation

### 10.1 Three price points, two entitlement states

Free vs. paid is the only distinction any client needs. Academic-year vs. monthly is a *billing cadence*, not a feature difference. **Mobile never needs to understand pricing** — it asks one question: is this account paid?

Three *feature* tiers would create cross-platform complexity. Three ways to pay for the same thing does not.

### 10.2 Sequence the asks; don't stack them

The permission ask ("connect your LMS, calendar, email") is scarier than the money ask for an AI-skeptical audience. Tier selection should not appear in onboarding at all.

1. **Launch:** paste your LMS calendar URL → immediately see your semester. No account, no payment, no permissions. (ICS makes this the lowest-trust-cost first connector in the category.)
2. **When they want a second device or more sources:** create an account.
3. **When they hit a paid capability:** the upgrade.

Additional connectors get offered in context, with a reason attached, rather than as a wall of checkboxes at signup.

### 10.3 Mobile: companion model

Desktop-first is a structural advantage. You will arrive at mobile with users who **already have accounts and already pay**, established on the web at 0% platform commission. The mobile app logs in, reads the journal, and never shows a paywall.

**Build the entitlement-check seam now**, even though nothing needs it yet.

### 10.4 App store litigation state (verify before implementing)

| Date | Event |
|---|---|
| Apr 30, 2025 | Epic ruling bars Apple from commission on external purchase links in US apps; no restriction on link presentation |
| Dec 11, 2025 | Ninth Circuit largely upholds; says Apple may charge a reasonable commission tied to necessary costs |
| Apr 2026 | Ninth Circuit lifts stay; remand proceeds while Apple seeks Supreme Court review |
| Jun 30, 2026 | Supreme Court agrees to review the contempt finding |
| Aug 13, 2026 | Apple proposes 15% standard, 10% subscription renewals, **5% Small Business Program** (the tier you'd fall into) |

Two practical notes: this is **US-only** — the same code is a rejectable violation in most other regions, so any implementation must be region-aware. And treat external-link support as something revised at least once in the next twelve months, not a one-time build.

**Unsettled enough that it should not influence architecture.** The companion-app model works regardless of outcome. All of this is post-cutoff; verify current state before implementing.

### 10.5 A/B testing — the sample size problem

For 80% power at 95% confidence:

| Effect to detect | Users per arm | Total |
|---|---|---|
| 3% → 4.5% (50% lift) | ~2,500 | ~5,000 |
| 3% → 6% (double) | ~750 | ~1,500 |
| 3% → 9% (triple) | ~240 | ~500 |

A plausible UA pilot is 200–850 users. **You can detect an effect that triples conversion and essentially nothing subtler.** Worse for price tests, where only users who *reach the paywall* count toward the sample.

An underpowered test that comes back "significant" is more likely a false positive than a real effect — worse than not testing, because you'll act on it.

### 10.6 The campus-density problem

**Your users all know each other.** Concentrated single-campus adoption is the growth strategy, which makes a concurrent price test a group-chat screenshot waiting to happen. For a product positioned on trustworthiness, that damage outweighs an underpowered test.

For price specifically, don't split concurrently within a campus:

- **Temporal cohorts** — Fall at one price, Spring at another. Confounded by time and product maturity, but the whole user base is in each arm.
- **Campus as arm** — UA vs UK. Confounded (different LMS, different dynamics, your physical presence at one) but knowably so, with comparable populations.

**Grandfather anyone who ever saw a lower price. Permanently.**

### 10.7 What to test, in priority order

1. **Price point** — $4.99 vs $9.99. A doubling, at least in detectable range. Cohort or campus split only.
2. **Time-to-first-value in onboarding** — probably higher leverage than price given Day 0 data, and safe to test concurrently (nobody compares onboarding flows in a group chat).
3. **The free tier boundary** — does email ingestion belong in free or paid? Currently a guess.
4. **Paywall trigger timing** — after first week, after first overcommitment warning, after second source.
5. **Notification cadence** — retention rather than conversion; validates the notification cap empirically.

**Don't test** button colors, copy variants, or anything ten user interviews would settle.

### 10.8 Execution

Direct-download desktop means you control the whole stack — no app store review, no paywall-vendor constraints.

- **Assignment:** deterministic hash of user ID → variant, computed server-side and stored. Same user always gets the same variant across sessions, devices, and reinstalls.
- **Delivery:** client fetches config from the same manifest endpoint already being built for model updates. No separate infrastructure.
- **Measurement:** existing telemetry events tagged with variant.
- **Pricing mechanics:** Stripe supports multiple prices per product; the variant selects the price ID.
- **Tooling:** PostHog (analytics + feature flags) or GrowthBook (flagging + experimentation). Both self-hostable, which fits the privacy positioning better than a SaaS vendor.

### 10.9 Discipline rules

- **Decide sample size and end date before starting.** Don't peek and stop when it looks good — the most common way small teams fool themselves.
- **One test at a time.** At your N you cannot split traffic further, and interacting tests are uninterpretable.
- **Write the hypothesis and mechanism first.** If you can't state a mechanism, you're fishing.
- **Prefer instrumenting over testing early.** Telemetry tells you *where* the problem is, which is worth more at 300 users than any single test result.

**Recommendation: treat the pilot as instrumentation-and-interviews. Save real A/B testing for past ~1,500 users.** Before that, the campus-level price comparison is a natural experiment, not a controlled one, and should be read as such.

---

## 11. Open decisions

| Decision | Status |
|---|---|
| Rust rewrite of engine | Recommended, not committed |
| Local extraction + cloud generation split | Recommended |
| Cloud inference provider (zero-retention required) | Not selected |
| Sync approach: user's own storage vs. encrypted relay | Leaning user's own storage |
| Launch platform | **Closed: desktop only** |
| Pilot campus | **Closed: UA, then UK** |
| First connector | **Closed: ICS feed parser** |
| Canvas personal access tokens | **Closed: rejected on FERPA grounds** |
| Free tier | **Closed: permanent, local-only, domain boundary** |
| Price | $9.99/mo, $69.99/academic year — recommended, not committed |
| Summer auto-pause | Recommended |
| Blackboard REST app ID request to UA OIT | Deferred until users exist |
| Institutional/LTI path | Deferred; step 4 only if numbers demand |
| Advisor channel | Recommended, test during pilot |
| Syllabus dedup cache | Recommended; needs opt-in design |
| Legal review — scraping and minors | **Not started. Highest priority external dependency.** |
| Co-founder | Not addressed. Both successful comparables had two people. |

---

## 12. Ingestion notes — 2026-09-01

Filed by Claude on ingestion. The body above is Quinn's, verbatim. This section records what was
checked against the actual repo, and it is the part to read if you only read one.

### 12.1 Three recommendations this repo has already shipped

The document reads as forward-looking product strategy. Three of its load-bearing recommendations
describe code that already exists and, as of today, is already ported to Rust.

| Document says | Repo state |
|---|---|
| §4.5 — "**Revised build order:** ICS parser is connector #1, not Blackboard REST" | **Already true, since before this document was written.** `config/ingest.yaml` line 1 holds the Blackboard **ICS feed URL**; `engine/ingest.py:parse_ics` has always been the connector. Blackboard REST was never built. `src/ingest.rs` ported it in wave 2 and it is covered by the golden oracle. |
| §5.3 — third-party coursework platforms (zyBooks, VHL, Gradescope…) "rarely sync deadlines back to the LMS and mostly have no public API. **This is the differentiator; no LMS-only tool catches them**" | **Already live since 2026-08-26.** `engine/coursework.py` + `zybooks.py` + `vhl.py`, running in the local runner twice daily against real CS 100 and GN 103 workload. The single thing the document names as the category-defining differentiator is the thing this repo has been running longest. |
| §4.6 — the Shovel opening: Shovel makes the user type effort estimates; derive them instead, from assignment type, credit hours, history and grading weight | **Already the design.** `effort_hours` + `effort_source` (`vendor`/`inferred`) + `effort_confidence` are in the note schema, and `CLAUDE.md`'s standing rule is that effort judgment happens at note creation or in enrichment, **never inside the engine**. The vendor path is live: zyBooks and VHL supply `effort_source: vendor`. |

**Why this matters beyond bookkeeping:** the document's competitive analysis concludes that
coursework tracking is commoditized to free and that the differentiator lies elsewhere. That
conclusion is right, and this repo is already on the correct side of it — which means the strategy
change is a *pricing and positioning* change, not a rebuild.

### 12.2 One factual refinement, from operating the thing

**§4.4 understates the ICS ceiling.** It says the ICS feed gives "due dates and schedule only —
not grades, grading weights, submission status, or completion state," and concludes it "does not
serve mechanic #6 (completion detection)."

UA's Blackboard Ultra ICS feed **does** carry gradebook entries: `_blackboard.platform.gradebook2.GradableItem-*`
uids appear in it, and `tasks/task-erste-reflexion.md` in the test fixture is one. What they lack is
a **course identifier** — which is exactly why `CLAUDE.md` records that "gradebook events carry no
course identifier; attribution = enrichment judgment + uid pins."

So the ceiling is higher than the document assumes, but the correction does not change its
conclusion: grading *weights* genuinely are absent from the feed, and they are the input §4.6 needs
to derive effort estimates. **§4.5's two-source architecture (ICS + syllabus PDF) stands** — the
syllabus half is load-bearing for a different reason than the document gives.

### 12.3 What the new pricing model obliges, architecturally

The free/paid boundary in §8.4 is the first thing in either document that constrains the engine.

- **Free is local-only and single-device.** That is a stronger constraint than the first document's
  §2.4, which routed proposal generation to the cloud for *everyone*. Under §8.4, cloud proposal
  generation is paid — so **the free tier must work with zero cloud calls at all**, deterministic
  engine only.
- **Most of the current approvals surface is a paid feature.** The approvals queue's main producer
  today is Gmail ingest, and §8.4 puts email ingestion behind the paywall, along with the
  opportunities/people/work/content domains and multi-device sync. A free-tier user gets coursework,
  the morning view, ranking and capacity — and an approvals queue with almost nothing in it.
- **The entitlement seam does not exist.** §10.3 says to build it now, before mobile needs it.
  Nothing in `engine/` or `src/` knows what a tier is, and nothing should until the boundary is
  confirmed — but it belongs in the same milestone as local profiles, because both partition the
  same thing: what this installation is allowed to do.
- **§7.6's syllabus dedup cache is the first genuinely server-side component** in either document.
  It is not in tension with "consider not running a sync server at all" (first doc §4) — that is
  about *user data*, and this is derived course data — but it is the first thing that requires
  infrastructure, an opt-in flow, and a privacy story of its own. Treat it as a project, not a
  feature.

### 12.4 Open questions this raises that neither document answers

1. **Does the free tier get an approvals queue at all?** §8.4 makes the answer "yes, but nearly
   empty." If the morning page's approvals line is normally blank for free users, that is a visible
   difference in the product's core surface, not a hidden capability flag. Worth deciding
   deliberately rather than discovering.
2. **§0 marks the first document's 0.1–5% campus-adoption range "too pessimistic as a ceiling,"
   citing Coursicle's 80–90%** — and then §9 reuses the same 0.1–5% percentages. That is consistent
   (§9 counts *paid* users and says so), but the §0 row reads as a larger revision than §9 delivers.
   The genuinely new claim is that the *free* user count is unbounded by those figures.
3. **Solo founder.** §11's last row — "Co-founder: not addressed. Both successful comparables had
   two people" — is the only open decision in either document with no owner and no next action.
   Coursicle took roughly four years from first script to business with two people at ~100 hrs/week;
   §3 states that plainly. It is recorded here so it does not quietly disappear between documents.

### 12.5 What did NOT change

- **Nothing about the Rust port.** The port is behaviour-preserving to a fixed golden file; pricing,
  tiers and distribution do not touch it. Waves 4–7 proceed unchanged.
- **The security precondition still stands.** This document does not repeat the first document's
  §5.1 claim that "the repo is now public" — which was checked on 2026-09-01 and is false. The
  correction and the required order of operations are in the first document's filed copy, §12.
- **`config/events.yaml`'s six campus sources** remain the opportunities producer, and §5.4 puts
  Handshake and campus engagement platforms in the same tier — consistent with what is already live.
