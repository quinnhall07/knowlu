# C5 — the relay fetch — plan review

**Object under review:** `docs/plans/2026-09-17-c5-relay-fetch-plan.md` (3,592 lines, 12 tasks,
status WRITTEN 2026-09-17, committed `141b12d`, not previously reviewed).

**Weighed against:** `docs/specs/2026-09-17-c5-relay-fetch-design.md` (529 lines, committed
`c69ae25`, reviewed); the cloud design's *Amendment 2026-09-17* rulings 1-6 with ruling 4 binding
(`docs/specs/2026-09-09-knowlu-cloud-design.md`); `CLAUDE.md` (no single-user assumptions, the eight
frozen references and the three surface references, `rank` never calls a model, `.no_console()`,
`ureq` with `cookies`, TLS by rustls/ring, 0 warnings, LF everywhere, TDD); `HANDOFF.md` §2's
ownership paragraph; and the checkout at `main` = `0101d8b`.

**Passes made.** (1) The spec read whole, then the plan read whole in eleven contiguous slices.
(2) A citation pass: every line number, symbol name, signature, constant and count the plan states,
checked against the file it names — `engine/src/{coursework,zybooks,vhl,cloudmodel,wincred,info,
ledger,judge,lib,main}.rs`, `engine/Cargo.toml`, `engine/tests/{dependency_boundary,no_console,
site,cloud_contract}.rs`, `app/src/{scheduler,onboarding,account,credentials,inference,lib}.rs`,
`app/static/index.html`, `app/tests/static_assets.rs`, `site/privacy.html`,
`cloud/supabase/functions/_shared/{http,db,entitlement,judge_handler}.ts`,
`cloud/supabase/functions/ingest-coursework/**`, `cloud/supabase/migrations/**`,
`cloud/supabase/config.toml`, `cloud/supabase/deno.json`, `.github/workflows/ci.yml`,
`scripts/wizard-check.py`. (3) A third-party API pass against the vendored sources:
`ureq 3.4.0` (`cookies.rs`, `agent.rs`, `config.rs`, `error.rs`) and `windows 0.62.2`
(`Win32/Security/Cryptography`, `Win32/Foundation`). (4) A security pass following the credential
and the raw page end to end: where a password is read, what encodes it, what can print it, what the
device returns, what the cloud stores, what a log line can carry, and what a redirect can reach.
(5) A dead-code and interface pass: every symbol a later task consumes, traced to the task that
produces it and to the shape it has in the checkout.

Nothing was executed: no `cargo`, no `deno`, no `supabase`, no commit. The report is the only output.

---

## Verdict

**Execute after fix round 1 (findings C1-C5, I1-I14).**

The spine of this plan is sound and unusually well-grounded. §3's host allow-list — the one thing in
C5 that has to hold against our own servers — is carried faithfully and completely: the table, the
five structural refusals, the wildcard matcher's `.`-prefix test, the userinfo and IP-literal and
punycode cases, the refusal that names the host and never the URL, and the integration-test placement
that makes the `cfg(test)` loopback row provably absent from a real build. The substitution table
reproduces `zybooks::quote`, `vhl::quote_plus` and `ledger::dumps_value` exactly where the spec says
it must, adds the header CR/LF refusal, and namespaces both placeholder forms by source with a test
for each half of the exfiltration pair. The session store's two reasons for replaying raw
`Set-Cookie` lines rather than ureq's own serialisation are correct against ureq 3.4.0's actual API,
and the DPAPI sketch compiles against `windows 0.62.2` as written. The deletion task is ordered after
the two port tasks for the right reason. The privacy drafts say what the code will do. Every Quinn
question is asked at the task that needs it, with the plan built to the recommendation meanwhile.

What stops it being *Execute* is that the server-side state machine is underspecified in two ways
that are not tuning and not typos. A `relay_runs` row holds one `plan` and one `plan_version`, the
registry is keyed by source, the handler selects by job, and a coursework run must drive two sources
with interleaved steps and a shared batch budget: nothing in the plan says how one run becomes two
plans (C5). And the accumulated page payloads have nowhere to live between round trips, because the
handler is stateless, `checkCursor` forbids a body in the cursor and Q1 rules zero retention — yet
the plan's own zyBooks test batches a twelve-book shelf across two round trips (C3). Those two
together are a design decision that belongs in the spec's §6 before Task 5 starts, not inside it.
Beside them: the loopback portal harness is placed in a file that cannot see the row it depends on
(C1), the rejected-login pause has no exit and its own test is unsatisfiable (C2), and the ten-minute
run budget is checked only between round trips while the sixty-second step budget is per HTTP request
rather than per step, so the relay can still be the thing the scheduler kills (C4) — which is
exit-gate item 6's stated property, inverted.

All five are bounded. None asks for a different architecture. Fix round 1, then execute.

---

## Findings

### Critical

**C1 — the loopback portal row is invisible to the file that needs it.**
`plan.md:1179-1188` puts `extra_source` (and with it the `{name: "loopback", hosts: &["127.0.0.1"]}`
row) behind `#[cfg(test)]`, and `plan.md:1222-1225` gates the scheme/port/IP relaxations on
`cfg!(test)`. That is right, and `plan.md:955-999` proves it right by asserting from an *integration*
test that `check_host("loopback", ...)` is `UnknownSource`. But `plan.md:1966-1977` then puts the
whole executor harness — every test that aims at a loopback portal — in
`engine/tests/relay_contract.rs`, which is an integration test in exactly the same position:
Cargo compiles the library **without** `cfg(test)` for both files. `relay_contract.rs` therefore sees
precisely what `relay_allowlist.rs` asserts it sees, and every portal test in Tasks 4, 8 and 9
(`plan.md:1979-2067`, `:2871-2935`, `:3025-3056`) fails with `unknown_source`. The file's own module
doc (`plan.md:1975-1977`) states the contradiction out loud without noticing it. The spec is right
and the plan moved away from it: §9's first bullet says these tests live "In `engine/src/relay.rs`'s
own `#[cfg(test)] mod tests`", citing `coursework.rs:3095-3160` as the precedent.
**Fix:** move the executor, redirect, capture, budget and `relay::run` suites into `relay.rs`'s
`#[cfg(test)] mod tests`, as §9 says. Keep `engine/tests/relay_allowlist.rs` exactly as written —
it is the guarantee. Either delete `relay_contract.rs` or reduce it to what is reachable without a
portal (nothing in the current draft is), and drop `Captures::get_for_test` with it: a `pub` test
accessor on a struct that holds captured bearer tokens is a door that should not exist.

