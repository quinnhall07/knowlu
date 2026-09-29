# Knowlu in the cloud — design

**Status: SIGNED 2026-09-09.** Quinn confirmed §1 (all twelve decisions) on 2026-09-09; §11 was ruled the same day — R3: keep both the academic-year price and the June–August pause; R1, R2, R4, R5, R6, R8 taken at their defaults (yes); R7 later — and §10 was applied to `VISION.md` in the same commit. Written 2026-09-09 as a draft. This spec redirects the product per Quinn's decisions of 2026-09-09 (§1) and supersedes, where they conflict, the Knowlu independent-app design (2026-09-04), the friends-shell design (2026-09-05), plan 3a's runtime tasks, and the plan-2/3/4 queue. It amends `VISION.md` (§10) and the product plan's open decisions. **Everything below that is not marked *decided* is a recommendation for Quinn to confirm or change.**

**Sources:** `VISION.md`; `docs/superpowers/specs/2026-09-04-knowlu-independent-app-design.md`; `docs/superpowers/specs/2026-09-05-knowlu-friends-shell-design.md`; `docs/superpowers/notes/2026-09-01-product-and-business-plan.md` (§2 AI architecture, §7 telemetry, §8 legal, §12 rulings); `docs/superpowers/notes/2026-09-01-market-pricing-and-distribution.md`; `docs/superpowers/notes/2026-09-09-knowlu-cloud-legal-landscape.md` (the legal briefing — §9 here summarises it and corrects one finding against Google's current pages); Google's *API Services User Data Policy*, *Workspace API User Data and Developer Policy* (2026-07-22), Gmail scope classification and restricted-scope verification pages, all read 2026-09-09.

---

## 1. Decisions of record (Quinn, 2026-09-09) — *decided*

| # | Decision | Consequence |
|---|---|---|
| D1 | **Build on the cloud, both meanings, CI first.** Releases are built and signed by CI; the backend is hosted. | Plan C0 (CI) is the first plan. The laptop leaves the release path; the updater keypair is regenerated and exists only in CI secrets. |
| D2 | **No free tier. One edition. $9.99/month. An account is required.** | The entitlement seam becomes "subscription active", checked at launch and cached; the "free tier runs with zero cloud calls" line in VISION is struck. |
| D3 | **All three judgments — enrichment, event verdicts, email triage — run in the cloud. No local runs.** Cheapest models that do the job. | Plan 3a's llama.cpp runtime, install UI and digest table are removed (C4). The judge *seam* stays; its tier 3 becomes an HTTP call to our service. |
| D4 | **The deterministic, offline-capable core stays in the app; everything that depends on the internet moves to the cloud — as much as possible.** *(amended 2026-09-17 — see the amendment at the end)* | The app keeps: vault, ranking, capacity, rendering, journal, scheduler, the deterministic parsers' *input fetch* for credentialed portals (§4.3). The cloud gets: accounts, entitlement, judgment, email ingestion, event feeds, sync, analytics, updates. |
| D5 | **Analytics:** (a) interaction events and (b) corrections to AI judgments, collected automatically under the ToS; (c) raw content only behind a separate opt-in; issue reports carry diagnostic context. | §6. Gmail-derived content is excluded from (c) by Google policy regardless of opt-in (§9). |
| D6 | **Deterministic measures:** rule promotion, an eval suite from corrections, grammars on every call, pinned models with logged prompt hashes. | §5.4. |
| D7 | **Cloudflare + Supabase.** No Firebase. | §3. |
| D8 | **A new repository, `knowlu`**, with fresh history; `quinn-ops` archived. | §7 — the cut. |
| D9 | **The vault is wiped, not migrated; Quinn re-onboards fresh at the repo cut.** Real users' vaults are plain folders the app creates. **Obsidian is dropped.** | §4.1, §7.3. Git leaves the product (C3). *(amended 2026-09-17 — see the amendment at the end)* |
| D10 | **Plan 2 Tasks 9–10, the three-day protocol, plan 3b, 3c and 4b are cancelled**; plan 2 Tasks 11–13 are executed as the repo cut. | Ledger ruling R-P2B-10. |
| D11 | Portal scraping (zyBooks, VHL) **stays on the device** — Quinn accepted this exception to D4 on the legal briefing's finding. | §4.3 "fetch on device, think in the cloud". *(amended 2026-09-17 — see the amendment at the end)* |
| D12 | **Email ingestion is Gmail OAuth (`gmail.readonly`), server-side.** Quinn chose it over forwarding on 2026-09-09 knowing the cost: Google restricted-scope verification plus an annual CASA assessment before more than 100 users, and 7-day tokens until then. | §5.3. The verification track starts in C1 (it needs the site, the privacy policy and a verified domain) so C2 can ship it. |

---

## 2. What the product is now, in one paragraph

Knowlu is a $9.99/month Windows desktop app with an account. On the student's machine it keeps a plain-text vault it created itself, ranks the day deterministically, renders the page, and runs two slots a day. Everything that needs the internet or a model is a service we run: the account and its subscription, the judgments (what course is this, how long will it take, does this event matter, is this email a task), the Gmail account it reads with the student's permission, the event feeds we fetch, the journal we sync, the analytics we aggregate, and the releases we sign and serve. The app is a thin, honest client of that service — and it still shows today's ranking on a train with no signal, because the ranking never needed the service.

**Three VISION commitments, restated for this shape:** (1) *the engine is deterministic; judgment enters it as data* — unchanged, and the judgment now arrives over HTTPS instead of from a child process; (2) *the source of truth is plain text on the user's machine; the cloud judges, syncs and bills — it never owns the vault* — amended from "local-first, not a cloud service"; (3) *the AI never produces anything the user sends externally* — unchanged.

---

## 3. Architecture

```
student's PC                                              knowlu cloud
------------------------------------------------------    ------------------------------------------------------
knowlu.exe (Tauri)                                        Supabase
  console page (read model from the engine)                 Auth: email + password / magic link
  tray + scheduler (two slots a day, DST-aware)             Postgres: accounts, entitlements, judgments,
  sync client: journal up, fields/verdicts down  ------>      corrections, events, issues, rules, evals
  update check  --------------------------------------.     Storage: journal snapshots (encrypted)
knowlu-engine.exe (child process per step)            |     Edge Functions: /judge/*, /ingest/*, /gmail/*,
  coursework: fetch zyBooks/VHL with LOCAL credentials|       /events, /sync/*, /telemetry, /issues, /entitlement
     raw payload  --> POST /ingest/coursework  -------+--> Stripe: subscription, Customer Portal, Tax
  rank: deterministic; reads fields the sync wrote    |   Inference provider: zero-retention API, pinned models
  write/journal: every note write journaled first     |   Google: Gmail API (gmail.readonly, token held server-side)
                                                      |   Cloudflare Pages: site, privacy, releases/, latest.json  <--'
%USERPROFILE%\Knowlu\<Profile>\   the vault (plain files)
%USERPROFILE%\Knowlu\Backups\     local snapshots
%LOCALAPPDATA%\knowlu\            settings, logs, cache
```

### 3.1 What runs where — the rule and its exceptions

**Rule (D4):** if a step needs the internet or a model, it runs in the cloud. If it can run with the machine unplugged, it runs in the app.

| Concern | Where | Why |
|---|---|---|
| Vault (tasks, approvals, courses, state, config) | device, plain files *(amended 2026-09-17 — see the amendment at the end)* | VISION commitment 2 as amended; the user can open, edit, back up and leave with it |
| Ranking, capacity, `today` rendering, journal, IDs, frontmatter surgery | device (`knowlu-engine`) | deterministic, offline; the engine is already this |
| Scheduler (slots), tray, notifications | device | offline; a slot with no network still ranks and still shows yesterday's judgments |
| LMS calendar (`.ics` URL) fetch + parse | **cloud** (`/ingest/ics`) | needs the internet, no credential (a capability URL the user pastes once into their account, stored server-side encrypted) |
| zyBooks / VHL **fetch** | **device** (`knowlu-engine coursework`) | credentials must not leave Credential Manager (D11, §9) |
| zyBooks / VHL **parse + reconcile** | cloud (`/ingest/coursework`) receives the raw payload | "think in the cloud": one parser to fix when a vendor changes markup, no app release |
| Event feeds (ICS/Localist/Engage/HTML) fetch + roster | **cloud** (`/events`) | internet; HTML sources finally work — the server can run a headless fetch the desktop never could |
| Enrichment, event verdicts, email triage | **cloud** (`/judge/*`) | D3 |
| Gmail read (OAuth) | **cloud** (`/gmail/*`) | D12; the restricted-scope token lives server-side, encrypted, and never on the device |
| Rule table (tier 2), promotion, evals | cloud | D6; rules are per-account rows, applied server-side before any model call |
| Analytics ingest + aggregates | cloud | D5 |
| Issue reports | cloud | D5 |
| Updates | Pages + the in-app updater (already built) | D1 |
| Entitlement | cloud decides, device caches (72 h grace) | D2; a dead connection never hides today's page |

### 3.2 The judge seam, kept

Plan 3a's `judge::judge_task(item, heuristics, rules, model)` and its three tiers stay as the *shape*: tier 1 heuristics run on the device (course map, vendor effort — they are deterministic and offline); tiers 2 and 3 run in the cloud behind `POST /judge/task`, `/judge/event`, `/judge/email`. The engine's `Model` trait gets one production implementation, `CloudModel` (`ureq`, bearer = the account's session token, one call per item, `CALL_TIMEOUT` kept), and the scripted fakes in the tests keep working unchanged. `quinn-ops judge` → `knowlu-engine judge` still **always exits 0**: no network, no entitlement, a 5xx — all normal outcomes reported on stdout, never an amber tray.

**What moves out of the engine:** `runtime.rs` (the process), `inference.rs` (download/install/digest table), the settings row and the wizard offer. **What stays:** `judge.rs`, `judgelog.rs` (now also the client-side buffer for (b)), `enrich.rs` (the pass), `write::propose_amendment` (judge-once and amend cards are unchanged — a cloud judgment that disagrees with a field Quinn set still comes back as a card).

---

## 4. The device

### 4.1 Installation creates the vault — *decided (D9)*

The wizard no longer asks for a vault folder. On first run the app creates:

```
%USERPROFILE%\Knowlu\
  <Profile name>\          the vault: tasks/ approvals/ archive/ courses/ info/ issues/ profile/ state/ config/
  Backups\<Profile name>\  rolling local snapshots (default; the setting stays re-pointable)
%LOCALAPPDATA%\knowlu\     profiles.json, profiles\<id>\{settings.json, seen.txt, logs\}, updates\, cache\
```

- `%USERPROFILE%\Knowlu` is **visible** (VISION: every automation is editable text) and **not OneDrive-redirected** (`Documents` is, which is how a live token reached Microsoft's cloud on 2026-09-09). A tray item **Open vault folder** points at it.
- The vault is **not a git repository** and the app never assumes git exists (C3 removes `history.rs`'s git use). The journal is the history; snapshots are the rollback; the cloud copy is the disaster copy.
- **Uninstall never deletes `%USERPROFILE%\Knowlu`.** A separate *Delete my data* action removes the vault, the snapshots, the app data, and calls `DELETE /account` (which is also the access/deletion right §9 requires).
- Two Windows users on one PC get two `%USERPROFILE%`s and therefore two vaults with no code. Two *profiles* under one Windows user (a friend demoing on your machine) stay supported by `profiles.json`; the second one lands at `%USERPROFILE%\Knowlu\<Other name>\`.
- `config/runners.yaml` is written at birth with `scheduler: app` and this machine's `device:` (friends-shell decision 4) — unchanged.

### 4.2 Onboarding — the new wizard (C1)

1. **Sign in or create account** (email + password, or a magic link; 18+ attestation checkbox; ToS + privacy policy acceptance logged with version and timestamp).
2. **Subscribe** — Stripe Checkout in the system browser; the app polls `/entitlement` until active. (A 7-day trial is a *recommendation*; see §11.)
3. **LMS calendar URL** — pasted once; validated by `/ingest/ics` (fetches it, reports the course count); stored server-side, encrypted.
4. **Coursework logins** (optional) — zyBooks / VHL into Credential Manager, as today; a first fetch runs on the device and its payload goes to `/ingest/coursework` to prove the round trip.
5. **Connect Gmail** (optional) — Google sign-in in the system browser requesting `gmail.readonly` only; the token lands in our cloud, never on the device (§5.3). Until Google's verification is complete the step says so in one sentence (test users only, re-connect weekly) rather than pretending.
6. **Slots and timezone**, as today.
7. **Done** — the first slot runs immediately, and the first page shows today's ranking: VISION success criterion 5, first session.

No step asks for a folder. The one-sentence honesty line ("no Gmail/events/enrichment yet") from plan 4a goes away because those are now the service.

### 4.3 Fetch on device, think in the cloud — *decided (D11)* *(amended 2026-09-17 — see the amendment at the end)*

For zyBooks and VHL the device keeps doing exactly what `coursework.rs` does today up to and including the HTTP fetch with the student's own credentials (cookie jar, CAS ticket, `m3a` host — all of it), then **stops before parsing**: the raw assignment JSON / dashboard HTML goes to `POST /ingest/coursework` with the account token, and the response is the reconciled item list the engine writes into the vault through `write` exactly as `sync_coursework` does now. The deterministic parsers move server-side unchanged (they are pure functions over payload + config); the frozen `zybooks-parsed-reference.json` / `vhl-parsed-reference.json` become the service's own oracle tests. A vendor markup change becomes a server deploy, not an app release. Credentials never leave the machine, and our servers never hold a portal password — which is the whole point (§9 Q6).

### 4.4 What the app stops doing

Git commit/push/rebase (`history.rs`); the model runtime; the free-tier/entitlement-less path; the vault-folder picker; the `--vault` argument on the shortcut (the profile knows); the `scheduler: script` mode (there is no script).

---

## 5. The cloud

### 5.1 Accounts and entitlement (Supabase Auth + Stripe)

- **Identity:** Supabase Auth, email + password with email confirmation, plus magic-link sign-in. No social login at launch (one path to test, one path to support). Sessions are JWTs the app stores in Credential Manager under `knowlu/<profile_id>/session`; refresh is the SDK's; sign-out revokes.
- **Account row:** `accounts(id, email, created_at, tos_version, tos_accepted_at, privacy_version, age_attested_at, stripe_customer_id, status)`. **No birthdate, ever** — the 18+ attestation is a boolean with a timestamp (§9 Q2).
- **Entitlement:** `entitlements(account_id, plan, status, current_period_end, source)` written only by the Stripe webhook handler. The app calls `GET /entitlement` at launch and every 6 h; the answer is cached in `settings.json` with a **72-hour grace** so a dead hotel Wi-Fi never blanks today's page; past grace the page shows a banner and the slots keep ranking but the cloud steps are skipped as named steps (`judge (skipped: no entitlement)`), never a failed slot.
- **Billing:** Stripe Checkout (hosted) for sign-up, Stripe Customer Portal for cancel/update card, Stripe Tax for Kentucky and any other registrations. A cancel link is in the settings panel and in every billing email (§9 Q8). Summer auto-pause (VISION) is *recommended* as a Stripe pause_collection on a June–August schedule; **confirm** (§11).
- **Deletion right:** `DELETE /account` cascades: entitlement cancelled at period end via Stripe, judgments/corrections/events/issues rows deleted, storage objects deleted, a tombstone `(email_hash, deleted_at)` kept 90 days for abuse control, then gone.
- **Anti-cracking, honestly:** the client is inspectable and always will be. What cannot be copied is server-side: the entitlement check every 6 h, the prompts and grammars (never shipped in the binary from C2 on), the parsers for credentialed portals (§4.3), the rule table, the sync. A patched client with no valid session gets no judgments, no ingestion, no sync — which is to say, it gets the deterministic ranking of an empty vault.

### 5.2 The judgment service (`/judge/*`)

One edge function per kind, one HTTP call per item, all three sharing a pipeline:

```
request {account, kind, item(by value: title, body[:1200], source, due, course?, …), heuristics_seed}
  → rules(account, kind).lookup(item)          tier 2 — per-account promoted rules + global rules
  → if incomplete: model(kind).judge(prompt, grammar)   tier 3 — pinned model, GBNF/JSON-schema constrained
  → validate (schema, confidence floor 0.6, known-course check against the account's course list)
  → log judgments(account, kind, item_id, tier, outcome, confidence, fields, model, prompt_hash, ms)
  → reply {verdict, tier, confidence}
```

- **Models:** per kind, pinned by exact id in a `models` table (`kind → provider, model_id, prompt_version, grammar_version, since`). Start with the cheapest tier that passes the eval suite (§5.4) — Haiku 4.5-class for all three; the eval decides, not taste. Changing a pin is a migration row with a date, so every historical judgment names the model that made it.
- **Determinism where it can exist:** temperature 0, constrained decoding, prompt hash logged; the eval suite re-runs on every prompt or pin change and blocks the deploy on regression.
- **Data minimisation, restated for the cloud:** the request carries what the prompt needs and nothing else (`config/ingest.yaml` never leaves the device — the 3a test survives as a client test); the log holds ids, field values, confidences and the four promotion features — **never the body**. Bodies are held in memory for the call and discarded; the provider is under zero-retention terms.
- **Rate and cost guards:** per-account daily caps per kind (enrichment 200, events 300, email 500 — all far above real use), a per-call timeout, and a monthly inference budget alert per account and global.
- **The engine's side:** `CloudModel` implements `judge::Model` and the two new traits (`EventModel`, `EmailModel`) by calling these endpoints; `knowlu-engine judge` is unchanged in shape and still exits 0. The heuristics tier stays on the device because it reads the vault; the seed it produces travels with the request.

### 5.3 Gmail — OAuth, server-side — *decided (D12)*

- **Scope:** `https://www.googleapis.com/auth/gmail.readonly` and nothing else at connect time. Calendar write scope is requested separately, only when the user approves their first calendar-event card (VISION: rare writes behind explicit approval); it is a *sensitive*, not restricted, scope and does not enlarge the assessment.
- **Where the token lives:** the refresh token is exchanged server-side and stored in Supabase Vault, encrypted, keyed to the account; the device never sees it. Disconnect revokes at Google and deletes the row.
- **What the service does, twice a day per account (aligned to the student's slots):** list messages from the last 7 days across the mailbox (the routine's "both the Crimson label and the personal inbox" rule generalises to every label, with user-excludable labels), skip any `gmail:<message-id>` already in the account's seen set, fetch each remaining message's headers and text part, run `/judge/email` under the five tiers (clear task → note; borderline / dated event / opportunity → proposal; information → dropped, with the noise definition), queue the results for sync, and **discard the message text** — the judgment row keeps the message id, the tier, the fields written and the confidence, never the body. Attachments are never fetched.
- **Limited Use, as engineering:** Gmail data is used only for this feature, which is prominent in the UI (the Decisions deck and the page name the source); it is transferred only to the inference provider to provide the feature, disclosed on the consent screen and in the privacy policy; no human reads it without the user's documented consent (the issue-report preview is that consent, and it scrubs by default); **Gmail-derived rows are flagged `origin = gmail_api` and the training export for opt-in (c) filters them out** — a test proves it. Per-user rule promotion on Gmail features is allowed ("that specific user's personalized model") and is the only learning done on it.
- **The verification track (Quinn-owned, long lead):** the Google Cloud project's consent screen needs a verified domain (`knowlu.com`), a homepage and a privacy policy at that domain, a scope justification and a demo video — all C1 deliverables — then restricted-scope verification ("several weeks") and the **CASA security assessment** (we access Gmail data through our servers, so no exemption applies): Tier 2 by an authorised self-scan where Google assigns it, a lab otherwise; renewed every 12 months from the Letter of Assessment. Until verification lands the app is in *Testing*: at most 100 test users, a tester warning screen, and refresh tokens that expire after 7 days — so the wizard's Gmail step says exactly that and offers to re-connect, and the pilot's Gmail feature is limited to named test users.
- **Campus accounts:** a Workspace admin can block third-party apps for `@crimson.ua.edu`; the wizard recommends the personal account (which is where Crimson mail already lands for anyone who forwards it) and shows *source went quiet* if a connected account stops yielding for 14 days.
- **Dedup and budget:** `gmail:<message-id>` lines in `state/ingest-seen.md` keep working; the 15-proposals-a-day cap is the engine's, and the service counts what it has queued so it never proposes past it.

### 5.4 Deterministic measures — *decided (D6)*

1. **Rule promotion, server-side.** Nightly per account: a feature (`source`, `organizer`, `title_prefix`, `series`; for tasks `created_by`+`title_prefix`) whose model verdicts agree ≥ 3 times with no disagreement in 60 days becomes a `kind: rule` proposal delivered by sync into `approvals/`; approval (in the deck, as any card) writes the rule row; tier 2 then answers without the model. Global rules (same pattern across ≥ 20 accounts on the same campus source) are promoted by us, reviewed by hand, and versioned.
2. **The eval suite.** Every correction (b) is a labelled example: `(request, our verdict, the user's value)`. The suite is the last 90 days of corrections per kind plus a frozen seed corpus — **the archived `quinn-ops` vault's seven course notes, ~420 event verdicts and the journal's amend/override records, read once from the archive by a C2 task** — replayed against the pinned model on every prompt/pin change; a regression past a per-kind threshold blocks deploy.
3. **Grammars on every call**, JSON-schema-constrained where the provider supports it, GBNF otherwise; a reply that fails validation is a low-confidence outcome, never a write.
4. **Reproducibility:** model id + prompt version + grammar version + prompt hash on every judgment row; a judgment can be re-run and compared.

### 5.5 Sync (C3) *(amended 2026-09-17 — see the amendment at the end)*

- **Up:** the journal is already an append-only, per-day, `ts`-ordered ledger. The client uploads new journal records (and the note text they produced) to `/sync/push`; the server stores them per account in Storage (encrypted at rest; client-side encryption with an account-derived key is *recommended* for note bodies — **confirm**, it costs server-side search and nothing else we need today).
- **Down:** the service's own writes (judgment fields, Gmail-derived notes, event verdicts, rule proposals) are queued as journal-shaped records the client pulls at `/sync/pull` and applies through `write` — so every cloud write is journaled on the device exactly like a local one, with `actor: agent:knowlu.<kind>` and judge-once intact.
- **Second device:** replaying the journal onto an empty vault reconstructs it — the migration script already proved the shape. Conflicts (two devices editing one field offline) surface as amend cards, never silent merges (VISION).
- **Backups:** the local snapshot tick stays; the cloud copy is the durable one. "Restore from cloud" is a wizard entry point.

### 5.6 CI and releases (C0) — *decided (D1)*

- `.github/workflows/ci.yml`: on PR/push — `cargo test --workspace` on `windows-latest`, 0 warnings enforced (`RUSTFLAGS=-D warnings` with the one `.rsrc` linker line allowed by grep), the static-assets and no-console guards, `cargo bloat` size gate for the engine (< 6 MiB).
- `.github/workflows/release.yml`: on tag `v*` — build both crates, stage the sidecar, Trusted Signing via `signtool` with the Azure credentials in repository secrets (OIDC federated login, no stored client secret if Azure allows it for the account), Tauri updater signature with the **regenerated** minisign key held only as a secret, produce the NSIS installer + `.sig` + `latest.json`, publish to Cloudflare Pages (`site/releases/`), attach to the GitHub release. `scripts/release.ps1` becomes the local dry-run of the same steps, never the shipping path.
- The app already has the updater (plan 4a): checks on launch and daily, never mid-slot. Nothing to add but a public key rotation.

---

## 6. Analytics and issue reports — *decided (D5)*

| Class | What | How collected | Consent basis |
|---|---|---|---|
| (a) Interaction events | the existing `uievents::ACTIONS` set (`object_seen`, decide, snooze, tick, view changes …) with `account_id` instead of a device name | the device's `state/events-ui/` ledger, batched to `/telemetry` at each slot; ≤ 1 KB per event; no free text | ToS + privacy policy at sign-up; disclosed plainly |
| (b) Corrections | every human override of a judged field (`journal` records with `via: dashboard` on a field the agent set, amend-card decisions, snoozes of proposals, declined events) — `(item_id, field, ours, theirs, kind)` | derived from the journal at sync; **no note body** | same |
| (c) Raw content | note bodies and event descriptions — for model improvement; **never Gmail-derived text** | a separate settings toggle, default **off**, its own screen listing what/why/how long, logged with policy version, delete-on-revoke; Gmail-derived rows are excluded by the `origin` flag and a test | explicit, unbundled, revocable opt-in |
| Issue reports | *Report an issue* in the tray: last 200 lines of the app + engine logs, app/engine build shas, OS build, profile id, vault shape counts (never names), the current view's payload keys; **shown to the user on a preview screen with tokens/emails scrubbed, editable, then sent** | on demand | the act of sending |

Aggregation rules from the product plan §7 survive: dashboards read aggregates; a slice under a minimum cohort (n < 10) is not shown; nothing in (a)/(b) is a course name or a title. What (a)+(b) answer that we actually wanted: is the ranking agreed with (override rate per field per kind), where do judgments go wrong (correction rate per source/campus), does the morning ritual hold (first-open time, `object_seen` dwell), retention.

**On "as much as legally allowed":** the design collects what has a named use. The legal briefing's cheapest compliant path is *one privacy policy, universal rights, no sale* — that is also the trust posture universities police. If a use appears that needs more, it is added with a policy version bump, not by default.

---

## 7. The repo cut — *decided (D8, D9)*

### 7.1 The `knowlu` repository

Fresh orphan history from the `quinn-ops` `main` HEAD of cut day (secrets and the personal vault never enter it; `PROVENANCE.md` names the source sha). A Cargo **workspace**: `engine/` (crate `knowlu-engine`, lib `knowlu_engine`, bin `knowlu-engine.exe`), `app/` (crate `knowlu`, bin `knowlu.exe`), `cloud/` (Supabase migrations + edge functions, Workers), `site/`, `scripts/`, `docs/{specs,plans,notes,reports,surface,procedures}`, `.github/workflows/`. `.gitattributes`: `* text=auto eol=lf`, `*.ps1 text eol=crlf`, `engine/tests/fixtures/** -text` (the frozen references are bytes; the engine still translates CRLF on every *vault* read/write because users' vaults are whatever they are). One release profile at the root. The full move/stay list is recorded in `PROVENANCE.md` and was accepted by Quinn verbatim on 2026-09-09.

**Stays in `quinn-ops` (archived, read-only, private forever):** the vault and its whole history, the Python reference engine and its 688 tests, the cutover harness, the cloud-routine prompt, every pre-Knowlu spec/plan/report, Obsidian's files.

### 7.2 Cut day (this week), in order

1. The prepared `knowlu` tree passes its gates (workspace builds, both suites green at 0 warnings, fresh-clone check, stale-name grep empty). `gh repo create knowlu --private`; push `main`.
2. Build `knowlu.exe` + `knowlu-engine.exe` from it (release).
3. Quit the running Knowlu. `Unregister-ScheduledTask quinn-ops-local-runner`. Disable the cloud routine's trigger at claude.ai/code/routines (a UI action, by hand, once — the same off-switch the starvation doc named; nothing else about the routine is touched).
4. Run the new build's **current** wizard (plan 4a's): profile "Quinn Hall", vault folder `%USERPROFILE%\Knowlu\Quinn Hall` (the picker still exists until C1 removes it), LMS URL, zyBooks/VHL logins, backup `%USERPROFILE%\Knowlu\Backups`, slots 12:00/18:00. The vault is app-scheduled from birth. First slot runs; today's page shows.
5. Repoint the Start-menu shortcut at the new `knowlu.exe` (no `--vault`); autostart re-registers itself on launch.
6. `quinn-ops`: remove the worktrees, leave the checkout on `main`, `gh repo archive`. Delete `%LOCALAPPDATA%\quinn-ops\{dual,rehearsal,scratch,shots}` and the old `KnowluBackup\profile_*` mirrors once the new vault has taken two clean slots.
7. Docs: `knowlu/HANDOFF.md` gets the cut recorded; this spec and the legal note move to `knowlu/docs/`; memory updated.

**The judgment gap** starts at step 3 and ends when C2's first endpoint is live: no enrichment, no event verdicts, no email triage in between (Gmail returns for Quinn as a test user as soon as C2's reader exists; for everyone else when verification lands). Coursework, LMS ingestion (still on-device until C1 moves it), ranking, capacity and the page all work. Accepted by Quinn 2026-09-09.

### 7.3 What is deliberately lost, and where it survives

Hand-written course notes with grade weights, ~420 event verdicts, every correction in the journal, `config/planning.yaml` and `week_template.yaml`, `profile/*.md`, `course_map` pins, task `progress` hours. All of it is in `quinn-ops`'s history. §5.4 reads the first three out once as the eval seed; the rest is re-entered through onboarding and daily use, which is the point of onboarding fresh.

---

## 8. Testing and oracles

- **Engine:** unchanged guards — `oracle.rs` (golden `today.md` on three fixture vaults), `surface_oracle.rs` (three surface references), `no_console.rs`, `dependency_boundary.rs` (now also forbids `tauri` and any cloud SDK in the engine; the engine talks HTTP with `ureq` and nothing else), the eight Python-written references frozen, the three Rust ones regenerable only with a reviewed diff. `CloudModel` is tested against a loopback `TcpListener` (the 3a rule: `127.0.0.1` only, joined before return); no test reaches the network.
- **Service:** the moved parsers keep their oracles (`zybooks-parsed-reference.json`, `vhl-parsed-reference.json` become fixture tests of `/ingest/coursework`); every edge function has a request/response contract test; the eval suite (§5.4) is a CI job with its own threshold file.
- **App:** `static_assets.rs` pins the wizard's step set and the absence of any `http(s)://` in static files; `scheduler.rs` tests the skip-as-named-step behaviour for `no entitlement` exactly as for `no ics_url`.
- **End to end:** a scratch profile against a **staging** Supabase project (the same edge functions, a test Stripe key, a sink inbound address) — `scripts/scratch-vault.ps1` grows a `-Staging` switch. Never the production project from a dev machine.
- **Line endings:** `git ls-files --eol` is a CI check: `.rs/.md/.toml/.json` are `i/lf`, `.ps1` are `i/crlf`, fixtures are `attr/-text`.

---

## 9. Legal — what the briefing forces, and one correction

The full briefing is `docs/superpowers/notes/2026-09-09-knowlu-cloud-legal-landscape.md` (943 lines, sourced). It is a landscape for a lawyer, not advice. Design consequences, by area:

| Area | Rank | What this design does |
|---|---|---|
| **Gmail restricted scopes** (`gmail.readonly` **and** `gmail.metadata` are restricted; only `gmail.labels` is not) | **blocking for Gmail beyond 100 users — a long-lead track, not a stop** | **D12: Gmail OAuth, server-side, Quinn's choice.** Restricted-scope verification (weeks; needs the verified domain, homepage, privacy policy, scope justification, demo video — C1) then the annual CASA (we access Gmail data through our servers, so no exemption). Until then: Testing mode, 100 test users, 7-day tokens, said plainly in the wizard. **Correction to the briefing:** Google's restricted-scope page (read 2026-09-09) says the assessment applies to "every app that … has the ability to access data from or through a third-party server" and that on-device-only apps are exempt — the briefing said the exemption was gone. Irrelevant to this design, recorded so nobody builds on the wrong sentence. |
| **Workspace API policy 2026-07-22** — no training "beyond that specific user's personalized model"; Limited Use; no human reading without consent | binding on all Gmail data, always | `origin = gmail_api` rows are excluded from (c) by the export filter and a test; per-user rule promotion is allowed; message text is discarded after judgment; the issue-report preview is the documented consent for any human look; the inference provider is the only transfer and is disclosed. |
| **Portal scraping** (zyBooks / VHL ToS forbid credential sharing; *Van Buren*, *Power Ventures*; Ala. Code § 8-38 SPII) | blocking if server-side | **D11: fetch on device, never store a portal password server-side.** Rate-limited, stops on a vendor block, the ToS tension is disclosed in the wizard step. |
| **Minors / contracts** (Alabama majority 19; § 26-1-1(f) makes 18-year-olds' contracts binding; Kentucky 18; COPPA < 13) | before widening | 18+ attestation checkbox at sign-up, boolean + timestamp, **no birthdate**. Under-18s are declined with a one-line reason. |
| **Subscriptions** (ROSCA, California ARL 2025, NY GBL § 527-a; the FTC click-to-cancel rule vacated 2025-07) | before first paid sign-up | Stripe Checkout with the terms checkbox, Customer Portal cancel link in-app and in every billing email, an annual reminder email, a 3-year consent log. |
| **Privacy notice, universal rights** (CalOPPA; state comprehensive laws not triggered at pilot scale; Alabama APDPA effective 2027-05-01 at 25,000 consumers) | before any non-founder user | One privacy policy on the site and in the wizard; access / export / correct / delete implemented for everyone (`DELETE /account`, `GET /account/export`); "we do not sell or share" stated and true. |
| **Alabama breach law** (§ 8-38: email+password pairs and OAuth tokens are SPII; 45-day notice) | before widening | We hold email + password hashes (Supabase) and session tokens: encrypt at rest, a one-page incident plan, a named contact. No portal passwords server-side (D11) keeps the SPII surface to the account itself. |
| **Sales tax** | before first Kentucky sale | Stripe Tax; register in Kentucky (6% on SaaS since 2023); get Alabama's answer in writing. |
| **FERPA, SOPIPA-family student laws** | none | FERPA binds institutions, not a student-chosen app; the student laws are K-12. Nothing to build; never market to K-12; never touch an LMS API token (unchanged since day one). |
| **University policies** | later | Never request campus SSO credentials (true today); design for a Workspace admin blocking third-party apps on the campus account (personal-account path, visible *source went quiet*). Read UA's and UK's acceptable-use pages by hand before the pilot widens — they would not render for the fetcher. |
| **Issue reports** | later | The preview-and-scrub screen (§6) is the whole requirement. |

**Questions for the lawyer** are listed at the end of the briefing; the two that matter most before the first paid sign-up are the 18+ gate in Alabama and the subscription cancel flow.

---

## 10. Amendments to VISION.md — a diff for Quinn to approve

```
 ## Design commitments
-2. **Local-first.** Not a cloud service that owns user data. The source of truth is plain text files
-   on the user's machine.
+2. **The source of truth is plain text on the user's machine.** The cloud judges, syncs and bills;
+   it never owns the vault. A user can open the folder, copy it, and leave with it.

 ## Product end state
-- **Local extraction, cloud generation.** Embedding and extraction models stay on the device, because
-  that is where raw personal content is touched; templated proposal generation goes to a
-  zero-retention cloud provider. ~1–2 GB resident locally (plan §2.4).
+- **Judgment is a service.** Extraction and classification run in our cloud against zero-retention
+  inference, grammar-constrained, model-pinned, logged by id. Nothing model-shaped ships in the app.
+  The one thing that stays on the device is the fetch that needs the student's own credentials
+  (fetch on device, think in the cloud).
-- **Sync replays the journal**, encrypted client-side, over storage the user already trusts. Device
-  pairing over accounts. Conflicts surface as proposals, never silent merges (plan §4).
+- **Sync replays the journal** to the account's own storage in our cloud. Accounts over device
+  pairing. Conflicts surface as proposals, never silent merges.
-- **Telemetry is structured events computed on-device**, aggregates only, opt-in, minimum cohort size
-  before any slice is usable. Never raw content — "depersonalized" does not survive a unique course
-  schedule (plan §7).
+- **Telemetry:** interaction events and AI corrections are collected under the terms, keyed to a
+  pseudonymous account id, read only as aggregates with a minimum cohort; raw content only behind
+  a separate, revocable opt-in. Never Gmail-API-derived content, by policy.
-- **A free tier must run with zero cloud calls.** Free is local-only, so the deterministic engine
-  has to be complete on its own. An entitlement seam belongs with local profiles, since both
-  partition the same thing: what this installation is allowed to do (market doc §8.4, §10.3).
+- **One edition, $9.99/month, an account required.** The deterministic engine is still complete on
+  its own — that is what keeps today's page on the screen when the network is not — but nothing is
+  free of an account. The entitlement is the subscription, checked by the service and cached on
+  the device with a grace window.
-- **Onboarding and local profiles are near-term.** A per-user vault, settings and credential store,
-  and a first-run flow. Hosted accounts are a *separate, later* decision, needed only for
-  cross-device sync and billing.
+- **Onboarding creates the vault.** The app makes `%USERPROFILE%\Knowlu\<Profile>` itself; nobody
+  picks a folder. Hosted accounts are the first step of onboarding, not a later decision.

 ## Standing rules that must survive every phase
+- **Credentials for the student's own portals never leave the student's machine.** The cloud may
+  hold the account's own secrets (session, LMS capability URL) encrypted; it never holds a portal
+  password.

 ## Open decisions
-| Hosted accounts vs local profiles only | **Local profiles now**; hosted deferred until sync or billing needs one |
+| Hosted accounts vs local profiles only | **Decided 2026-09-09: hosted accounts, required** |
-| Cloud inference provider (zero-retention required) | Not selected |
+| Cloud inference provider (zero-retention required) | Anthropic API, cheapest model that passes the eval suite per kind; provider pinned per kind, swappable behind the seam |
-| Sync: user's own storage vs our encrypted relay | Leaning user's own storage as the default |
+| Sync: user's own storage vs our encrypted relay | **Decided 2026-09-09: our storage (Supabase), per account** |
-| Price: **$9.99/mo, $69.99/academic year**, permanent free tier | Recommended, not committed (…) |
+| Price | **Decided 2026-09-09: $9.99/mo, no free tier.** The academic-year price and the summer pause are open (§11) |
-| The free/paid boundary | **Closed in principle** (…) |
+| The free/paid boundary | Moot: there is one edition |
-| Telemetry final scope | Structured events agreed; scope not set |
+| Telemetry final scope | **Decided 2026-09-09:** §6 of the cloud design |
-| Legal read on scraping and minors | **Not started — highest-priority external dependency** (plan §8) |
+| Legal read on scraping and minors | Landscape written 2026-09-09; a lawyer before the first non-founder paid sign-up |
-| Making this repo public | **Blocked.** Requires credential rotation, then `git filter-repo`, then splitting the vault out of the code repo — in that order (plan §12) |
+| Making the code public | `quinn-ops` stays private and archived; `knowlu` has no secrets in its history by construction (the fixtures carry the founder's coursework titles — replace them with synthetic vaults before any public release) |
```

> **Amended 2026-09-16 (Quinn's ruling): see the Amendment section at the end.** (The "Cloud
> inference provider" row of the "Open decisions" diff above, naming Anthropic, is superseded.)

The "People" amendment, the tutoring/SetNForget/content domains, the success criteria and the other standing rules are unchanged.

---

## 11. Recommendations — ruled 2026-09-09

Quinn's word, 2026-09-09: **R3 — keep both** the academic-year price and the June–August pause ("we'll have to keep it"). Every other row stands at its default.

| # | Recommendation | Ruling |
|---|---|---|
| R1 | Start the Google verification track in C1 week 1 — domain, privacy policy, consent screen, demo video — so it is not on C2's critical path | yes (default) |
| R2 | A 7-day free trial on the $9.99 plan (card up front, cancel any time) — the first-session moment is the retention lever and a wall before it costs more than a week of inference | yes (default) |
| R3 | Keep the $69.99/academic-year price and the June–August pause as Stripe options | **both kept — Quinn, 2026-09-09.** The pause is the one thing nobody else does; it halves summer revenue and that cost is accepted. Implemented as a Stripe `pause_collection` schedule (C1) |
| R4 | Client-side encryption of note bodies in sync (account-derived key) | yes (default); it forecloses server-side search we do not need *(amended 2026-09-17 — see the amendment at the end)* — **reversed** |
| R5 | Global rules (cross-account promotion on shared campus sources) reviewed by hand before activation | yes (default) |
| R6 | Staging Supabase project + test Stripe from day one | yes (default) |
| R7 | Replace the three fixture vaults with synthetic ones before any public code release | later; noted in `PROVENANCE.md` |
| R8 | Anthropic as the launch inference provider (zero-retention API terms; the eval suite picks the model) | yes (default) **Amended 2026-09-16 (Quinn's ruling): see the Amendment section at the end.** |

---

## 11a. Rulings after signing

| Date | Ruling (Quinn) | Consequence |
|---|---|---|
| 2026-09-09 | **The LMS calendar link is captured by a sign-in window, not pasted.** The wizard opens the campus LMS in a pop-up window; the student signs in there themselves (campus SSO + Duo, in their own session); the app then navigates that window to the calendar's share page itself and keeps **only the calendar link**. No credential is asked for, stored or replayed; the window's session data is discarded after capture. | Amends §4.2 step 3. C1 delivers it behind one swappable module with a feasibility spike first (Quinn present for the SSO login) and the paste-a-link path as the fallback for a campus whose pages defeat it. Standing rule unchanged: never request campus SSO credentials — the student types them into the LMS's own page, never into ours. The university-policy read (§9) must cover the flow before the pilot widens. |
| 2026-09-09 | **The personal calendar is connected first, together with the school one.** Quinn's cut-day note: the current wizard never asks for the busy-time calendar (`calendars:` in `config/ingest.yaml` stays empty) and the first page was empty until a slot ran. The C1 wizard's first connection step is *Connect your calendars*: school (the sign-in window above) and personal. | Amends §4.2: the calendar step precedes coursework logins and Gmail. **C1** delivers the personal calendar by its secret iCal address (validated on the device, stored server-side as a `sources` row of kind `calendar_ics`, written to the vault's `calendars:` so today's engine counts busy time), and every wizard finish runs the first slot at once (§4.2 step 7 — now also true of the pre-C1 app, `scheduler::needs_first_run`). **C2** adds Google sign-in for the same panel — one Google connect for `calendar.readonly` first (a *sensitive* scope: lighter verification, no CASA) and `gmail.readonly` incrementally — with the calendar fetch server-side beside `/ingest-ics`; until C2 lands, the secret address is the path. |
| 2026-09-09 | **The first run after onboarding must land a usable page.** Quinn's first slot on the fresh vault: 28 course-less tasks, no zyBooks or VHL work, four items already past due (one from 2025). Diagnosis: (1) the wizard stores portal logins but never maps the discovered zyBook / VHL section to a course, so `coursework` skips them as "not in config"; (2) Blackboard's gradebook feed carries no course text, and the fresh vault has no `course_map` pins and no `courses/` notes, so every task is `course: null`, effort 1.0, `needs_enrichment: true` (the accepted judgment gap made visible); (3) a first ingest creates every item in the feed's window, including ones due before the vault existed. | Three rulings. **R-OB-1 (C1 + C2):** onboarding maps coursework sources to courses — after the first fetch the discovered books and sections are shown with a suggested course and confirmed by the student; an unknown book on a later run is a proposal, never a silent skip (`/ingest-coursework` reconcile). **R-OB-2 (C1):** the school sign-in window also captures the enrolled course list (Blackboard's own API for the signed-in student) and seeds `courses/` and `course_map` codes; part of the sign-in spike's go/no-go. **R-OB-3 (C2, and the device's `ingest` until then):** a first ingest never creates a task already past due — such items are recorded as seen and archived as `imported-past`, so the first page shows the future. Enrichment (C2's `/judge-task`) remains the answer to course and effort for items the map cannot place. |
| 2026-09-09 | **The school is chosen from a searchable list of every US college and university**, not from two radio buttons. | Amends §4.2 (C1's wizard). **R-OB-4:** the list is derived from the federal IPEDS *Institutional Characteristics* file (NCES, ~6,400 Title IV institutions, public domain): `UNITID`, name, city, state, web host — bundled as a compact static asset regenerated by a script that records the source file and date; a typeahead in the wizard; "my school isn't listed" falls back to free text. The chosen school writes `campus:` (unitid, name, state) into the vault's config; `scaffold::CAMPUSES` becomes the *curated* layer (today UA and UK) that adds event feeds and the LMS kind on top of the list; an uncurated school gets no event feeds and its LMS kind from the sign-in window (or asked). Timezone is suggested from the state, the OS zone as the default. The community `university-domains-list` (MIT) may be merged later for campus e-mail domains. |
| 2026-09-09 | **Quinn's first onboarding is reset**; he re-onboards once the C1 wizard carries R-OB-1/2/4. | The first vault and profile were archived under `%LOCALAPPDATA%\knowlu-pre-cut-2026-09-09\attempt-1\`; until then he has no daily page unless the old runner task is re-registered from its saved XML. |
| 2026-09-09 | **Grades are wanted** from the same signed-in session, later. | Not designed. The same window could read the student's own grade pages; it waits on the policy read and on C1's link capture proving out. Listed in §13. |
| 2026-09-09 | **Cut day proceeds with the current wizard**; the account requirement arrives with C1 and existing installs are adopted in place (sign-in on first launch after C1, no re-onboarding). The current wizard's defaults become `%USERPROFILE%\Knowlu` (vault parent) and `%USERPROFILE%\Knowlu\Backups` (backups) — the folder question itself goes away in C1 (§4.1). | A one-line change on `main` before C1 branches; C1's plan gains an "adopt existing install" task. |

---

## 12. Plans, in order

| Plan | Delivers | Gate to start |
|---|---|---|
| **The cut** (this week) | the `knowlu` repo live; Quinn onboarded fresh at `%USERPROFILE%\Knowlu\`; scheduled task and cloud routine off; `quinn-ops` archived; plan 2 Tasks 11–13 done by construction | the prepared tree passes its gates |
| **C0 — CI release** | `ci.yml`, `release.yml`, Trusted Signing + regenerated updater key in secrets, first tagged release served from Pages, the in-app updater proven end to end from a CI build | the cut |
| **C1 — accounts and the new wizard** | Supabase project (prod + staging), Auth, Stripe subscription + webhooks + Portal + Tax, `/entitlement` with the 72 h cache, the wizard of §4.2 (no folder step, 18+ attestation, ToS/privacy acceptance), `DELETE /account` and export, telemetry ingest for (a) and (b), issue reports with the preview screen, privacy policy + ToS pages on the site, **the Google OAuth consent screen submitted for restricted-scope verification** | C0 |
| **C2 — the judgment service** | `/judge/{task,event,email}` with pinned models and grammars, `CloudModel` in the engine, `/ingest/ics` and `/ingest/coursework` (parsers moved, oracles moved), `/events` (feeds + roster server-side, HTML sources working), the Gmail reader (`/gmail/*`; test users until verification, everyone after CASA) and the email tiers, rule promotion + the eval suite seeded from the archive; **closes the judgment gap** | C1 (needs accounts to key everything) |
| **C3 — sync** | `/sync/push`, `/sync/pull`, journal replay, restore-from-cloud, second-device support; `history.rs`'s git removed; local snapshots kept *(amended 2026-09-17 — see the amendment at the end)* — the order from here is C3′ → C5 → C4 → the pilot | C2 |
| **C4 — removal** | plan 3a's runtime/inference code, the settings row, the wizard offer, `SUPPORTED_RUNTIMES`, the (c)-toggle UI landing, the pilot widening checklist (lawyer sign-off, UA/UK policy read, Kentucky tax registration) | C3 |

Each plan is written with the writing-plans skill from this spec, carries a fidelity ledger against §1's decisions and §10's amendments, and is executed subagent-driven in the `knowlu` repo — except the cut, which is a procedure with Quinn at the machine.

---

## 13. What this spec does not decide *(amended 2026-09-17 — see the amendment at the end)*

The console's visual redesign (parked by Quinn 2026-09-07); mobile; email ingestion by forwarding (a Cloudflare Email Routing inbox — documented as the fallback if Google refuses verification, not built); a second campus's specifics; the co-founder question; the exact per-kind model ids (the eval suite's job); reading grades from the student's signed-in LMS session (wanted — §11a). (The academic-year price and the summer pause were open when this was drafted; both were kept on 2026-09-09 — §11 R3.)

---

## Amendment 2026-09-16 — the inference provider (ruling R8 amended by Quinn)

**R8 is amended.** The launch provider is no longer Anthropic's own API. It is **OpenRouter**,
pinned per request to one named, zero-retention upstream per judgment kind — never routed
dynamically and never allowed to fall back to a second host: **CoreWeave** for the `task` and
`event` kinds, **DeepInfra** for the `email` kind. The two models, their precision and their
prices, as pinned in `cloud/supabase/migrations/20260916000100_provider_swap.sql`:

| kind | model | precision, host | usd / M tokens in, out |
|---|---|---|---|
| `task` | `ibm-granite/granite-4.2-8b` | bf16, CoreWeave | 0.10, 0.15 |
| `event` | `ibm-granite/granite-4.2-8b` | bf16, CoreWeave | 0.10, 0.15 |
| `email` | `qwen/qwen3.5-35b-a3b` | fp8, DeepInfra | 0.14, 1.00 |

Every request carries `provider.order`, `allow_fallbacks: false` and OpenRouter's zero-data-retention
flag; a request the named host cannot honour on every point is refused, not silently answered by a
different host. This costs about **$0.24 a month for a typical student and $0.87 for a heavy one**,
against **$1.73 and $6.44** on the Anthropic pin R8 originally named (§2 and §6 of the scoping note
below).

**§5.2's "zero-retention" claim was not true of the provider R8 actually named.** Anthropic's
commercial API is a self-serve tier that retains prompts up to 30 days by default (up to two years on
a detected violation); it is not on zero-retention terms without a sales conversation this product
never had. That inaccuracy runs wherever this spec repeats the claim — the architecture diagram's
"Inference provider: zero-retention API, pinned models" (§3), §5.2's "the provider is under
zero-retention terms", and the line §10 actually applied to `VISION.md` on 2026-09-09, "our cloud
against zero-retention inference, grammar-constrained, model-pinned, logged by id" (§10, ~line 275)
— and none of those sentences is rewritten here (§10 is a historical diff and the rest of the body
stands as written); this amendment is the correction of record. The claim **is**
true of the pinned endpoints named above, and it is enforced per request rather than assumed from
an account-level setting: every call the adapter sends carries the pinned row's `route` — one
named upstream, `allow_fallbacks: false`, `zdr: true`, `require_parameters: true` — and
`_shared/judge_openrouter.ts`'s `assertPinnedRoute` refuses to send anything the row does not
carry that pin for, before any fetch (whole-branch review I1); CoreWeave's and DeepInfra's own
zero-retention terms cover the upstream that actually reads the text.

This rests on `docs/notes/2026-09-16-inference-provider-and-model-scoping.md` (Quinn's ruling of
2026-09-16 on that note's option 1, option 1 being "per-kind pins on one zero-retention host") and
the four research reports behind it —
`docs/reports/2026-09-16-inference-scoping-{legal-floor,provider-offers,model-capability,host-check}.md`,
the last written after Quinn's ruling to find which host actually serves both picks self-serve —
and `docs/plans/2026-09-16-inference-provider-swap-plan.md`, the five-task plan that carried it out.

**The eval suite still picks the model.** Nothing about how a pin is chosen or validated changes:
`cloud/eval/run_eval.ts` replays the seed against whichever model a `models` row names and blocks a
regression past `cloud/eval/thresholds.json` on any PR that touches a prompt, a schema or a pin —
before this amendment against Anthropic's Haiku 4.5, after it against the two models above, and
against whatever is pinned next.

## Amendment 2026-09-17 — desktop only, the account vault, the relay fetch (Quinn's rulings of 2026-09-17) — **SIGNED by Quinn, 2026-09-17**

Written by the controller from the discussion of 2026-09-17, which began with Quinn's question
whether the design accounted for a student reaching their data from several devices, and ended
with six rulings. Signed by Quinn on 2026-09-17 ("Signed."), the same day, in the session that wrote it; every
plan argues from it, and the paused C3 branch does not resume as written. As with the amendment of 2026-09-16, the body of this spec is
not rewritten: the rows and sentences it supersedes carry a marker pointing here, and this section
is the correction of record.

**Ruling 1 — desktop only.** Knowlu is a Windows desktop app, and that is the product. There is no
web app and no mobile app in the plan; the question "should there be parity across desktop, web and
mobile" is closed as *no*. What a student needs across machines is the same account on every
desktop they use — laptop, office, home — and ruling 2 gives them that. §13's "mobile" stays
undecided and the business plan's "mobile port timing" row stays a hypothesis to revisit after the
pilot; nothing here builds toward either. Consequence: no engine port to a browser or a phone, no
device-pairing flow, and the engine stays on the device (ruling 3). The one door deliberately left
open: because the engine is one Rust crate, a server-run engine for an account with no desktop
remains possible later without undoing anything built now.

**Ruling 2 — the account is the source of truth for the vault.** §11 R4 (client-side encryption of
note bodies, "yes (default)") is **reversed**, and C3's precondition P1 is settled as its option (c):
the service holds each account's notes and journal **readable by the service**, encrypted at rest by
the platform, purged by `DELETE /account`. Quinn's words: privacy matters "as long as it remains
legal and doesn't break user trust", and a promise the product cannot keep across a student's own
machines is not trust. Each desktop keeps a plain-text **mirror** — the folder the app creates
(D9), on which the engine ranks with the machine unplugged; every write journals locally exactly as
today and syncs through the account; two desktops moving one field offline still surface an amend
card, never a silent merge (§5.5's clause, VISION). §3.1's vault row becomes *account (source of
truth) + device mirror*. What goes: the device sync key, the recovery code, the sync switch and its
own screen (C3's P2 screen), the key-generation machinery, "restore with your recovery code"
(restoring is signing in on a new desktop; the mirror fills from the account), and the folder
export — VISION commitment 2's "open the folder, copy it, and leave with it" is struck on Quinn's
word ("it just doesn't make sense to do that anymore"); `GET /account/export` stays as the
data-access right (§9). The local snapshot mirror (`backup.rs`) stays as it is. The published
privacy sentence — `site/privacy.html`'s "No note bodies" bullet and its three twins (the same
sentence at the top of that page, `site/index.html`, and the wizard's `PRIVACY` pinned by
`engine/tests/site.rs` and `app/tests/static_assets.rs`) — is rewritten to say that the service
stores the student's tasks and notes to keep their desktops in step, encrypted at rest and deleted
with the account; the lawyer packet (C1's P5 list) gets that delta. §9's categories are unchanged;
the data inventory gains "notes and journal".

**Ruling 3 — the engine stays on the device, fed and updated by the cloud.** D4 stands: ranking,
capacity, rendering, the journal and the scheduler run on the device and work offline, and today's
list exists with no connectivity because it is computed from the mirror. What the engine is
parameterised by moves to the cloud as data: promoted rules (already server-side, D6), the fetch
plans of ruling 4, and any ranking parameter or semester calendar that today lives in `config/`.
The engine refuses to run a slot without a valid entitlement past the 72-hour grace the app already
caches (a task of C3′ below; today only the app gates). The reason on the record, in Quinn's terms:
the attacker is a person repurposing the app's code to get its full utility without paying, or a
competitor shipping a free copy. With the fetch sequence, the parsers, the rules and every judgment
server-side and entitlement-gated, an orphaned binary ranks a hand-made folder and nothing else.
The engine is not moved to the cloud: there is no measurable load to take off a desktop (a rank is
milliseconds), and moving it would trade offline ranking for a cache without buying protection the
account dependencies do not already give.

**Ruling 4 — the relay fetch; D11 amended.** The standing rule holds: *credentials for the student's
own portals never leave the student's machine* (VISION), and the cloud never holds a portal
password. What moves to the cloud is the **fetch sequence** — the login flow, the URLs, the order —
which today lives in `engine/src/coursework.rs`, `zybooks.rs` and `vhl.rs`. Quinn's words: "we can
execute our fetch scripts (which will grow as we expand capabilities) without having to give the
client access". The device becomes a **credential-substituting HTTPS relay**, and the contract is:

- The cloud composes each request — method, URL, headers, body — with placeholders of the form
  `{{credential:<source>:username}}` and `{{credential:<source>:password}}`. The device substitutes
  from Credential Manager, sends the request from the student's machine with a per-source cookie
  jar, and returns the raw response (status, headers, body) to the cloud. The cloud parses (§4.3
  already put the parsers there) and composes the next step. A run is a short sequence of such
  steps inside one slot, driven by the device's scheduler, so nothing fetches while the laptop is
  closed — exactly as today.
- **Host allow-list per credential, enforced on the device.** A credential saved for a source is
  substituted only into an HTTPS request to that source's registered hosts (zyBooks:
  `*.zybooks.com`; VHL: `www.vhlcentral.com` and `m3a.vhlcentral.com`); any other request carrying
  a placeholder is refused by the device and reported as a run error. The list is a compiled-in
  table in the app, the way `SUPPORTED_RUNTIMES` pins runtimes — the one portal-specific thing the
  client keeps, because it is the guarantee that lets the privacy page say a password only ever
  goes to the site it was given for, and it holds even against our own servers.
- The device knows no login flow, no URL and no parser. Adding a portal or repairing one after a
  vendor changes its markup is a cloud change with no release and no user action; the frozen parsed
  references stay as the server parsers' oracles.
- Sessions: the device keeps each source's cookie jar between slots, so a portal is logged into once
  and re-authenticated only when the session expires; a login that fails becomes one card ("your
  zyBooks password no longer works"), never a retry loop.
- Raw pages are the student's data under ruling 2: received for the run, parsed, and not retained
  beyond it (the parsed rows are what persists; the policy says so in one sentence).
- `coursework-discover` — the wizard's course-mapping read — becomes a relayed cloud job over the
  same contract.
- The legal posture of D11 is unchanged: every request originates from the user's device, with the
  user's credentials, at the user's instruction; the vendor sees the student's own IP and session.

**Ruling 5 — sequence: all of it ships before the pilot.** §12's order from here is: **C3′ — the
account vault** (sync without encryption on the paused branch's tables; the privacy sentences; the
export dropped; the entitlement check in the engine; git leaves the product; `ics_url` leaves the
vault) → **C5 — the relay fetch** (the contract above, replacing the on-device fetchers) → **C4 —
removal** (the local model runtime, as written) → the pilot. The paused `c3-sync` branch's Tasks 1–3
are kept where they fit (the two tables with plaintext columns, the row validators) and retired
where they do not (the envelope, the key, the vectors). Quinn's words: "I want what we've just
decided shipped before the pilot."

**Ruling 6 — what this amendment does not decide.** Web and mobile (parked, with the door of
ruling 1); a server-run engine (not needed while desktop only); the exact relay step protocol, the
per-source host table and the retention window for raw pages (C5's spec); the wording of the
privacy sentence (C3′'s task, read by Quinn and the lawyer before merge, as C3's P2 already
required).

**Markers.** D4, D9 and D11 in §1, §3.1's vault row, §4.3, §5.5, §11 R4, §12's C3 and C4 rows and
§13 carry *(amended 2026-09-17 — see the amendment at the end)*. `VISION.md`'s commitment 2 and its
"Mobile port timing" and "Sync" rows are amended in the same commit; `HANDOFF.md` §2–§4 record the
new stream order and close the parity question. Plans: `docs/plans/2026-09-14-c3-sync-plan.md` is
superseded by a C3′ plan written from this amendment, and C5 gets its own spec section and plan.

## Amendment 2026-09-29 — "What's next?": the product vision refocused (Quinn's rulings of 2026-09-29) — **for Quinn's signature**

Written from a vision interview with Quinn on 2026-09-29; `VISION.md` was rewritten from it the same
day and states the result. As with the earlier amendments, the body of this spec is not rewritten;
this section is the correction of record, and plans argue from it once it is signed.

1. **Desktop only, reaffirmed.** Ruling 1 of 2026-09-17 stands unchanged: Windows desktop, no web or
   mobile app, nothing building toward either.
2. **Nudges.** VISION's "toasts only for time-critical-and-at-the-desk" becomes: at most two nudges a
   day, each for a real risk (a deadline at risk at the current pace, a task repeatedly avoided), with
   quiet hours and a one-click snooze.
3. **Who started a change decides how it lands.** The student asked → act at once with undo. Knowlu
   noticed → propose and wait, showing who it came from and a summary of the context. An
   authoritative source changed → apply and list it. Anything another person can see stays a
   proposal. Knowlu never invents a due date.
4. **The calendar.** Knowlu's own in-app calendar is the complete picture. Later, Knowlu writes a
   dedicated "Knowlu" calendar inside the student's Google or Outlook account and never edits their
   other calendars. Until that piece is specced, the commitment model's Q3 (Google Calendar stays
   `calendar.readonly`) and C9 (no write-back) stand.
5. **Syllabi are uploaded by the student**, at onboarding and each term: class meetings, exam dates,
   grade weights and scale, everything confirmed by the student. This amends the commitment model's
   Q2 ("login-only… typing is a last resort") to "sign-ins and syllabi"; typing stays the last resort.
6. **Audience:** undergraduates first; graduate students are served by the same product.
7. **Grades and GPA** join the product: per-course rings, a term GPA with an uncertainty band, a
   projected cumulative GPA on request; read from the LMS grade page and homework platforms on the
   device and from the syllabus; deterministic maths; never in telemetry; hideable.
8. **The assistant and quick capture** join the product: fixed flows on the smallest model each
   passes its evaluation with, preview cards for every assistant change, planning advice and short
   study tips but never the work itself; a global hotkey that captures a task at once and asks for
   what is missing later. VISION commitment 3 (the AI never writes what the student sends) is
   unchanged.
9. **The MVP** is the founder's quinn-ops feature set plus grades from Blackboard; everything else is
   phased after it (`VISION.md`, "Build order").
