# AI platforms as a threat, and how the response fits v0.5.0, 2026-11-27

Status: a recommendation, not a ruling. Written in a Claude Code session on 2026-11-27 at Quinn's request,
after Anthropic shipped "Install as app" (artifacts installed as Mac and Windows apps, beta, 2026-11-24).
It continues an earlier chat about whether "SaaS is dying" dooms Knowlu, whose summary Quinn pasted in;
§6 lists where that summary disagrees with signed text. Nothing here changes a signed document. The
items that would need Quinn's word are listed in §5.

**Sourcing.** The facts about Anthropic come from claude.com, anthropic.com and support.claude.com
(fetched 2026-11-27). The facts about Google, OpenAI, Microsoft, Instructure and Anthology come from
search-result summaries only, because those sites could not be fetched from the session. **Check them
before quoting**, especially Google's Classroom app in Gemini.

## 1. Bottom line

**Don't pivot the product. Change the approach, now, before the pilot.** Of the threats looked at,
Anthropic's new feature is the least dangerous to Knowlu. The serious one is free, good-enough
morning briefs from general assistants, which schools increasingly give students for nothing. The
defence is the part of Knowlu those assistants cannot reproduce. The pilot has to measure whether that
part is what students actually value, and §4 sets the tripwires that decide it.

Knowlu stays a Rust + Tauri desktop app. It should not become a Claude app (§2).

## 2. "Install as app"

**What it does today** (https://claude.com/resources/articles/install-as-app, 2026-11-24;
https://support.claude.com/en/articles/9487310):

- From the Share menu, an artifact becomes a Mac or Windows app with its own icon and window. It
  updates itself when the artifact is edited. No packaging, signing or API key.
- Three permission levels:
  - browser-level by default;
  - "app permissions": always-on-top, global shortcuts, notifications and a tray item;
  - "full computer access": run programs and read, change or delete files. This level is available
    **only in apps the user made themselves, or (Team and Enterprise) apps their organization made**.
- It can call Claude and the user's connectors (Gmail, Google Calendar and others). Those calls count
  against **the user's own Claude plan**.
- It must be online and signed in to Claude to open. Anthropic does not review the apps. Sharing is
  allowed, but there is no store.
- Not stated anywhere: background or scheduled runs, offline use, OS credential access.
- Nearby: Cowork scheduled tasks run in the cloud with connectors (support article 13854387), and
  Canvas reaches Claude only through an institution admin's setup (support article 11725453).

**Where it is likely heading in 6–12 months.** This is inference: Anthropic has published no roadmap.
The release order (artifacts that call Claude, then storage and connectors in artifacts, live Cowork
artifacts, the marketplace, installable apps) points to:

- scheduled triggers for installed apps;
- a directory of shared apps;
- Linux support.

**The change that would make it a real threat:** full computer access in *shared* apps. Then someone
could build a Knowlu-like app once and hand it out.

**Verdict: low threat now, medium in a year.** It shows that a planner's screen is cheap to build. The
market note already says so (§1.3: coursework is the entry ticket, not the pitch).

**Why Knowlu should not become one:**

- Every student would need a Claude subscription, about $20 a month, on top of Knowlu's price.
- It only runs while open and online.
- It cannot hold portal sign-ins, and it cannot be distributed with full access.
- It would break signed commitments: the deterministic engine, the plain-text vault on the student's
  machine, and judgment pinned to models we chose at zero-retention hosts. It would also make one AI
  platform something Knowlu cannot leave.

## 3. The other threats

| Threat | Real? | Why |
|---|---|---|
| **Morning briefs from general assistants:** ChatGPT scheduled tasks and its "College Student" plugin (ChatGPT Edu), Cowork scheduled tasks, Gemini Daily Brief | **Yes, the biggest** | Free or already paid for, on the phone, and good enough for what an LMS email and a calendar hold. Worse wherever a school gives students ChatGPT Edu or Gemini free |
| **Google: the Classroom app in Gemini** (lists upcoming assignments, builds study plans; 18+) | Real but narrow | Sees only Google's own tools. Most US colleges run Canvas or Blackboard. *Not checked against the source* |
| **The LMS vendors' own AI** (Instructure IgniteAI, Anthology AVA) | Partial | Built for teachers or limited to one course. Across-courses inside one LMS is easy for them. Homework sites, email, jobs and clubs are not |
| **Agents that use the student's logged-in browser** (Claude in Chrome and similar) | Slowly eroding | Weakens "only we can read zyBooks." What is left is reliability: scheduled, deterministic, cheap, every day, with failures shown |
| **Open-source clones; Coursicle and the other free planners** | No | Already priced in by the market note |

The sharper price matters here. The signed price is $12.99 every 4 weeks or $84.37 a year, with
14 days free (cloud design, Amendments 2026-11-10 and 2026-11-17). Against free briefs, the price is
the number most at risk. v0.4.1 builds prices so they can change, so that lever is ready. Leave the
price alone until the pilot's willingness-to-pay data says otherwise.

**What stays defensible.** All four are already design commitments:

1. **Homework sites and grade pages read on the device, with the student's own sign-in.** Platforms
   avoid this on purpose; Anthropic deliberately keeps full access to apps you made yourself.
2. **"Nothing missed," with visible failures.** A generated brief misses things silently. Knowlu's
   ranking is deterministic and a broken source says so.
3. **Typed commitments and honest free time.** A brief lists what is due. Knowlu fits it into the
   student's day.
4. **It runs while closed and works offline.** Installed Claude apps do neither.

## 4. Pivot tripwires

Decided now, so they are commitments, not a mood. Reconsider the product if, at the end of the pilot:

- fewer than half of the "Knowlu caught it" moments come from sources the assistants cannot read
  (homework platforms, grade pages, typed commitments, the registrar), or
- most pilot students already have a free AI brief and say it is enough.

The fallback is selling to schools' student-success offices. The market note (§6.2) says that conflicts
structurally with the consumer product, so it is a real pivot, not a tweak.

Signals to keep watching:

| Signal | What should be ready |
|---|---|
| Installed Claude apps get scheduled runs, or full access in shared apps | The connector spec (§5, item 7), so Knowlu ships into that world instead of being cloned by it |
| Assistant briefs reach students' phones every morning | The dedicated "Knowlu" calendar, which reaches the phone without a mobile app |
| Schools give students ChatGPT Edu or Claude for Education free | A listing in those connector directories, and pilot data on whether students still pay |
| Browser agents become reliable on homework sites | Win on cost and reliability, not on being the only reader |

Two hedges already in place should stay: the stated preference for open-weights models, and a stable,
documented read model (`docs/surface/anatomy.md`).

## 5. How the response fits v0.5.0

v0.5.0 is the fully usable product with the sell-first onboarding (`HANDOFF.md`; the v0.4.0 design
note's deferral table). It carries the write-path features, privacy bump B (terms plus the onboarding
answers, C16 in `docs/specs/2026-11-17-legal-pages-rewrite.md`) and the rest of the billing lane.
v0.6.0 is Ask.

| # | Item | Where | Why |
|---|---|---|---|
| 1 | **Source-kind tagging.** Each interaction event that marks something caught or done carries the source kind (LMS feed, zyBooks, VHL, email, grades, commitment, registrar), never content | **A v0.4.x point release, live before the pilot** | Without it the first tripwire cannot be measured. `reviewer` checks it against the current privacy page; if it needs new words, it rides bump B |
| 2 | **The positioning in the onboarding:** homework sites read on your machine, runs while closed, never misses quietly, no AI subscription needed | **v0.5.0 onboarding copy** | The ruled onboarding (`docs/specs/2026-10-23-onboarding-redesign-draft.md`) opens every step with why it helps. That is where the pitch belongs, at the cost of copy only |
| 3 | **A third closed-choice setup answer:** "What do you use to keep track now?" (ChatGPT, Gemini, Claude, the LMS, a paper or app planner, nothing) | **v0.5.0, inside C16 and bump B** | Bump B already adds two answers. Adding a third now is nearly free; later it costs another privacy bump. **Needs Quinn: it changes bump B's wording** |
| 4 | **Planned work modelled so it can be exported** | **v0.5.0, in the calendar write-path work** | It keeps the dedicated calendar (item 5) a small step later |
| 5 | **The dedicated "Knowlu" calendar in Google and Outlook** | **Not v0.5.0. v0.5.x; start Google verification during v0.5.0** | It needs `calendar.events`, a new consent and a privacy bump. Quinn ruled one Google client for sign-in and data (onboarding spec, Q8), so adding the scope probably sends sign-in's consent screen through Google verification again (an inference about Google's process, not checked). The release should not wait on it. **Needs Quinn: VISION.md has it as "later"** |
| 6 | **Artifacts as prototypes** for the long onboarding and the new calendar, class-page and Settings screens, shown to friends before `console-ui` builds them; also an internal pilot-metrics dashboard | **v0.5.0 development workflow** | Cheap to make, and it tests the onboarding before it is built. Not product scope |
| 7 | **Ask, a connector, or both.** A read-only local MCP connector over the `surface` read model would let a student's own Claude or ChatGPT explain the plan, and could carry capture as proposal cards written through `write` | **A one-page `planner` comparison during v0.5.0; Quinn rules before v0.6.0's spec** | It might replace building and running our own chat while keeping what is scarce: the data, the ranking and the trust. Against: we no longer control the model, which sits awkwardly with "judgment is a service" and VISION commitment 3. Gmail's Limited Use policy and the privacy page make it Contract-size |
| 8 | **Agent-drafted extraction scripts for new homework platforms** ("teach once, run deterministically forever") | **Development workflow alongside v0.5.0** | Coverage is the moat, so speed it up. Add a platform to v0.5.0 only if the pilot campus's intro courses need one |

