# Knowlu: know what's next

What Knowlu is and must stay, so near-term decisions don't foreclose it. **Check every design against
this file.** It states what holds now; how each point was decided lives in `docs/specs/2026-09-09-knowlu-cloud-design.md`
(signed, with its amendments), the product and business plan
(`docs/notes/2026-09-01-product-and-business-plan.md`, "the plan"), and git history.

## The question

Every morning Knowlu answers one question for a student: **what should I work on today, and in what
order?** It answers from sources that already know the student's obligations, without manual entry.
That is the whole product.

**The test for every feature:** does it improve the morning answer? A feature that does not is cut or
deferred, however good it is on its own.

## Design commitments

A design that breaks one of these is wrong even if it is otherwise better.

1. **The engine is deterministic.** Same input, same output, no inference inside it. Judgment happens
   at the boundary, in our judgment service, and enters the engine as data. `rank` never calls a model.
2. **The account is the source of truth; every desktop holds a plain-text mirror the engine ranks
   offline.** The cloud stores, judges, syncs and bills. The service can read what it stores
   (encrypted at rest, not end-to-end), the privacy page says so, and deleting the account deletes it.
   Nothing the student sees depends on the network being up.
3. **The AI never produces anything the student sends externally.** No drafted emails, notes or
   messages. Its only generative output is internal proposals and information displays.
4. **Time is typed by commitment.** Every confirmed commitment is a note in the vault with a kind
   (class, lab, work, club, meeting, office hours, …) and a level: **hard** (never overlapped), **soft**
   (a proposal may overlap it, and says so) or **optional** (never busy time; it changes nothing the
   morning answer shows). Knowlu never proposes over a hard commitment. Google Calendar and other feeds
   are inputs to the vault, never its source of truth. Detail: `docs/specs/2026-09-23-commitment-model-design.md`.

## Knowlu succeeds when

1. A student opens Today first thing and works from it, without a parallel list in their head.
2. The ranking is agreed with, or correctable in under 30 seconds when it isn't.
3. Nothing is missed that Knowlu had the information to catch.
4. Work finishes before it is due, visible as slack trending flat or up.
5. **A new student reaches "it already knows what I should work on today" in their first session**,
   not their first week. Onboarding is judged against this and nothing else (plan §9.3).
6. **Adding a source is one action** (a URL or a sign-in), and a source that breaks later fails
   visibly rather than going quiet.
7. **A second student needs nothing from anyone.** No hand-edited config, no shared credentials.

Failure modes designed against: trust decay, notification habituation, capture friction, silent
changes.

## The product

- **A Windows desktop app**, Rust + Tauri, OS-native webview. No web app and no mobile app, and nothing
  builds toward either; the account carries a student between desktops. Base app under 50 MB, ideally
  under 20.
- **One edition: $9.99 a month, an account required**, no free tier. An academic-year price is kept as
  an option; billing pauses June to August. The deterministic engine still keeps today's page on the
  screen when the network is down.
- **Onboarding creates the vault.** The app makes it under `%USERPROFILE%\Knowlu\`; nobody picks a
  folder. Signing in is the first step of onboarding.
- **Minimize the AI surface.** Most capabilities need no model. Classification over generation,
  schema-constrained output on every call, and a rule-promotion loop that retires model calls into
  deterministic rules as patterns repeat (plan §2.2–§2.3). The model's job keeps shrinking to the
  residue rules cannot decide.
- **Judgment is a service.** Extraction and classification run in our cloud, each kind pinned to
  one named model at one named zero-retention host, with no fallback routing, and logged by id. Nothing model-shaped ships in the app. **Fetch on
  device, think in the cloud:** the one thing that stays on the device is the fetch that needs the
  student's own portal credentials.
- **Teach once, run deterministically forever.** A browser and a model bootstrap a source once; the
  output is a small readable extraction script every later sync runs. A broken script is a visible
  failure, and its regeneration is a proposal to approve (plan §3.3).
- **Sync replays the journal** to the account's storage. Conflicts surface as proposals, never silent
  merges.
- **The first connector is the LMS `.ics` feed, not an LMS API.** No admin approval, no credential
  handling, scoped to one student, the same for Blackboard and Canvas. Canvas personal access tokens
  are rejected on FERPA grounds (market doc §4.3–§4.5).
- **Knowlu only ever reads** by default. Rare writes, such as a block on the student's own calendar,
  stay behind explicit approval and a scope the student grants for that purpose.
- **Learning is per student.** Interests decay and are explored, never driven to zero; a declined
  recurring opportunity stays declined across the series.
- **Telemetry** is interaction events and AI corrections under the terms, keyed to a pseudonymous
  account id, read only as aggregates with a minimum cohort. Raw content only behind a separate,
  revocable opt-in, and never content derived from the Gmail API.

## Standing rules

- If only the student is affected, Knowlu acts. If another person can see the outcome, or the
  student's schedule changes, it proposes and waits for approval.
- One daily email at most. Toasts only for time-critical-and-at-the-desk. Ambient first.
- Failures are visible; silence is never ambiguous.
- Every automation is editable text: nothing lives only in a UI or only in a chat transcript.
- **Credentials live in the OS keychain**, never in the repo, a log, or anything a model prompt
  touches. **Portal credentials never leave the student's machine**; the cloud may hold the account's
  own secrets (session, LMS feed URL) encrypted, never a portal password.
- **Data minimization is a rule:** collect the least that answers the question.
- **No single-user assumptions.** Anything that would need hand-editing for a second student is a bug.

## Open decisions

Each forecloses something if decided carelessly. Decided questions are stated above as they stand.

| Decision | Status |
|---|---|
| Where a student's class schedule comes from (onboarding question, registrar `.ics`, their Google Calendar) | Open; the commitment model needs it before "never overlap a class" has data |
| Writing back to Google Calendar | Not now; needs the `calendar.events` scope, a new consent and a privacy-page update |
| A mobile or web client | No for now; revisit the retention hypothesis (plan §6.3) after the pilot |
| Co-founder | Unowned, and the only open decision with no next action |
| Legal read on scraping and minors | Landscape written (`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md`); a lawyer before the first non-founder paid sign-up |
| Making the code public | Not before the fixtures are replaced with synthetic vaults; they carry real coursework titles |
