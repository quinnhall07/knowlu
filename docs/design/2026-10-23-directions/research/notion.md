# Notion as a design source

Research for the 2026-10-23 directions run (five UI directions for onboarding, the "Here's your
week" reveal and the daily "what's next" page). Written 2026-10-23. Every direction starts from
Notion and Stripe; this file covers Notion.

**How to read it.** Each finding gives what it is, why it works, the source, how it carries over to
Knowlu, and where it does not. Sources are graded:

- **Primary:** Notion's own blog, help centre or release notes, or Notion's founders and designers
  in their own words.
- **Teardown:** an independent flow record or analysis (Appcues GoodUX, Candu, Supademo, Onboard Me).
  Teardowns disagree on details, such as how many signup questions there are. Where they do, the
  disagreement is noted.
- **Observed:** the behaviour of the shipped product as widely documented. These findings have no
  single URL, so treat them as needing a fresh look before a direction relies on one.

**What did not carry over.** Notion is a blank tool that the user builds in. Knowlu builds the plan
for the student. Most of Notion's onboarding exists to beat the blank page, and Knowlu should never
show one: by the "Here's your week" reveal, everything is already filled in. So the parts that
transfer are about **how Notion makes a filled page feel calm, owned and editable.** The "choose a
template" parts mostly do not transfer.

---

## 1. Onboarding

### 1.1 One intent question shapes everything after it
- **What:** Early in signup, Notion asks "How do you want to use Notion?" (Work, Personal, School).
  Some flow records describe two or three questions ("What do you need?", "How do you work?", team
  or solo). The answer decides which templates appear, which starter workflows load and what the
  default workspace looks like.
- **Why it works:** One cheap answer buys a workspace that looks made for the user. It is also a
  segmentation signal for Notion; Community Cafe reports the quiz showed which personas adopted
  early.
- **Sources (teardown):** https://supademo.com/user-flow-examples/notion ;
  https://candu.ai/blog/how-notion-crafts-a-personalized-onboarding-experience-6-lessons-to-guide-new-users ;
  https://onboardme.substack.com/p/how-notion-solved-the-blank-page-product-strategy-deepdive
  (third question "What will you use Notion for?").
- **Carries over:** Knowlu should not ask, because it can **infer** (VISION: "asks for sign-ins,
  nothing else"; memory: login-only onboarding). The idea that carries over is that **the first
  thing the student gives should visibly shape what comes next.** After the school sign-in, the
  next screens should name the student's own school and term, and the proof should count their
  real courses.
- **Does not carry over:** a persona quiz. The school sign-in answers it better than the student
  could.

### 1.2 The preview builds as you answer
- **What:** Candu records that while the user picks a use case, a preview of the interface changes
  next to the form, so the workspace takes shape before setup ends. The flow has a progress
  indicator, and Appcues calls its copy "succinct, colloquial, and reassuring".
- **Why it works:** Each answer gets an immediate, visible result, so the questions feel
  worthwhile. The user can see what they are building.
- **Sources (teardown):** Candu (above); https://goodux.appcues.com/blog/notions-lightweight-onboarding
- **Carries over, strongly:** a two-pane onboarding. The left pane holds the current step: the
  benefit line, then the sign-in, then the proof. The right pane holds **the student's week,
  assembling.** Course chips appear after the school sign-in. Due dates land on days. Turned-in
  work drops into a quiet "already done" tray. Class blocks appear once the syllabi are confirmed.
  The "Here's your week" reveal is then the right pane growing to full width, not a new page. This
  is the ruled order (account, school, proof, payment, Gmail, calendar, homework sites, syllabi,
  reveal) told as one picture filling in.
- **Does not carry over:** Notion's preview is a mock-up. Knowlu's must be real data or plainly
  pending ("reading syllabi..."). Never show invented placeholder tasks in place of real ones.

### 1.3 Optional steps are labelled optional, and come after value
- **What:** Import from Trello, Asana or Google Docs is optional, "helpful for high-intent signups".
  App cross-sells (desktop, mobile, web clipper) are optional and come last. Supademo names their
  placement as Notion's main onboarding risk: prompts for Calendar setup, the desktop app and meeting
  scheduling appear **before** the first real value, adding steps without moving activation.
- **Sources (teardown):** Appcues; Supademo.
- **Carries over:** the spec already makes homework sites and the calendar skippable "with the cost
  named". From Notion's mistake: **never put an optional ask before the first proof.** The school
  sign-in and its proof ("found 5 classes, 23 assignments, 9 already turned in") must come before
  anything skippable, and before payment, which the ruled order already does.
- **Does not carry over:** Notion's import is a one-off migration. Knowlu's sources are kept
  connections, so each skip should say what Knowlu will miss every day, not only today.

### 1.4 Learn by doing, through a checklist page made of the product itself
- **What:** New users land on a "Getting Started" page that is itself a Notion page. It holds
  to-dos such as "Type '/' for slash commands", with hover tooltips. Doing the checklist teaches the
  product. There are no modal tours. Appcues reports Notion's reasoning: "too much white space could
  actually be unnerving for new users".
- **Sources:** Appcues (teardown); https://designerfund.com/blog/how-to-unlock-product-success-through-growth-design
  (teardown: "without relying on tutorials or pop-ups").
- **Carries over:** Knowlu's first main page should teach through its own rows. The first-day list
  could hold one real, small item ("Confirm Tuesday's lab time, 10 seconds") that teaches the
  decision pattern by being a real decision. Undo on the first auto-marked turned-in item teaches
  "everything can be undone" the same way.
- **Does not carry over:** a fake "learn Knowlu" task list. Knowlu's list is sacred ("the list of
  what to do is the core of the product"), so a tutorial item in it would be a lie about the work.

### 1.5 Five templates, not five hundred
- **What:** Notion uses the signup answers to show about **5** recommended templates out of a huge
  gallery. Supademo flags "unlimited template variety can slow the decision to start".
- **Sources (teardown):** Appcues; Supademo.
- **Carries over:** a short, curated choice wherever Knowlu must ask. The syllabus-confirm step
  should show one proposed time per class to confirm, with "something's off" behind it, not an
  editor.
- **Does not carry over:** templates as a concept. Knowlu has one shape.

---

## 2. Information architecture

### 2.1 Everything is a block; type is separate from data
- **What:** "Everything you see in Notion is a block": text, list items, database rows, pages. A
  block has an ID, properties, a type, a content array (its children) and a parent. **Changing the
  type leaves the properties alone.** A checked to-do turned into a heading and back keeps its
  check. "Indentation is structural": nesting is the real parent-child tree, not visual spacing.
- **Why it works:** One mental model covers the whole product. Every block has the same handle,
  menu and drag, so a gesture learned once works everywhere.
- **Source (primary):** https://www.notion.com/blog/data-model-behind-notion (Jake Teton-Landis,
  Notion engineering).
- **Carries over:** Knowlu already has this shape. A task note is one object (title, course, due
  date, estimate, why) that can **render** as a Today row, a week-strip mark, a course-page line,
  an archive entry or a "Late window closed" entry. Design every view as a render of the same
  object, with the same handle, the same open-on-click detail and the same edit-in-place controls
  (DESIGN-PRINCIPLES 12 and 13). Moving a task between Today and Later changes where it renders,
  never what it is.
- **Does not carry over:** free nesting and free typing. Students should not restructure their
  tasks into pages. The vault schema is fixed.

### 2.2 Indentation as structure, used sparingly
- **What:** As above. Children render under their parent: lists indent, toggles hide their
  children until opened, pages open them.
- **Carries over:** a task's steps (from the product brief: "steps") can be its indented
  children, collapsed behind a toggle, so a row stays one line until opened. The toggle is
  Notion's best tool for keeping detail "on request" (principle 5).
- **Does not carry over:** more than one level. A tired student at 11 pm should never expand a tree.

### 2.3 Home: a greeting, then what's next, then recents
- **What:** Notion Home is a sidebar tab made of sections: upcoming events (from Notion Calendar,
  joinable), recents, My Tasks (fixed at the bottom), favorites and others. Thomas Frank's guide
  and the help centre describe a time-of-day greeting with an editable nickname. Each section can be
  resized, reordered or hidden through its `•••` menu. **Sections with nothing to show do not
  appear:** Favorites appears only after a first favorite, Shared only after a first share.
- **Sources:** https://www.notion.com/help/home-and-my-tasks (primary);
  https://thomasjfrank.com/notion-home-everything-you-need-to-know/ (expert guide).
- **Carries over:** the hide-until-relevant rule is a fit for Knowlu's ambient level. "Needs you"
  (decisions), "Late window closed" and notices should **not exist on screen** until they have
  something in them. No "0 decisions" box. Home's order (now, then soon, then my work) also matches
  Knowlu's jobs list ("what do I do now?" first).
- **Does not carry over:** user-configurable widgets. Notion can hand layout to the user because
  it is a construction kit. Knowlu's order is the product's judgment, and "position carries meaning"
  (principle 6) breaks if each student rearranges it. Let them hide the optional, never reorder the
  core.

### 2.4 The sidebar is quiet and stays put
- **What (observed):** a fixed, collapsible left sidebar on a slightly warmer or greyer surface than
  the page. It holds the workspace switcher, search, Home and Inbox at the top, then the page tree.
  Item icons are small, labels are regular weight, and hover reveals `+` and `•••` only on the row
  under the pointer.
- **Why it works:** navigation stays one glance away but never competes with the page. Hover-only
  controls keep the resting state clean.
- **Carries over:** the main page should be the page. Courses, week and settings go in a narrow,
  quiet rail that collapses at 1280 px. Per-row actions (snooze, done, open) appear on hover or
  focus, **and stay keyboard-reachable** (principle 14). They are never visible-only-on-hover for
  touch or keyboard users.
- **Does not carry over:** the infinite page tree. Knowlu has about five destinations.

---

## 3. Typography, whitespace and colour

### 3.1 Spacing by rhythm and neighbour, not by baseline grid
- **What:** In "Updating the design of Notion pages", Notion explains that it dropped baseline
  alignment ("hard on the web" with variable block heights) for standardised spacing that gives
  "a feeling of rhythm". Paragraph spacing was split from line spacing because paragraph breaks were
  hard to scan. **Adjacency rules** shrink the padding between neighbouring list items "to make them
  chunk together neatly" while paragraphs get more room.
- **Source (primary):** https://www.notion.com/blog/updating-the-design-of-notion-pages
- **Carries over:** a direct recipe for the Today list. Rows of the same group (Today, Tomorrow)
  sit tight. The gap **between** groups is clearly larger. A sentence such as a "why" line or a
  decision gets paragraph spacing. The gaps do the grouping, so the page needs fewer labels and
  boxes ("whitespace does the structural work").
- **Does not carry over:** nothing to exclude. This is pure craft.

### 3.2 One family, a narrow measure, text never pure black
- **What:** Notion uses one sans family (Inter-based) from the 11 px calendar time labels up to
  display headlines. Pages offer three typographic voices, `Default`, `Serif` and `Mono`, plus
  "Small text" and "Full width" toggles. Default content sits in a narrow centred column. Text is
  near-black (`rgba(0,0,0,0.9)`) on a warm white or warm grey.
- **Sources:** https://www.notion.com/help/customize-and-style-your-content (primary, the
  options); https://blakecrosley.com/guides/design/notion-calendar (independent design analysis,
  the colour values and one-family claim).
