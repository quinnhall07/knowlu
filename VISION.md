# Vision — the end state

This file holds the full end state so v1 decisions don't foreclose it. **Check every new design
against it.** Sources: design doc §1 and §11; the product and business plan
(`docs/superpowers/notes/2026-09-01-product-and-business-plan.md`, "the plan" below).

**Two horizons, one system.** quinn-ops is Quinn's daily driver *and* the seed of a product for
students. Ruling, 2026-09-01: **design toward the app, build for personal use** — with the caveat
that friends will test and demo soon, so onboarding and per-user profiles are near-term work, not
post-pilot work.

## The question it answers

*What should I work on today, and in what order?* — from sources that already know your obligations,
without manual entry. That is the whole product. Everything else is in service of it.

## Design commitments

Three, and they drive nearly every technical decision. A design that breaks one of these is wrong
even if it is otherwise better.

1. **The engine is deterministic.** Same input, same output. No inference inside it. Judgment happens
   at the boundary — today in Claude, in the product in small local models — and its output enters
   the engine as data.
2. **Local-first.** Not a cloud service that owns user data. The source of truth is plain text files
   on the user's machine.
3. **The AI never produces anything the user sends externally.** No drafted emails, notes, or
   messages. Its only generative output is internal proposals and info displays.
   *(Added 2026-09-01. This one is new, and it cut a feature — see People, below.)*

## The system succeeds when — personal

1. Quinn opens the Today tab first thing and works from it without a parallel mental list.
2. The ranking is agreed with, or correctable in under 30 seconds when it isn't.
3. Nothing is missed that the system had the information to catch.
4. Work completes before due dates, visible as slack trending flat or up.

Failure modes designed against: trust decay, notification habituation, capture friction, silent
changes.

## The system succeeds when — product

5. **A new user reaches "it already knows what I should work on today" in their first session**, not
   their first week. The plan §9.3 makes this the decisive retention moment; onboarding is judged
   against it and nothing else.
6. **Adding a source is one action** — a URL or a login — and a source that later breaks fails
   visibly rather than silently going quiet.
7. **A second user needs nothing from Quinn.** No hand-edited config, no shared credentials, no
   assumption that the vault is a git repo someone else pushes to.

## Domains beyond school (v1 is school-only)

- **Tutoring** — Calendly bookings already arrive on the calendar. Client notes per client, session
  prep proposed the day before, covered/to-cover tracking. First names preferred.
- **SetNForget** — Linear stays the work task list; this system surfaces "work on weekly sprint
  goals" as a commitment and keeps MCP access to Linear for editing from Claude Code.
- **Content** — a single weekly commitment line until posting actually starts.
- **Opportunities** — Handshake and ECDC event scanning, internships, research, dinners. Needs a
  noise filter shaped by real examples of things missed.
- **People** — **AMENDED 2026-09-01.** Contact-cadence *reminders* and meeting *proposals* for
  advisors, professors and directors. **Email drafting is cut** — commitment 3 wins over the original
  specification. The system may tell Quinn it is time to contact someone; it never writes what he
  would send. `docs/drafts/2026-08-20-course-emails.md` remains a one-off artefact; no producer
  replaces it.

## Product end state

Full detail in the plan; this is what a design must not foreclose.

- **Desktop only at launch**, Rust + Tauri, OS-native webview. Target **under 50 MB** base app,
  ideally under 20. Not Electron. Mobile is a later port, and the shared Rust core is what makes it
  a UI job rather than a rewrite (plan §6.2).
- **Minimize the AI surface.** Most capabilities need no model at all. Schema-constrained decoding on
  every model call; classification over generation; a **rule-promotion loop** that retires model
  calls into deterministic rules as patterns repeat (plan §2.2–§2.3).
- **Local extraction, cloud generation.** Embedding and extraction models stay on the device, because
  that is where raw personal content is touched; templated proposal generation goes to a
  zero-retention cloud provider. ~1–2 GB resident locally (plan §2.4).
