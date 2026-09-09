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
   at the boundary — today in Claude, in the product in our judgment service — and its output enters
   the engine as data. *(2026-09-09: "small local models" became "our judgment service" — cloud
   design D3.)*
2. **The source of truth is plain text on the user's machine.** The cloud judges, syncs and bills;
   it never owns the vault. A user can open the folder, copy it, and leave with it.
   *(Amended 2026-09-09 from "Local-first. Not a cloud service that owns user data." — cloud design §10.)*
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
- **Judgment is a service.** Extraction and classification run in our cloud against zero-retention
  inference, grammar-constrained, model-pinned, logged by id. Nothing model-shaped ships in the app.
  The one thing that stays on the device is the fetch that needs the student's own credentials
  (fetch on device, think in the cloud).
- **Teach once, run deterministically forever.** A browser and a model bootstrap a source *once*; the
  output is a small human-readable extraction script that every later sync runs directly. A broken
  script is a visible failure, and its regeneration is a proposal to approve — "judge once,
  re-propose freely" applied to scrapers (plan §3.3).
- **Sync replays the journal** to the account's own storage in our cloud. Accounts over device
  pairing. Conflicts surface as proposals, never silent merges.
- **Telemetry:** interaction events and AI corrections are collected under the terms, keyed to a
  pseudonymous account id, read only as aggregates with a minimum cohort; raw content only behind
  a separate, revocable opt-in. Never Gmail-API-derived content, by policy.
- **The first connector is the LMS ICS feed, not an LMS API.** Both Blackboard Ultra and Canvas
  expose a student-generated `.ics` URL: no admin approval, no credential handling, scoped to one
  student, and the same connector for both campuses. Canvas personal access tokens are **rejected
  on FERPA grounds** — a student's token can expose other students' records, and universities are
  actively prohibiting the pattern (market doc §4.3–§4.5). This repo has always worked this way.
- **quinn-ops only ever reads.** Read-only by default is a landing-page sentence, and it shrinks
  OAuth scope and legal exposure at the same time. Rare writes — a block on the user's own
  calendar — stay behind explicit approval (market doc §5.2).
- **One edition, $9.99/month, an account required.** The deterministic engine is still complete on
  its own — that is what keeps today's page on the screen when the network is not — but nothing is
  free of an account. The entitlement is the subscription, checked by the service and cached on
  the device with a grace window.
- **Onboarding creates the vault.** The app makes `%USERPROFILE%\Knowlu\<Profile>` itself; nobody
  picks a folder. Hosted accounts are the first step of onboarding, not a later decision.

## Standing rules that must survive every phase

- If only the user is affected, the system acts; if another person can see the outcome or the user's
  schedule changes, it proposes and waits for approval.
- One daily email at most. Toasts only for time-critical-and-at-the-desk. Ambient first.
- Failures are visible; silence is never ambiguous.
- Every automation is editable text — nothing lives only in a UI, and nothing lives only in a chat
  transcript.
- **Credentials live in the OS keychain.** Never in the repo, never in a log, never in anything a
  model prompt touches.
- **Credentials for the student's own portals never leave the student's machine.** The cloud may
  hold the account's own secrets (session, LMS capability URL) encrypted; it never holds a portal
  password.
- **Data minimization is a rule, not a preference.** Collect the least that answers the question.
- **No single-user assumptions.** Anything that would need hand-editing for a second user is a bug
  from 2026-09-01 onward, even while only Quinn runs it.

## Open decisions

Tracked here because each one forecloses something if decided carelessly.

| Decision | Status |
|---|---|
| Hosted accounts vs local profiles only | **Decided 2026-09-09: hosted accounts, required** |
| Cloud inference provider (zero-retention required) | Anthropic API, cheapest model that passes the eval suite per kind; provider pinned per kind, swappable behind the seam |
| Sync: user's own storage vs our encrypted relay | **Decided 2026-09-09: our storage (Supabase), per account** |
| Mobile port timing | Revisit against pilot retention — plan §6.3 flags desktop-only as the first hypothesis if the ritual dies |
| Price | **Decided 2026-09-09: $9.99/mo, no free tier.** The $69.99/academic-year price is kept as a Stripe option (cloud design §11 R3) |
| The free/paid boundary | Moot: there is one edition |
| Summer auto-pause (no billing June–August) | **Decided 2026-09-09: kept** (cloud design §11 R3 — Quinn accepted that it halves summer revenue). Nobody else does it |
| Co-founder | **Unowned.** Both successful comparables had two people; this is the only open decision with no next action |
| Telemetry final scope | **Decided 2026-09-09:** §6 of the cloud design |
| Legal read on scraping and minors | Landscape written 2026-09-09 (`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md`); a lawyer before the first non-founder paid sign-up |
| Making the code public | `quinn-ops` stays private and archived; `knowlu` has no secrets in its history by construction (the fixtures carry the founder's coursework titles — replace them with synthetic vaults before any public release) |