What v0.5.0 owes item 7, whichever way it goes: every new read-model field (a week of events, class-page
rows, per-source read times) is documented in `docs/surface/anatomy.md` as it lands.

Order: item 1 in a point release; items 2–4, 6 and 8 in v0.5.0, with the start of item 5's verification;
item 7 decided before v0.6.0 is specified.

## 6. Where the earlier chat disagrees with signed text

The earlier chat did not have the repository. Three of its suggestions conflict with signed rulings:

- **Canvas personal access tokens:** it recommended them. `VISION.md` and the market note (§4.3) reject
  them on FERPA grounds.
- **An optional cloud sync:** it proposed adding one. Sync already exists: the account is the source of
  truth (C3′).
- **Pooling classmates' data for a network effect:** it suggested this. It would need a privacy
  amendment, and `VISION.md`'s Learning section says what is learned from one student's email trains
  only that student's model. Sharing deduplicated syllabi (market note §7.6) is the safe version.

Its legal advice (act only as the student's tool, read deadlines and never homework content, never
bypass a CAPTCHA or 2FA, comply and use a per-site shutoff if a letter arrives) agrees with
`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md`. A lawyer before the first outside sign-up is
already a rule.

## 7. Found along the way

`VISION.md` (The product) still says $9.99 a month, a 7-day trial, a $39.99 term and the Pell price. The
signed prices are $12.99 every 4 weeks or $84.37 a year, 14 days free, and no Pell price for now. VISION
is Quinn's to edit, so this note only flags it.