- **Teach once, run deterministically forever.** A browser and a model bootstrap a source *once*; the
  output is a small human-readable extraction script that every later sync runs directly. A broken
  script is a visible failure, and its regeneration is a proposal to approve — "judge once,
  re-propose freely" applied to scrapers (plan §3.3).
- **Sync replays the journal**, encrypted client-side, over storage the user already trusts. Device
  pairing over accounts. Conflicts surface as proposals, never silent merges (plan §4).
- **Telemetry is structured events computed on-device**, aggregates only, opt-in, minimum cohort size
  before any slice is usable. Never raw content — "depersonalized" does not survive a unique course
  schedule (plan §7).
- **The first connector is the LMS ICS feed, not an LMS API.** Both Blackboard Ultra and Canvas
  expose a student-generated `.ics` URL: no admin approval, no credential handling, scoped to one
  student, and the same connector for both campuses. Canvas personal access tokens are **rejected
  on FERPA grounds** — a student's token can expose other students' records, and universities are
  actively prohibiting the pattern (market doc §4.3–§4.5). This repo has always worked this way.
- **quinn-ops only ever reads.** Read-only by default is a landing-page sentence, and it shrinks
  OAuth scope and legal exposure at the same time. Rare writes — a block on the user's own
  calendar — stay behind explicit approval (market doc §5.2).
- **A free tier must run with zero cloud calls.** Free is local-only, so the deterministic engine
  has to be complete on its own. An entitlement seam belongs with local profiles, since both
  partition the same thing: what this installation is allowed to do (market doc §8.4, §10.3).
- **Onboarding and local profiles are near-term.** A per-user vault, settings and credential store,
  and a first-run flow. Hosted accounts are a *separate, later* decision, needed only for
  cross-device sync and billing.

## Standing rules that must survive every phase

- If only the user is affected, the system acts; if another person can see the outcome or the user's
  schedule changes, it proposes and waits for approval.
- One daily email at most. Toasts only for time-critical-and-at-the-desk. Ambient first.
- Failures are visible; silence is never ambiguous.
- Every automation is editable text — nothing lives only in a UI, and nothing lives only in a chat
  transcript.
- **Credentials live in the OS keychain.** Never in the repo, never in a log, never in anything a
  model prompt touches.
- **Data minimization is a rule, not a preference.** Collect the least that answers the question.
- **No single-user assumptions.** Anything that would need hand-editing for a second user is a bug
  from 2026-09-01 onward, even while only Quinn runs it.

## Open decisions

Tracked here because each one forecloses something if decided carelessly.

| Decision | Status |
|---|---|
| Hosted accounts vs local profiles only | **Local profiles now**; hosted deferred until sync or billing needs one |
| Cloud inference provider (zero-retention required) | Not selected |
| Sync: user's own storage vs our encrypted relay | Leaning user's own storage as the default |
| Mobile port timing | Revisit against pilot retention — plan §6.3 flags desktop-only as the first hypothesis if the ritual dies |
| Price: **$9.99/mo, $69.99/academic year**, permanent free tier | Recommended, not committed (market doc §8). **Supersedes the old $4.99 row** — the free tier now absorbs the affordability constraint, so the paid tier is priced on value |
| The free/paid boundary | **Closed in principle** (market doc §8.4): free is coursework + morning view + ranking + capacity, local-only, single device. Paid is sync, email ingestion, the other four domains, and cloud generation. **Which side email ingestion belongs on is still a guess** (§10.7 lists it as a thing to test) |
| Summer auto-pause (no billing June–August) | Recommended (market doc §8.5). Nobody else does it |
| Co-founder | **Unowned.** Both successful comparables had two people; this is the only open decision with no next action |
| Telemetry final scope | Structured events agreed; scope not set |
| Legal read on scraping and minors | **Not started — highest-priority external dependency** (plan §8) |
| Making this repo public | **Blocked.** Requires credential rotation, then `git filter-repo`, then splitting the vault out of the code repo — in that order (plan §12) |