- **Carries over:** one family with weight and size doing the hierarchy, tabular numerals for times
  and hours, and a reading measure of about 60-75 characters for the task column, even at 1280 px.
  Use the extra width for an ambient side column, not wider rows. Near-black on warm neutrals suits
  Knowlu's red wordmark and keeps red free for the brand (principle 9).
- **Does not carry over:** per-page font choice. It is decoration with no job for Knowlu
  (principle 7).

### 3.3 Soft colour fills with a semantic job
- **What:** Notion has a small fixed palette (default plus about nine named colours), each with a
  soft background and a darker text tone. Blake Crosley's analysis of Notion Calendar notes that each
  colour has "a semantic role rather than a decorative one". Ivan Zhao's office rule makes the same
  point about restraint: "if you cut green from the palette entirely, everything still looks okay."
- **Sources:** Crosley (above); https://designerfounders.substack.com/p/ivan-zhao-notion (Zhao,
  quoted).
- **Carries over:** a course gets a soft tint (a light fill and a dark label), applied the same way
  everywhere: Today row, week strip, class block, grade line. The tint is the course's identity,
  never a state, so it can never read as an alarm. The course name is always printed next to it
  (principle 10).
- **Does not carry over:** red as a course colour. Red is the Knowlu brand.

---

## 4. Empty states and making a tool feel ready