**C2 — the rejected-login pause has no exit, and Task 8's own test cannot pass.**
`plan.md:2946-2947` skips "a source that is paused by an open `info` item with
`close_key: login:<name>`", and `plan.md:2955` closes that card only "For each source that
authenticated". A paused source is never offered to the relay, so it can never authenticate, so the
card never closes: the pause is permanent. `info::list_info` (`engine/src/info.rs:108`) returns open
items only, so nothing else re-opens the path either. The card body the plan writes
(`plan.md:2972-2976`) promises the opposite — "Knowlu will not try that login again until you do
[save the password again]" — and nothing on the device notices that the student did. The consequence
is that `a_later_run_that_authenticates_closes_the_card_and_unpauses` (`plan.md:2892-2899`) is
unsatisfiable as specified, and exit-gate item 15(d) (`plan.md:3534-3538`: "save the right password,
run again, and confirm the card closes and the source resumes") cannot be demonstrated. The
credential write the student actually performs is `onboarding::store_credentials`
(`app/src/onboarding.rs:850-856`), which the plan explicitly does not touch (`plan.md:314-316`:
"No `app/src/relay.rs`, no new Tauri command"). **Fix:** name the unpause. The cheapest correct one
is to have the app close the card when a password is saved: `store_credentials` already knows the
vault and the source, and `info::close_info(vault, Some("login:<source>"), None, ...)` through the
engine's `write` with `console_ctx()` is the existing shape. Alternatives, each worse: bound the
pause with `NewInfo::expires` so it lapses after N days, or record the credential's Credential
Manager `LastWritten` beside the card and unpause when it moves. Whichever is chosen, Task 8 gains
the step, the app-side change is named in the ownership list, and Q2's question to Quinn should say
what re-arms the source, because "pause" without an answer to that is "stop forever".

**C3 — accumulated page payloads have nowhere to live between round trips.**
`plan.md:2720-2722` says the `books` stage accumulates `{code, payload}` "into the **payload
accumulator the handler holds, not the cursor**". An edge function holds nothing between
invocations; the only state is the `relay_runs` row, and `checkCursor` (`plan.md:2438-2439`,
`:2424-2430`) refuses any key in `{body, html, payload, page, cookie, token}` and any string over
4 KiB, with Q1 (`plan.md:252`) ruling zero retention anywhere else. Yet `plan.md:2655-2659` asserts
a twelve-book shelf yields eight steps with "the rest com[ing] on the next round trip" — so eight
payloads must survive a round trip with nowhere to be. The same problem bites below eight books:
VHL's three steps are strictly sequential (home to login to dashboard), so `vhl.dashboard`'s HTML
lands in a later batch than `zybooks.book:*` on any shelf that fills a batch, and both are needed by
the single `ingestHandler` call at `plan.md:2487-2496`. **Fix:** decide it in the spec's §6, not in
a task. Two clean options. (a) *Parse incrementally*: call `parseAssignments`/`parseDashboard` per
source as each raw body arrives and keep only the parsed rows in the cursor — parsed rows are not
raw pages and are what R4-19 says may persist — then assemble `done` from them; this costs a
restructure of the `ingestHandler` hand-off, which currently takes raw payloads, so say what
replaces it and how `warnings`/`proposals` stay byte-identical. (b) *One batch or bust*: require
every payload-bearing step of a run to land in one batch, which caps a shelf at `MAX_BATCH` minus
VHL's one and turns a thirteenth book into a named warning. Option (a) is the honest one; option (b)
is a product limit that has to be said out loud. What is not available is the current text.

**C4 — the ten-minute run budget is not a ten-minute bound, and the test that "proves" it compares
two constants.** `plan.md:2266-2271` ends the loop "on `RUN_WALL_CLOCK`" between round trips, and
`plan.md:2187-2214`'s `perform` charges steps and bytes but never time. `STEP_TIMEOUT` is set as
`timeout_global` on the agent (`plan.md:2165-2175`), which in ureq 3.4.0 bounds **one HTTP request**,
and the device issues up to `MAX_REDIRECTS + 1 = 6` requests per step because it follows the chain
itself (`plan.md:2198-2202`). So one step can take 6 x 60 s, one batch of `MAX_BATCH = 8` can take
48 minutes, and the deadline is not consulted once inside it. Add `CALL_TIMEOUT` (120 s, unchanged
at `engine/src/cloudmodel.rs:42`) for the round trip itself. A run that begins its last batch at
9:59 can therefore finish at 50 minutes — well past `scheduler::CHILD_TIMEOUT`
(`app/src/scheduler.rs:27`, 20 minutes), which kills the child, sets `engine_ok = false`, paints the
tray amber and puts the slot into retry backoff: exactly the outcome `plan.md:494-497` says the
design exists to prevent. `the_run_stops_at_its_own_wall_clock_not_the_scheduler_s`
(`plan.md:2058-2066`) asserts only `RUN_WALL_CLOCK < 20 min` and the values of eight constants; it
establishes nothing about behaviour, and exit-gate item 6 (`plan.md:3461-3462`) states the property
as though it did. **Fix:** give `RunBudget` a monotonic `deadline: Instant`, check it before every
step and before every redirect hop (`budget.remaining()` as the per-request `Timeout` rather than a
fixed `STEP_TIMEOUT`), make `STEP_TIMEOUT` a bound on the whole step including its chain, and add a
test that a scripted portal which stalls past the deadline ends the run as a `budget` warning at
under ten minutes of simulated time. Then exit-gate item 6 is true.

**C5 — one run, one `plan` column, two sources: the registry and the selector do not meet.**
`plan.md:2463` has the handler "pick the plan from `plans/mod.ts` by `job`"; `plan.md:2726` defines
the registry as `export const PLANS = { zybooks: zybooksPlan, vhl: vhlPlan }`, keyed by **source**.
The jobs are `coursework` and `coursework-discover` (`plan.md:2245`), so `PLANS[job]` is undefined
for both. The migration reinforces the single-plan shape: `plan text not null, plan_version int not
null` (`plan.md:2370-2371`), one row per run. But the device drives one run covering both sources —
`relay::run(&client, Job::Coursework, &sources, ...)` (`plan.md:2260-2263`), `sources` built from
`["zybooks", "vhl"]` (`plan.md:2944`) — and exit-gate 15(b) (`plan.md:3523-3524`) expects
`zybooks.signin`, `zybooks.items`, `zybooks.book:*`, `vhl.home`, `vhl.login`, `vhl.dashboard` in one
run's logs. Nothing says how the two plans' cursors coexist, how `MAX_BATCH = 8` is divided between
them, whose `{kind: "failed"}` ends the run versus contributes a warning and lets the other finish
(Task 9 step 4 requires the latter for discover, `plan.md:3119-3120`), or what `plan_version` means
for two modules. **Fix:** specify a composite driver. Concretely: `PLANS` keyed by job, with
`courseworkPlan` fanning out to the per-source modules; `cursor.sources = {zybooks: {...},
vhl: {...}}`; a batch filled round-robin across the sources that still have steps; a per-source
`failed` recorded as a warning that retires that source and leaves the other running; and
`plan_version` a stable composite (each source's version, or their hash) so M4's mid-run-deploy 409
still fires. Say it in the spec's §6 and let Task 5 and Task 6 implement it.

### Important

**I1 — the 10 MiB `readJson` cap is below what the device's own budgets let it send.**
`plan.md:538-540` and `plan.md:2460-2461` raise `readJson`'s cap to `10 << 20` "because a `results`
batch carries up to eight response bodies of up to 2 MiB each" — which is 16 MiB before any encoding,
and the encoding is not free: a non-UTF-8 body travels base64 (4/3) and a UTF-8 body travels
JSON-escaped. `MAX_RUN_BYTES = 8 MiB` (`plan.md:2151`) caps a whole run's raw bytes, so the true
worst case for one POST is about 8 MiB raw, which is roughly 10.7 MiB base64 — still over the cap.
`readJson` (`cloud/supabase/functions/_shared/http.ts:48-52`) also measures `text.length` in UTF-16
code units **after** `await req.text()`, so the cap is a sanity bound and not a memory guard, and a
413 mid-run ends the run with `the service refused` on every slot for as long as the vendor's page
stays large. **Fix:** derive the cap rather than choose it. Set it at `MAX_RUN_BYTES` times 4/3 plus
slack (16 MiB), *and* give the device its own outgoing-body bound so it never posts what the server
must refuse — charge the serialised `results` size in `RunBudget` and turn the step that would blow
it into `too_large`. Alternatively lower `MAX_BYTES` to 1 MiB (still about 19x the largest measured
payload) and keep the cap at 10 MiB. Either way,
`a_results_batch_over_the_cap_is_a_413_not_a_500` should pin the relationship between the two
numbers, not one of them.

**I2 — `scrub` knows three encodings and the relay adds a fourth.**
`plan.md:1263-1278` moves `scrub` "byte for byte", and `engine/src/zybooks.rs:434-451` replaces each
secret in three forms: raw, `quote` (percent-encoded), and `json_escape_ascii`. The relay's own form
encoder is `quote_plus` (`plan.md:1651`, from `engine/src/vhl.rs:518`), which renders a space as a
plus sign rather than `%20`. A portal password containing a space, sent in the VHL CAS form POST,
reaches a ureq transport error as `hun+ter` and survives `scrub` untouched — and that string goes
into `warnings`, into the run record, into `state/runner-log.md` and into the Runs view. The module
doc the plan writes claims the opposite in so many words (`plan.md:1128-1130`: "in all three forms a
secret can take"). **Fix:** add `quote_plus(secret)` to `scrub`'s form list when it moves, extend
`scrub_replaces_the_raw_percent_encoded_and_json_escaped_forms` to a fourth case, and add a
form-body transport-failure test with a space-bearing password. This is a pre-existing gap in
`vhl.rs`; C5 is where it stops being incidental and becomes a stated guarantee.

**I3 — the body the device returns to the cloud is never scrubbed of the request's own secrets.**
`plan.md:2211-2214` shapes the reply by stripping `Set-Cookie`, replacing `redact: true` captures and
decoding the bytes. Nothing runs `scrub` over the body. But `Filled::secrets` exists for exactly this
(`plan.md:1560-1562`), and there is a concrete case: VHL's rejected login **serves the login page
again** (`engine/src/vhl.rs:1077`, ported at `plan.md:2793-2800`), and a re-served form commonly
carries the submitted username back in a `value=` attribute. That page travels to our servers
verbatim. R4-1's sentence is about passwords, but a portal username is a credential half, and the
class of failure — a vendor echoing what we posted — is not one the relay defends against at all.
**Fix:** run `scrub(&body, &filled.secrets)` as the last shaping step, after the capture redaction
and before the UTF-8 decode decision, and test it with a portal that echoes the posted form. It
breaks byte-for-byte fidelity only when a secret is literally present in the page, which is precisely
when breaking it is right. Say so in the spec's §2.4, which currently promises "byte for byte".

**I4 — the URL that is checked is not the URL that is sent.**
`plan.md:2189-2191` fixes the order as `check_host(&step.source, &step.url)` then `fill(...)`, for a
good reason (a refused host must never read a credential). But nothing re-checks `filled.url`, which
is what `agent.get`/`agent.post` receives. Today the gap is probably not exploitable — a
placeholder in the authority does not parse as an `http::Uri` authority, and `Slot::Url`
percent-encodes the delimiters, so a substituted value cannot open a new authority from the path —
but "probably not exploitable" is not the standard this module is held to, and neither property is
tested. **Fix:** check twice. `check_host` on `step.url` before `fill` (keep it), then `check_host`
on `filled.url` immediately before the socket opens, asserting the two agree. Add two cases: a
placeholder in the authority is refused, and a substituted value cannot change the host.

**I5 — capture redaction is a text replace, so it can miss a token or corrupt a body.**
`plan.md:2212-2213` replaces "each `redact: true` captured value in the body text" with a marker. The
value comes from a JSON pointer into the *parsed* body (`plan.md:2207-2210`), so its textual form in
the raw bytes may differ — a token containing a non-ASCII character, a slash or a quote is escaped
in the wire text and the raw value will not match, and the token then travels to the cloud with
`captured: ["zybooks_token"]` claiming it did not. In the other direction, a short value (a numeric
id, say) matches elsewhere in the body and corrupts what the parser reads. Since `Capture.from` is a
closed set of one (`plan.md:1542-1545`), the body is already known to be JSON. **Fix:** redact at the
pointer — replace the value in the parsed `serde_json::Value` and re-serialise through
`ledger::dumps_value` — and drop the text replace. If byte fidelity for the non-redacting case
matters (it does: the BOM test at `plan.md:2020-2024`), re-serialise only when at least one
`redact: true` capture fired. Add a test with a token containing a character the serialiser escapes.

**I6 — `the_password_is_exposed_in_exactly_one_place` counts its own source and can never pass.**
`plan.md:1491-1498` does `include_str!("relay.rs")` then
`assert_eq!(src.matches(".expose()").count(), 1)`. The file it reads contains the real call site in
`substitute_one` **and** the same literal inside this very assertion, so the count is 2 and the test
fails on the day it is written — and `plan.md:1493-1494` then invites the implementer to "read the
new call site before changing the number", which is how the pin gets defeated. The correct shape is
already in the plan: `the_engine_holds_no_portal_url_and_no_login_flow` (`plan.md:3160`) scans
`text.split("#[cfg(test)]").next()`. **Fix:** count over the pre-`cfg(test)` half only, and say in
the message that the number is a guarantee and not a tally.

**I7 — three symbols the plan lists under "Stays" lose their only non-test callers, and 0 warnings
goes red.** `plan.md:370-372` and `plan.md:405-407` keep `coursework_request`, `post_coursework` and
(implicitly) `FetchedSource`. After Task 8, `collect_cloud` calls `relay::run` and decodes `done`;
`plan.md:486` states that C5 calls `post("/relay", ...)` and nothing else. So
`coursework::coursework_request` (`engine/src/coursework.rs:639`), `FetchedSource` (`:630`) and the
POST half of `post_coursework` (`:670-676`) are reachable only from `#[cfg(test)] mod tests` —
which suppresses nothing under a plain `cargo build --workspace`, the gate `plan.md:162-164` requires
at zero warnings. **Fix:** split `post_coursework` into the POST wrapper (deleted in Task 10, with
`coursework_request` and `FetchedSource`) and the reply decoder (kept, renamed, reused by
`collect_cloud` on `done`), and correct the "Stays" list. The tests at `coursework.rs:3262` and its
neighbours move with the half they exercise.

**I8 — `sectionMapping` is not an export of `parse_vhl.ts`.**
`plan.md:3115-3117` says "`routeZybook` and `sectionMapping` are the **one** predicate each caller
shares, imported from `parse_zybooks.ts` and `parse_vhl.ts`, never re-implemented". `routeZybook` is
exported (`cloud/supabase/functions/ingest-coursework/parse_zybooks.ts:86`). `sectionMapping` does
not exist there in any form: `parse_vhl.ts` exports `MOUNT_MARKER`, `parseDurationHours` and
`parseDashboard` only, and does the lookup inline (`parse_vhl.ts:81`, `:99`). The Rust original is
`vhl::section_mapping` (`engine/src/vhl.rs:300`), which Task 10 deletes. **Fix:** either export a
`sectionMapping(sections, sectionId)` from `parse_vhl.ts` and have `parseDashboard` use it — a small
edit to a C2 file the plan currently declares read-only (`plan.md:309-312`), so say so — or state
that `vhlRows` calls `parseDashboard`'s own predicate by another named route. Do not let Task 9
discover this at the import line.

**I9 — hand-off H2's code does not compile and hand-offs are applied verbatim.**
`plan.md:648-688` adds a fourth parameter `sessions: &Path` to `slot_argv`, then writes the pushed
step as `crate::scheduler::sessions_dir(&cs_data_dir).to_string_lossy().into_owned()` — and
`cs_data_dir` is a binding that exists nowhere in `slot_argv`, which has no `ConsoleState`
(`app/src/scheduler.rs:339`). The plan's own rule is that a hand-off is "applied **verbatim from the
hand-off text** by the controller" (`plan.md:591-592`). **Fix:** the line is
`sessions.to_string_lossy().into_owned()`; `sessions_dir(&cs.data_dir)` belongs at the single call
site in `run_slot_inner`, which the hand-off already says. Correct the snippet.

**I10 — H6 asks the controller to find a sentence that does not exist.**
`plan.md:815-823` says "The wizard's one-sentence ToS disclosure on the coursework panel gains a
clause. Find the existing sentence (the one `app/tests/static_assets.rs` already pins — read the
test for the literal it looks for)". There is no ToS-tension disclosure on that panel. The panel
(`app/static/index.html:131-141`) carries one lede — "Optional. Stored in Windows Credential Manager
on this machine — never in the vault, never in a backup, and never sent to us." — which is a
credential-storage sentence, not a disclosure that using a portal login this way sits against the
vendor's terms. No test pins such a literal: `the_logins_panel_maps_what_it_finds_to_a_course`
(`app/tests/static_assets.rs:508-527`) pins the mapping block, and nothing in `app/static/` or
`site/` contains "terms of service", "on your behalf" or an equivalent for the portals. The spec's
§8 asserts "The wizard's one-sentence disclosure of the ToS tension stays", which is not true of the
checkout. **Fix:** decide whether the disclosure is being *written* rather than *extended*. If it
is, it is legal copy and belongs with Q6 in front of Quinn and the lawyer, not appended by a
controller; H6 then carries the whole sentence and Task 9's assertion pins both halves. If Quinn
rules that the existing lede is the disclosure, say that in H6 and append to it — but the plan
should not describe a sentence the repository does not have.

**I11 — the session report cannot support "re-authenticate only on expiry" for a cookie session.**
`plan.md:1878-1885` reports `{"cookies": <count>, "captures": [{name, expires_at}, ...]}`, and
`plan.md:1825` asserts not even a cookie name travels. For zyBooks that is enough: the captured token
carries its own `expires_at`. For VHL there is no capture at all — the session *is* the cookie — so
the cloud's only input is a count, and `observe` (`plan.md:1865`) only ever pushes, so the count
never falls when a cookie expires or the vendor sends `Max-Age=0`. The cloud therefore cannot decide
"skip the login steps"; it can only guess, be served the login page again, and reauth — which works,
but costs an extra round trip on most runs and makes fidelity row R4-17 (`spec §12`, "yes") an
overstatement. **Fix:** report per-cookie expiry with no name and no value — for example
`{"cookies": {"count": N, "earliest_expiry": <unix or null>, "session_cookies": M}}` — and prune
expired entries in `SessionRecord::load`/`observe` so the count means something. A session cookie
with no `Expires` reports `null`, which is honest: the cloud then knows it must try and may be
wrong. Exit-gate 15(c) is what turns that into a number.

**I12 — `{kind: "reauth"}` carries no steps and the handler is said to name no source.**
`plan.md:2725` defines `PlanResult` as
`{kind:"steps"} | {kind:"reauth"} | {kind:"failed", warning} | {kind:"done"}`, and `plan.md:2620`
and `:2796` assert `reauth` on a dead session. Spec §4 says the cloud "compose[s] the login steps and
retr[ies] that step **once**". Nothing in the plan says who composes them: `reauth` carries no steps,
and `handler.ts` "names no source" (`plan.md:2726-2727`) so it cannot know what a login is. **Fix:**
make it explicit in `mod.ts`'s `Plan` contract — on `reauth` the handler calls `plan.start(ctx)`
with `reauthed: true` written into that source's cursor, and the plan's `next` is responsible for
re-issuing the step that failed after the login completes. Two lines of interface, and
`a_dead_session_on_the_item_list_is_not_an_empty_shelf` then tests a real path rather than a
sentinel.

**I13 — `expires_at=lt.now()` is unverified PostgREST, and a wrong literal 500s every `/relay`
call.** `plan.md:2451-2453` sweeps with `restDelete(rest, "relay_runs", "expires_at=lt.now()")`.
`restDelete` (`cloud/supabase/functions/_shared/db.ts:95-101`) calls `ok(res, ...)`, which throws on
a non-2xx, and the sweep is step 3 of the handler (`plan.md:2459`) — before `readJson` and before
any run logic. PostgREST passes the filter value to Postgres as a literal; Postgres accepts the
special datetime input `now` but not the string `now()`, so the safe spelling is
`expires_at=lt.now`. No function in this repository does a time-filtered PostgREST call, so there is
no precedent to copy, and Task 5 step 9's four staging proofs (`plan.md:2536-2539`) exercise 401,
402, 400 and 404 — the first two return before the sweep, so only two of the four would catch it.
**Fix:** use `expires_at=lt.now`, and add a fifth staging proof that inserts a row with
`expires_at` in the past, calls `/relay` once, and reads the row gone.

**I14 — a blown budget can be the warning that loses the run log.**
`plan.md:2467-2469` ends a budget-exceeded run with `done` carrying "a sentence naming the bound",
and `plan.md:2124-2132` asserts markers like `too many round trips`, `batch too large` and
`too much data`. Only one warning reaches `state/runner-log.md` — the rest collapse into `(+N more)`
— and which one wins is `rank_warnings`, which sorts by `FAILURE_MARKERS`
(`engine/src/coursework.rs:1437-1449`: `session invalid`, `fetch failed`, `0 assignments parsed`,
`config unreadable`, `coursework pass failed`, `the service is unavailable`, `parse failed`). None of
the three budget phrasings contains any of them, so a blown budget sorts *below* a benign
"uncategorised; using default importance" and the only durable diagnostic under Task Scheduler hides
it — the exact failure `rank_warnings`'s own doc comment says it exists to prevent. **Fix:** phrase
every budget warning so it contains an existing marker (`coursework: fetch failed (the run hit its
round-trip budget); nothing changed` reads correctly and costs nothing), or add one marker and move
the `assert_eq!(FAILURE_MARKERS.len(), 7)` pin at `coursework.rs:2671` in the same commit. Say which
in Task 4, and assert the chosen phrasing sorts first in `rank_warnings`.

### Minor

**M1 — the test counts in Task 10 are wrong.** `plan.md:408-414` says `zybooks.rs` is "1,254 lines,
parser + network + 44 tests" and `vhl.rs` "1,275 lines, parser + network + 39 tests". The line counts
are exact; the test counts are 38 and 36 (`grep -c "#\[test\]"`). `plan.md:3232-3234`'s predicted
fall of "roughly 83" is really about 74. Say "record the real number" — which the plan already does
elsewhere — and drop the two literals.

**M2 — line-cite drift, all small, all worth correcting once.** `coursework_request` is at
`coursework.rs:639`, not `:636` (plan and spec); `post_coursework` at `:670`, not `:671`;
`cloudmodel.rs`'s `scrub` call at `:252`, not `:251` (`plan.md:486` — `plan.md:1285` has it right);
`account::api_base` at `app/src/account.rs:42`, not `:43`; `anon_key` at `:46`, not `:47`;
`site/privacy.html`'s *Coursework logins* bullet at `:24`, not `:22`, and the *Your coursework
logins* definition being rewritten is the `<dd>` at `:38` (the `<dt>` is `:37`); the
`section_mapping` "one predicate both callers share" doc is `coursework.rs:1255-1257`, which is
`vhl_rows`'s comment rather than a rule stated at `:1255`; and `vhl.rs`'s "single most important
line" is the doc at `:342-344`, not `:17-21` (which is the module doc's third live-rollout fact, and
says the same thing).

**M3 — two hand-off snippets do not match the file.** H1b (`plan.md:631-635`) shows
`#[cfg(windows)] pub mod vhl;` — `engine/src/lib.rs:58-59` declares both `zybooks` and `vhl`
unconditionally, with no attribute. H1a (`plan.md:613-614`) places `pub mod relay;` "after
`pub mod ranking;`, before `pub mod runs;`" — `runs` is at `:62` and `ranking` at `:72`, so that
placement does not exist. Both snippets are corrected by their own prose, which is good practice;
the snippets should be corrected too, because `plan.md:591-592` says they are applied verbatim.

**M4 — Task 1 step 6 leaves `vhl.rs` unable to compile.** `plan.md:1280-1283` adds
`use crate::relay::scrub;` to `vhl.rs`, but the same step moves `quote_plus` out of that file and
`vhl::urlencode` (`engine/src/vhl.rs:534-540`) calls it. The import needs
`use crate::relay::{quote_plus, scrub};`.

**M5 — two types are consumed and never declared.** `RunBudget` appears in `perform`'s signature
(`plan.md:2183`) and `SourceContext` in `run`'s (`plan.md:2261`) and in Task 8's construction
(`plan.md:2944-2947`), but neither has a definition anywhere in the plan. Add both to Task 4's
Produces with their fields, since Task 8 reads `SourceContext`'s shape off a sketch.

**M6 — the corpus pins are quoted as literals from an unmerged stream.** `plan.md:2545-2546` tells
the implementer to confirm `migrations_test.ts`'s counts are "unchanged (24 functions, 5 views as
C3′ Task 1 step 4 left them)". On `main` today they are **18** and **4**
(`cloud/supabase/migrations/migrations_test.ts:304`, `:320`); C3′'s plan says it moves 23 to 24 and
leaves the view pin at 5, so the numbers come from a state this review cannot see. **Fix:** say
"unchanged from the branch point" and have the task report record the observed pair. A literal that
disagrees invites an edit to a file C5 does not own.

**M7 — the "whole tree" Deno command is not CI's.** `plan.md:178-180` gives C2's merged command and
says it "must be copied exactly". `.github/workflows/ci.yml:84` runs
`deno test --allow-read --allow-write=cloud/eval --allow-net=127.0.0.1 --allow-env=... --config
cloud/supabase/deno.json cloud/supabase/ cloud/eval/` — the plan's version drops
`--allow-write=cloud/eval` and the `cloud/eval/` path. It is a fine command for `cloud/supabase/`
alone, but Task 12's gate should be CI's line verbatim, since CI is the gate.

**M8 — `replay_into`'s sketch will not borrow-check.** `plan.md:1873` writes
`Cookie::parse(line, &origin)` inside `replay_into(&self, ...)`. In ureq 3.4.0,
`Cookie::parse<S: Into<Cow<'a, str>>>(...) -> Cookie<'a>` and
`CookieJar::insert(&mut self, cookie: Cookie<'static>, uri: &Uri)`
(`ureq-3.4.0/src/cookies.rs:63`, `:123`), so a `&String` borrowed from `&self` yields
`Cookie<'_>` and will not insert. Pass an owned `String` (`rec.line.clone()`). The test at
`plan.md:1740-1741` happens to pass a `&'static str` literal, so it compiles and the method does
not — worth naming so the implementer does not conclude the design is wrong.

**M9 — resolving a relative `Location` has no library and no stated rule.** `plan.md:2200-2202` says
`Location` "is resolved with `ureq::http::Uri`'s own parts, not string concatenation". `http::Uri`
has no relative-resolution API, and the `url` crate is forbidden by this task's own dependency test
(`plan.md:1312`). Root-relative (`/next`) is a path-and-query swap and is easy; a bare relative
(`next`, `../next`) is not, and CAS occasionally emits one. **Fix:** state the rule — absolute and
root-relative are resolved, anything else is `transport` naming "an unresolvable redirect" — and test
it. A refusal here is safe: every hop is re-checked anyway, so the risk is a wrong fetch, not a
wrong host.

**M10 — a real user path is written into a repository document.** `plan.md:149` names a
named user account home directory as the vault never to touch. It is a guard rail rather than a
configuration, but it is a single-user literal in a checked-in file, and CLAUDE.md's first rule is
about exactly this shape. "Quinn's own vault, whatever `profiles.json` names, is never a target"
says the same thing and names nobody's disk.

**M11 — `discover_json` has two `cfg` arms and the plan changes one.** `coursework.rs:1290`
(`#[cfg(windows)]`) and `:1366` (`#[cfg(not(windows))]`) both take three parameters; Task 9 step 3
(`plan.md:3091-3097`) gives one new signature and H4b (`plan.md:786-802`) one dispatch arm. Both
arms need the fourth parameter or the non-Windows build breaks. Same for `collect_cloud`, whose
`#[cfg(not(windows))]` arm is at `:872`.

**M12 — `Cargo.lock` is claimed as C5's own and is a shared-single-owner file.** `plan.md:225` lists
`Cargo.lock` under "C5 owns"; `HANDOFF.md:72` lists root `Cargo.toml`/`Cargo.lock` under "Shared
files with a single owner". In practice enabling `Win32_Security_Cryptography` adds no package (the
`windows` crate's features are internal), so the lock probably does not move — which exit-gate item
12 already asserts. Say that it is expected not to change, rather than claiming ownership of it.

---

## The planner's three open decisions

**1. Portal session lifetime, measured at the exit gate — agree, with one addition.** It genuinely
cannot be learned offline, exit-gate 15(c) (`plan.md:3527-3533`) asks the right question in the right
order (a second slot immediately, a third the next morning, per portal), and the instruction to write
the answer into `HANDOFF.md` "including 'zyBooks re-authenticated every time', which would be a
finding and not a failure" is the correct posture. The addition: the measurement only means something
if the cloud's skip decision is expiry-driven, and today it is not for VHL (I11). Fix the report
shape first, then measure — otherwise 15(c) measures the reauth path in both cases and tells you
nothing about expiry. Also make 15(c) blocking for Task 12 rather than a note: `ttl_s: 43200` (Q4)
and the "logged into once" claim in the privacy draft both rest on it, and a published promise that
outruns the measurement is the one thing on this list that a later commit cannot fix.

**2. A 10 MiB `readJson` cap for `/relay` — disagree as stated; derive it.** See I1. The number is
chosen against a worst case (8 x 2 MiB = 16 MiB) that exceeds it, and the realistic worst case
(`MAX_RUN_BYTES` = 8 MiB raw, base64 at 4/3 = 10.7 MiB) also exceeds it, so the cap can refuse a
healthy run. Two changes make it a decision rather than a number: (a) set the server cap from
`MAX_RUN_BYTES` with the expansion factor written down — 16 MiB, or 12 MiB if `MAX_BYTES` drops to
1 MiB, which is still about 19x the largest payload ever measured; (b) give the device a matching
outgoing bound so a 413 is unreachable rather than merely unlikely, with the cross-pin test
`a_results_batch_over_the_cap_is_a_413_not_a_500` asserting the *relationship*. And say in the plan
that `readJson` reads the whole body before it measures (`_shared/http.ts:48-52`), so the cap is a
sanity bound and not a memory guard — otherwise the next reader will assume it is one.

**3. Budget warnings folded into `done.warnings` / `done.errors` — agree, with one correction.**
It is the right call for the reason the plan gives: `done` stays byte-identically the reply
`post_coursework` decodes today, the device's decoder is untouched, and a new field would put a
contract change in the one place C5 has worked hardest to leave alone. `done.errors` for discover is
right too — `errors_from_discovery` (`app/src/onboarding.rs:63`) already reads it and the wizard
already degrades to a typed mapping. The correction is I14: folding into `warnings` puts the budget
line into a channel whose *ordering* is decided by `FAILURE_MARKERS`, and none of the three proposed
phrasings matches one, so the one durable log line can be a benign per-item note while the run
quietly did nothing. Phrase the budget warnings with an existing marker and assert the ordering.
That is a wording change, not a design change, which is the point of agreeing with the decision.

---

## Fidelity

Spec section by section: carried by which task, or not carried.

| Spec | Carried by | State |
|---|---|---|
| §1 C5-D1 (one endpoint, batch-at-a-time; the device composes nothing) | Tasks 4, 5; ledger `plan.md:273` | Carried. `a_device_never_invents_a_request` scans for an `https://` literal outside `PORTAL_SOURCES`. |
| §1 C5-D2 (the table lives in the engine, the app imports it) | Task 1; ledger `plan.md:274` | Carried, with the R4-12 narrowing restated and argued. |
| §1 C5-D3 (every request checked, placeholder or not, every hop) | Tasks 1, 4 | Carried in intent; **I4** (the filled URL is never re-checked) is the gap. |
| §1 C5-D4 (per-source DPAPI session store, never in the vault) | Task 3 | Carried. The "never in the vault" half is tested twice (`plan.md:2931-2933`). |
| §1 C5-D5 (captures kept on the device, redacted out of the body) | Tasks 2, 4 | Carried; **I5** weakens the redaction mechanism. |
| §1 C5-D6 (`Set-Cookie` stripped from every reply) | Task 4, `no_set_cookie_header_ever_reaches_the_reply` | Carried, tested on a 302 as well as a 200. |
| §1 C5-D7 (no new engine command) | Tasks 8, 9; H4 adds no subcommand | Carried, with the argument repeated where it belongs. |
| §1 C5-D8 (versioned plan modules; a run row with no body and no secret) | Tasks 5, 6, 7 | Partly. The row and its guards are carried; **C3** and **C5** are what is missing. |
| §2.1 how a run starts and ends; the `client` block | Tasks 4, 5 | Carried (`plan.md:2072-2088` asserts protocol, job, hosts, redacted config, timezone, session). `has_credential` and the session report are there; **I11** on the report's content. |
| §2.2 the shape of a step | Task 2 (`plan.md:1511-1551`) | Carried in full: method closed to GET/POST, the four body shapes, the unnamed jar, `follow_redirects`, `max_bytes`, `capture`. |
| §2.3 substitution and its four encodings | Task 2 | Carried, and checked: `quote` for URL, `quote_plus` for form, serializer for JSON, verbatim-with-CR/LF-refusal for headers, both namespaced by source. Gaps: **I2**, **I4**. |
| §2.4 the reply shape | Task 4 | Carried: BOM kept, `body_b64` for non-UTF-8 (the RFC 4648 vector checks out), `too_large` as a refusal with no body, the closed error set, `Set-Cookie` gone. Gaps: **I3**, **I5**. |
| §2.5 budgets (nine rows) | Task 4 constants, Task 5 server half | All nine values carried and cross-pinned device-to-server. **C4**: only the counters are enforced; the clocks are not. |
| §2.6 unreachable cloud, 409, idempotency, nothing written before `done` | Task 4 (`plan.md:2100-2121`) | Carried, and correct against `CloudError::label()` (`cloudmodel.rs:190-201`). |
| §3 the host allow-list, all five rules and the static test | Task 1 | Carried completely, and the strongest part of the plan. Every §3 rejection has a case; the refusal names the host and is asserted not to carry a token-bearing URL. |
| §4 sessions: location, DPAPI, raw `Set-Cookie` replay, reauth on expiry, the one card | Tasks 3, 8 | Store carried and the two ureq reasons are measured, not asserted (both true against `cookies.rs:135-152`). Reauth-on-expiry weakened by **I11**; the one card is **C2**. |
| §5 data handling: zero retention, what is logged, the privacy page | Tasks 5, 11 | Cursor guard carried (`checkCursor` plus the migration's column scan). The "log the error's class, never its message" rule is stated (`plan.md:135-138`) but has no test on the server side — the device side does. Privacy page carried in Task 11 with real drafts. |
| §6 server side: layout, plan modules, run row, parse hand-off, discover | Tasks 5, 6, 7, 9 | Layout, `config.toml`, the two plans step-for-step (every cited zyBooks and VHL line number checks out), the in-process `ingestHandler` with a closed gate, the three pre-vault flags: all carried. **C3**, **C5**, **I8**, **I12** are the holes. |
| §7 what leaves the engine | Tasks 1, 10 | Deletions and moves carried, with `the_engine_holds_no_portal_url_and_no_login_flow` as the proof. **I7**: the "Stays" list keeps three symbols that lose their callers. `ureq`'s `cookies` kept with the rewritten reason; `json` deliberately not added. |
| §8 legal posture unchanged | Task 9 / H6 | Not carried: **I10**. The sentence H6 extends does not exist. |
| §9 testing and oracles | Tasks 1, 4, 6, 7 | Ports carried one-for-one with accurate line cites; the frozen references untouched and their Deno readers confirmed. **C1**: §9's own placement rule for the loopback suite was not followed. |
| §10 Q1-Q5 | Tasks 5, 8, 1, 3, 4 | All five asked at the task that needs them, verbatim, each built to the recommendation. Q6 (the privacy wording) correctly added. |
| §11 what the spec does not decide | Respected | `/ingest-coursework` stays public, no third portal, no grades, C4 untouched. |
| §12 ruling-4 ledger | The plan's own ledger | Answers to it row by row. R4-11 and R4-12's strengthenings and R4-15's narrowing are all restated with their arguments. |
| CLAUDE.md: frozen references | Global Constraints, Task 12 step 2 | Carried, and the point that C5 is the moment somebody could decide the two parsed references are unused is made explicitly. |
| CLAUDE.md: `rank` never calls a model | Global Constraints, Task 12 | Carried; `rank_cannot_reach_a_judgment_endpoint` is re-run as a gate item. |
| CLAUDE.md: `.no_console()` | Task 10 step 5 | Carried; neither deleted file spawns a child, verified by grep at HEAD. |
| CLAUDE.md: one YAML emitter, one JSON writer, no note re-dumped | Global Constraints; Task 12 step 3 | Carried; the Deno scan for `serde_json::to_string` in `relay.rs` is a nice touch. |
| CLAUDE.md: no single-user assumptions | Global Constraints | Carried in the code; **M10** in the plan text. |

---

## Interfaces verified against the code

| Symbol the plan consumes | Where it is | State |
|---|---|---|
| `wincred::read_credential(&str) -> Result<Credential, CredError>`; `Credential{username, password: Secret}`; `Secret::expose()`; no `Display` | `engine/src/wincred.rs:145`, `:85`, `:62`, `:71`, `:78` | Matches. |
| `cloudmodel::{CloudConfig, load, resolve, Unavailable, CloudError, CloudClient::{new,post,get}, CALL_TIMEOUT}` | `engine/src/cloudmodel.rs:47`, `:88`, `:136`, `:60`, `:177`, `:232`, `:271`, `:285`, `:42` | Matches. `CloudError::fatal()` is 401/402/403 (`:206`); `label()` gives "no session", "no entitlement", "rate limited", "the service refused" (`:190-201`) — the four the plan's test expects. |
| `crate::zybooks::scrub` call site to rewrite | `engine/src/cloudmodel.rs:252` (plan says `:251` in one place, `:252` in another) | Exists; one call, plus the module-doc mention at `:17`. |
| `ledger::dumps_value(&Value) -> String`, sorted keys, Python separators | `engine/src/ledger.rs:356-363` | Matches; the expected body in `plan.md:1372-1375` is what it emits. |
| `judge::one_line`, `judge::clip` | `engine/src/judge.rs:330`, `:315` | Match. |
| `info::{KINDS, NewInfo, open_info, list_info, close_info}`; `open_info(vault, &item, ctx, None, None)`; `kind: "notice"` | `engine/src/info.rs:27`, `:57`, `:69`, `:108`, `:157` | Match. `list_info` returns open items only — which is what makes **C2** a deadlock. |
| `coursework::{redact, REDACT_ALLOW_*, resolve_timezone, sync_coursework, assignment_from_row, FAILURE_MARKERS, rank_warnings, propose_map_cards, asked_map_keys}` | `coursework.rs:565`, `:559-560`, `:157`, `:245`, `:607`, `:1437`, `:1458`, `:815`, `:952` | Match. `courses` is in the zyBooks redact allowlist, `credential_target` is not — so `plan.md:2085-2087` holds. |
| `coursework::{fetch_zybooks, fetch_vhl, collect, Fetcher, BookRouting, route_zybook, zybooks_rows, vhl_rows, discover_json, parse_duration_hours, DURATION, main_with_fetchers}` (to delete) | `:464`/`:532`, `:511`/`:543`, `:1383`, `:426`, `:443`, `:454`, `:1231`, `:1259`, `:1290`/`:1366`, `:103`, `:91`, `:1491` | All present; both `cfg` arms exist for the fetchers, `collect_cloud` and `discover_json` (**M11**). |
| The `(fetchers, resolve(vault).ok())` match that collapses | `coursework.rs:1540` | Exact. |
| The run record's three steps (`zybooks`, `vhl`, `sync`) | `coursework.rs:1561-1593` | Built from a loop over the two source names; unaffected by C5, as H5 claims. |
| `zybooks::{SIGNIN_URL, BASE, USER_AGENT, zybooks_headers, get_json, signin, fetch_zybook_codes, fetch_assignments, fetch_payloads, scrub, quote, json_escape_ascii, decode_json}` and every ported test name | `zybooks.rs:34`, `:35`, `:42`, `:525`, `:556`, `:574`, `:603`, `:629`, `:651`, `:434`, `:454`, `:471`, `:519`; tests at `:1075`, `:1093`, `:1102`, `:1129`, `:1175`, `:1182`, `:1191`, `:1205`, `:1219`, `:1230`, `:1243`, `:935` | Every one matches, including the four header values and the UA string quoted in Task 6. |
| `vhl::{parse_user_session_form, first_dashboard_link, discover_sections, quote_plus, urlencode, login_and_fetch_dashboard, default_opener, section_mapping, login_page, landing_page}` and every ported test name | `vhl.rs:404`, `:468`, `:319`, `:518`, `:534`, `:549`, `:351`, `:300`, `:662`, `:677`; tests at `:1013`, `:1042`, `:1077`, `:1093`, `:1106`, `:1120`, `:1143`, `:1154`, `:1200`, `:1227`, `:1238`, `:1247`, `:1261` | Every one matches. |
| `_shared/http.ts`: `json`, `fail`, `methodNotAllowed`, `readJson(req, limit = 1 << 20)`, `asResponse` | `:13`, `:18`, `:22`, `:48`, `:65` | Match. Default cap is 1 MiB as stated; **I1** on the raise. |
| `_shared/db.ts`: `Rest`, `restSelect`, `restUpsert`, `restPatch`, `restDelete`, `restFromEnv` | `:12`, `:41`, `:71`, `:86`, `:95`, `:122` | Match. `restDelete` throws on non-2xx via `ok()` — relevant to **I13**. |
| `_shared/entitlement.ts`: `requireActiveEntitlement(req)` throwing a `Response` | `:69` | Matches; the `.catch((e) => e as Response)` rule the plan imposes on every handler test is right. |
| `_shared/judge_handler.ts`: `type Entitle` | `:6` | Matches `(req) => Promise<{account_id}>`. |
| `ingest-coursework/handler.ts`: `ingestHandler(entitle)`; reply `{assignments, warnings, proposals}`; the 24-assignment fixture count | `:113`, `:184`, `handler_test.ts:148` | Match. |
| `parse_zybooks.ts`: `requireSuccess`, `routeZybook` | `:55`, `:86` | Match. |
| `parse_vhl.ts`: `sectionMapping` | absent | **I8** — not exported, not defined. |
| `parse_zybooks_test.ts:22-23`'s BOM comment (H7) | verbatim match | Matches character for character. |
| `config.toml`'s `[functions.judge-rules]` as the last block | `:103-104` | Matches; H8 appends correctly. |
| `migrations_test.ts`'s two corpus pins | `:304` (18), `:320` (4) | **M6** — the plan quotes 24 and 5 from C3′. |
| `accounts.id references auth.users(id)` (so `account_id = auth.uid()` is valid RLS) | `20260910000100_accounts.sql:12` | Matches. |
| `app/src/scheduler.rs`: `CHILD_TIMEOUT`, `slot_argv`, `JudgePlan`, `IcsState`, `inference::judgments_dir` | `:27`, `:339`, `:292`, `:162`, `:247` | Match. `slot_argv` takes three parameters today; H2's fourth is well-founded but its body is **I9**. |
| `app/src/onboarding.rs`: `discovery_argv`, `rows_from_discovery`, `errors_from_discovery`, `discover_coursework`, `store_credentials` | `:74`, `:32`, `:63`, `:95`, `:850` | Match (plan says `:74` in one place, `:72` in another). |
| `app/src/account.rs`: `PENDING_TARGET`, `api_base`, `anon_key`, `PRIVACY_VERSION` | `:32`, `:42`, `:46`, `:25` | Match. `PRIVACY_VERSION` is `2026-09-16` on `main`; the plan assumes C1b moved it. |
| `app/src/credentials.rs::target_for` | `:17` | Matches `knowlu/<profile_id>/<source>`. |
| `app/src/inference.rs::SUPPORTED_RUNTIMES` as the pattern | `:219` | Exists; read-only here, as C4 requires. |
| `engine/tests/dependency_boundary.rs`: `MANIFEST`, `MANIFESTS` | `:11`, `:16` | Match. The new test's forbidden list (`url = `, `cookie_store = `, `reqwest`, `hyper`, `attohttpc`, `minreq`) false-positives on neither manifest. |
| `engine/tests/no_console.rs`'s floor | `:51`, currently `>= 3` | C3′ is assumed to lower it to 2; unverifiable here. Neither deleted file contains `Command::new`. |
| `engine/tests/cloud_contract.rs`'s loopback harness | `:15-60` | Exists and is copyable — but see **C1** for where the copy may live. |
| `engine/tests/site.rs`'s `PRIVACY` const | `:15` | Matches; the new test's `include_str!("../../site/privacy.html")` resolves correctly. |
| `site/privacy.html`'s three targets | `:13` (the one-line promise, C3′'s), `:24` (the bullet), `:38` (the definition), `:108` (Security) | The quoted "today" text matches exactly; the line numbers drift (**M2**). |
| `ureq 3.4.0`: `Agent::config_builder`, `timeout_global`, `max_redirects`, `max_redirects_will_error`, `http_status_as_error`, `cookie_jar_lock`, `Cookie::parse`, `CookieJar::{get,insert,iter}`, `save_json` behind `feature = "json"`, `Error::BodyExceedsLimit`, `body_mut().with_config().limit(n).read_to_vec()` | `config.rs:669`, `:497`, `:507`, `:435`; `agent.rs:190`; `cookies.rs:63`, `:108`, `:123`, `:135`, `:143`; `error.rs:66`; `zybooks.rs:421-426` | All present and all as the plan describes. Both reasons for not using `save_json`/`iter` are correct. `insert` wants `Cookie<'static>` — **M8**. |
| `windows 0.62.2`: `CryptProtectData`, `CRYPT_INTEGER_BLOB`, `CRYPTPROTECT_UI_FORBIDDEN`, `LocalFree(Option<HLOCAL>)`, feature `Win32_Security_Cryptography` | `Win32/Security/Cryptography/mod.rs:1659`, `:8271`; `Win32/Foundation/mod.rs:38`; `Cargo.toml:518` | All match; the `seal` sketch at `plan.md:1913-1927` compiles as written, and the feature implies `Win32_Security`, already enabled. |
| `regex` available for the placeholder matcher | `engine/Cargo.toml:33` | Present. |
| `RunBudget`, `SourceContext` | nowhere | **M5** — consumed, never defined. |

---

## What I did not check

- **Nothing was executed.** No `cargo build`, no `cargo test`, no `deno check`/`lint`/`test`, no
  `supabase` command, no `scripts/ci/eol-check.ps1`, no `scripts/wizard-check.py`. Every compile,
  borrow-check and test-outcome claim in this report is read from the sources, not observed.
- **C3′ and C1b have not merged**, so five of the plan's premises are unverifiable from `main`:
  `engine/src/entitle.rs` and `entitle::gate` in front of `coursework`; `sync` as the slot's first
  step in `slot_argv`; the `base64`/`ring` refusals in `dependency_boundary.rs` (neither crate is a
  dependency today, and no refusal line exists yet); `no_console.rs`'s floor at `>= 2` (it is `>= 3`
  today); and `PRIVACY_VERSION` at `2026-09-17` (it is `2026-09-16`). The corpus pins are the same
  class of assumption (**M6**). I checked that each premise is *stated* and checkable in one command,
  which the plan does well; I did not check that it will be true.