### 4.1 Placeholders that teach the next action
- **What (observed):** an empty line shows grey hint text, "Type '/' for commands" (or, in newer
  builds, a hint to press space for AI). A new page shows a grey "Untitled" title and a short row of
  one-click starts (templates, import, AI). The hint disappears as soon as there is content.
- **Why it works:** the empty state is the instruction, placed where the action happens, and it
  never outlives its use.
- **Carries over:** where Knowlu has a real empty state, the empty text names the next action and
  sits where the content will appear. "Nothing left today. Next up tomorrow: Lab report, 40 min" is
  an empty state that is also a "what's next" answer. The brand voice says being ahead is
  permission to rest, so say that, then show the next thing quietly.
- **Does not carry over:** an empty first-run Today. Knowlu's onboarding guarantees the page is
  full the first time it appears (the spec's "main page waits until everything has run once").

### 4.2 Sample content and templates beat a blank page
- **What:** Notion preloads a workspace matched to the intent answer (weekly to-do, project
  planner, habit tracker) and surfaces community templates when users add pages. The onboarding
  teardowns call this the fix for "blank page paralysis": users learn by editing a working example.
  A 2026 tutorial notes some users delete the default templates straight away.
- **Sources (teardown):** Onboard Me (above); Supademo; Appcues.
- **Carries over, inverted:** Knowlu's "sample content" is the student's own real work, fetched
  during onboarding. The lesson is that **a full, editable first view beats an explanation.** The
  "Here's your week" reveal is Knowlu's version of the preloaded workspace.
- **Does not carry over:** fake sample data in the student's list. Seed data the user has to delete
  is exactly what Quinn's first install suffered from (28 live tasks, many already turned in).
  Turned-in work is archived at first sight, by the ruled spec.

### 4.3 The first-session goal is one structured page that matches intent
- **What:** Supademo defines Notion's activation ("First Strike") as creating or receiving a
  working page that matches the stated purpose. The AI path ("generate a page") is the fastest
  route there.
- **Carries over:** Knowlu's equivalent is VISION's "a new student reaches 'it already knows what's
  next' in their first session". Every onboarding step should be judged by whether it moves toward
  that moment. The reveal should open on the **first task**, not on a tour.

---

## 5. Notion Calendar (formerly Cron): time views

### 5.1 Keyboard first, so the UI can stay minimal
- **What:** single-key commands (`n` new event, `g` go to date, `t` today, `s` share
  availability), arrow keys between days, `?` for the shortcut list, a command menu, and
  system-wide shortcuts that show the menu bar or join the next meeting from anywhere. Crosley sums
  it up: "The UI can be minimal BECAUSE keyboard commands handle complexity."
- **Sources:** https://www.notion.com/help/notion-calendar-keyboard-shortcuts (primary);
  https://www.notion.com/blog/introducing-notion-calendar (primary); Crosley (analysis).
- **Carries over:** Knowlu's main page could have a small single-key set: `j`/`k` to move,
  `x` or space for done, `z` to snooze, `e` to edit the estimate, `w` for the week, `?` for the
  list. With these, row chrome can stay hidden until hover or focus. This also serves principle 14.
- **Does not carry over:** keyboard as the main teacher. Most students will click. Shortcuts are
  an accelerator, never the only path.

### 5.2 The menu bar: see what's next without opening the app
- **What:** Notion Calendar lives in the menu bar: today's schedule and a join button, one glance
  away. Notion's stated reason: "checking the time of an upcoming meeting may feel harmless, but
  this context-switching can be detrimental to productivity."
- **Source (primary):** https://www.notion.com/blog/introducing-notion-calendar
- **Carries over, strongly:** Knowlu already has a tray. A tray flyout showing **the one next
  task, the next commitment, and free time left today** answers "what's next?" without opening a
  window. It is the most Knowlu-shaped idea in Notion's whole catalogue. Desktop-only fits it
  well.
- **Does not carry over:** a join-meeting focus. Students' "next" is mostly work, not calls.

### 5.3 Deadlines from the work tool shown in the time view
- **What:** Notion Calendar shows Notion database items as events, such as sprint deadlines, and
  links pages to events, so an event opens its document in one click. Linked-page indicators stay
  small and open on demand. Crosley's heading for this is "Integration should be available, not
  visible."
- **Sources:** Notion Calendar launch post (primary); Crosley (analysis).
- **Carries over:** Knowlu's week view should put **due work and commitments on one time axis**:
  class blocks as filled bands, due items as small marks at their due time, and free time as the
  gaps. A class block could link to that class's open tasks. Each link is a small indicator that
  expands when clicked. This is where the "Here's your week" reveal can live.
- **Does not carry over:** drag-to-create time blocking. Knowlu's students "think in priority, not
  time slots" (PRODUCT-BRIEF), and class times come from syllabi and the registrar, never typed.
  Dragging work into slots would turn Knowlu into a scheduler.

### 5.4 A grid that organises without competing
- **What:** 1 px grid lines at about 9% opacity, monospaced numerals in the time gutter, events as
  soft fills with darker text, and no decorative containers (Crosley).
- **Carries over:** a recipe for the week strip and the reveal. Faint structure, tabular times,
  course tints, and whitespace instead of cards.

### 5.5 Blocked time keeps the details private
- **What:** "Event blocking" mirrors busy time across a person's work and personal calendars. Others
  see the time as blocked but not what it is: "Colleagues won't be able to see you have a doctor's
  appointment."
- **Carries over:** a good model for Knowlu's own privacy story in onboarding copy. The calendar
  step can say what Knowlu reads (busy times and class events) and where it stays (on the device,
  matching "commitment proposals never leave the device").

---

## 6. Philosophy worth stealing

- **"Make the dumbest path great."** Ivan Zhao: "Most of the time, what matters is the dumbest
  path. Make that great," and "Designers spend too much time on edge cases." Source:
  https://designerfounders.substack.com/p/ivan-zhao-notion . For Knowlu, the dumbest path is: open
  the app, see the first task, start it. Design that path first, and push decisions, grades and
  settings one step away.
- **System first.** Zhao: "When you add a new LEGO brick, it might pollute the whole thing. So you
  have to think system-first." Every new Knowlu surface should reuse the row, the tint and the
  toggle rather than invent a component.
- **Calm by subtraction.** Users compare Notion's appeal to Marie Kondo's (Station F interview,
  https://stationf.co/news/interview-with-ivan-zhao-co-founder-and-ceo-of-notion/). The calm comes
  from a small palette, one type family and hidden chrome, not from soft illustrations.
- **The self-organising inbox (Notion Mail, April 2025).** The student says once what matters, and
  Notion AI auto-labels incoming mail into custom views. Jason Ginsberg (to TechCrunch) said the aim
  was not "attaching AI to an existing inbox" but letting users build a custom one. Sources:
  https://www.notion.com/blog/introducing-notion-mail ;
  https://techcrunch.com/2025/04/15/notion-releases-its-ai-driven-email-inbox . For Knowlu's
  Gmail step, the proof after consent could be "found 3 emails that look like assignments, from 2
  professors". **Does not carry over:** asking the student to define labels. Knowlu infers.

---

## 7. Where Notion is the wrong model

- **Notion is a canvas; Knowlu is a verdict.** Notion's whole UX assumes the user decides structure.
  Knowlu decides the order and lets the student correct it in under 30 seconds. Borrow Notion's
  surface calm, not its open-endedness.
- **Configurability.** Notion hands layout to users (Home sections, fonts, full width). Knowlu's
  position carries meaning, so the core order must not be user-rearrangeable.
- **Status through decoration.** Covers, emoji icons and per-page fonts are Notion's ownership
  levers. In Knowlu they would be noise. Ownership comes from seeing your own real classes.
- **Hover-only discovery.** Notion leans on hover. Knowlu has to be fully keyboard and
  screen-reader operable, so every hover control needs a focus equivalent.

---

## Sources (all checked 2026-10-23)

Primary: notion.com/blog/data-model-behind-notion · notion.com/blog/updating-the-design-of-notion-pages ·
notion.com/blog/introducing-notion-calendar · notion.com/blog/introducing-notion-mail ·
notion.com/help/home-and-my-tasks · notion.com/help/customize-and-style-your-content ·
notion.com/help/notion-calendar-keyboard-shortcuts · designerfounders.substack.com/p/ivan-zhao-notion ·
stationf.co/news/interview-with-ivan-zhao-co-founder-and-ceo-of-notion/

Teardowns and analysis: goodux.appcues.com/blog/notions-lightweight-onboarding ·
candu.ai/blog/how-notion-crafts-a-personalized-onboarding-experience-6-lessons-to-guide-new-users ·
supademo.com/user-flow-examples/notion · onboardme.substack.com/p/how-notion-solved-the-blank-page-product-strategy-deepdive ·
blakecrosley.com/guides/design/notion-calendar · thomasjfrank.com/notion-home-everything-you-need-to-know/ ·
designerfund.com/blog/how-to-unlock-product-success-through-growth-design ·
techcrunch.com/2025/04/15/notion-releases-its-ai-driven-email-inbox

Not found: a Growth.Design Notion case study, and an official Notion post on its onboarding redesign.
The onboarding details above therefore rest on teardowns that differ on the question count.