- **PostgREST's acceptance of `now()` as a filter value** (**I13**) — I reasoned from Postgres's
  datetime input rules and from the absence of any precedent in this repository, and did not test it
  against a live PostgREST.
- **Supabase platform limits**: the edge runtime's request body ceiling and memory ceiling against a
  10 or 16 MiB `results` POST, and whether `--use-api` bundles a module graph that reaches two
  sibling function directories (`../_shared/` and `../ingest-coursework/handler.ts`). The plan argues
  the second from the `_shared/` precedent, which is sound but is not the same import.
- **Real portal behaviour**: session lifetimes, whether zyBooks' `auth_token` outlives twelve hours,
  whether VHL's CAS ticket tolerates a device-driven redirect chain, and whether either vendor's
  markup still matches the ports. Only exit-gate 15 can settle these.
- **The DPAPI round trip at runtime.** I verified every signature and constant the `seal` sketch
  uses; I did not verify that `CryptUnprotectData` on a corrupted blob returns an error rather than
  succeeding with garbage (the plan's test at `plan.md:1788-1804` assumes it errors, which is right,
  but is an assumption here).
- **Whether `http::Uri` rejects a `{` in the authority**, which is what makes **I4** a defence-in-
  depth finding rather than an exploitable one. I did not read the `http` crate's authority
  validation table; the fix I recommend does not depend on the answer.
- **The exact reading of C3′'s and C1b's own plans** beyond the two facts I looked up (the corpus-pin
  bump and the shared-owner list). Cross-stream ownership overlap on `site/privacy.html`,
  `engine/tests/**` and `app/tests/**` is sequential by merge order and I treated it as such.
- **`docs/plans/2026-09-17-c1b-sign-in-plan.md`, its spec, and the C3 review report** were not opened
  or touched: other agents hold them.
- **The prose of the privacy drafts as legal text.** I checked that they describe the mechanism the
  code will have, that they contain none of `engine/tests/site.rs`'s four banned claim words, and
  that Task 11's assertions match the drafts. Whether they are adequate is Q6's question for Quinn
  and the lawyer, which the plan correctly refuses to answer itself.
