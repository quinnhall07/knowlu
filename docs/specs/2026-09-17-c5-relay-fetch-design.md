# C5 — the relay fetch: design

**Status: valid; amended 2026-09-17 after the plan review, in two rounds
(R-C5-plan-1/2/3/4/5).** The review of the implementation plan
(`docs/reports/2026-09-17-c5-relay-fetch-plan-review.md`, `5a2ca69`, and its appended re-review,
`9f186c7`) found places where this spec was underspecified rather than merely unimplemented — how
one run drives two sources, where a parsed body lives between round trips, what makes the paused
source's card true, and what `ureq` can actually tell us about a cookie — and one place where §8
asserts something the repository does not contain. **§6 is rewritten below** (not marked: it is the
correction of record), **§2.4, §2.5, §4 and §8 each gain a paragraph**, and four §12 rows are
restated. The controller's rulings are R-C5-plan-1 (the pause has an exit), R-C5-plan-2 (parse
incrementally; no shelf cap), R-C5-plan-3 (the composite driver), **R-C5-plan-4** (that exit is a
console *Settings → Logins* row, because the wizard's own command is unreachable after onboarding)
and **R-C5-plan-5** (`Expires`/`Max-Age` are parsed off the raw `Set-Cookie` line, because
`ureq::Cookie` exposes neither); the plan's two *Fix round … resolutions* sections carry them in
full. This spec is written from the cloud design's *Amendment 2026-09-17*
(`docs/specs/2026-09-09-knowlu-cloud-design.md`). It was drafted while that amendment stood
**PROPOSED**, and was written to be valid on signature; Quinn **signed it on 2026-09-17**, so this
spec is valid now and its decisions stand. Ruling 4 is the binding text here, with rulings 1–3 and 5
as context; every decision below is argued from those rulings. Ruling 6 names three things it left to this spec — the
relay step protocol, the per-source host table and the retention window for raw pages — and §2, §3
and §5 decide the first two; the third is §10's first Quinn-owned row. Nothing here starts before
C3′ merges (ruling 5).

**Sources read:** the amendment and §3.1, §4.3, §5.2, §9 and §11a of the cloud design;
`engine/src/coursework.rs`, `zybooks.rs`, `vhl.rs`, `wincred.rs`, `cloudmodel.rs`, `main.rs`;
`app/src/credentials.rs`, `scheduler.rs`, `inference.rs`, `onboarding.rs`;
`cloud/supabase/functions/ingest-coursework/`, `events/`, `_shared/entitlement.ts`;
`docs/notes/2026-09-09-knowlu-cloud-legal-landscape.md` §6; `site/privacy.html`; `CLAUDE.md`.

---

## 1. What C5 changes, in one paragraph

Today the device owns the whole credentialed fetch: `coursework::fetch_zybooks`
(`engine/src/coursework.rs:464`) and `fetch_vhl` (`:511`) read the password out of Credential Manager
and hand it to `zybooks::signin` and `vhl::login_and_fetch_dashboard`, which know the URLs, the
headers, the CAS form and the host change. C2 already moved the **parsers** server-side, so what the
device still owns is exactly the *sequence*. C5 moves the sequence too: the cloud composes one
request at a time with `{{credential:…}}` placeholders in it; the device fills them from Credential
Manager, sends the request from the student's machine with that source's cookie jar, and returns the
raw response; the cloud parses it and composes the next request. The password never leaves the
machine, the vendor still sees the student's own IP and session, and `engine/src/zybooks.rs` and
`engine/src/vhl.rs` are deleted. What the client keeps is one compiled-in table of which hosts a
given credential may ever be sent to — the guarantee that survives even a compromised server of ours.

| # | Decision | Where |
|---|---|---|
| **C5-D1** | One endpoint, `POST /relay`, drives a run as a **batch-at-a-time step protocol**; the device never composes a request of its own. | §2 |
| **C5-D2** | The **host allow-list lives in the engine** (`knowlu_engine::relay::PORTAL_SOURCES`), not in the app, and the app imports it. | §3 |
| **C5-D3** | Every relayed request is checked against the table — **placeholder or not**, and on every redirect hop, which the device follows itself. | §3 |
| **C5-D4** | A per-source session store on the device holds the recorded `Set-Cookie` lines and plan-named captured values, DPAPI-protected, never in the vault. | §4 |
| **C5-D5** | A value the plan marks `capture` is kept **on the device** and redacted out of the body returned to the cloud, so zyBooks' bearer token is treated like the password. | §2.3, §4 |
| **C5-D6** | `Set-Cookie` is stripped from every response the device returns. The cloud never sees a session cookie. | §2.4, §5 |
| **C5-D7** | No new engine command: `relay.rs` is a library driven from inside `coursework` and `coursework-discover`. | §7 |
| **C5-D8** | Fetch plans are versioned Deno modules per source under `cloud/supabase/functions/relay/plans/`; the run's own state is a `relay_runs` row that never holds a page body or a secret. | §5 |

---

## 2. The contract

### 2.1 How a run starts and ends

A run lives inside one slot (`coursework → ingest → judge → rank`, `app/src/scheduler.rs:339`), so
nothing fetches while the laptop is closed. The `coursework` step resolves its `CloudClient` exactly
as today (`cloudmodel::resolve`, `coursework.rs:1540`) and then, instead of calling
`zybooks::fetch_payloads`, calls `relay::run(&client, Job::Coursework, …)`.

```
device                                            cloud (POST /relay)
  POST {protocol:1, job, run:null, client:{…}}  →
                                                ←  {run:"<uuid>", steps:[Step, …]}      (1..8)
  perform each step in order, stop at the first failure
  POST {protocol:1, job, run:"<uuid>", results:[StepResult, …]}  →
                                                ←  {run, steps:[…]}  or  {run, done:{…}}
```

`done` ends the run and is the only reply carrying data: for `job: "coursework"` it is exactly the
body `/ingest-coursework` answers today — `{assignments, warnings, proposals}`, which
`post_coursework`'s reply decoder (`coursework.rs:670`, kept as `decode_coursework_reply`) reads
unchanged — and for `job: "coursework-discover"` it
is `{zybooks, vhl, errors}`, the shape `discover_json` (`:1290`) prints today. **The device's own
JSON goes through `ledger::dumps_value`** like every other body this crate writes.

The first call carries `client`: the source names this build can authenticate and their host patterns
(§3), the resolved IANA timezone (`resolve_timezone`, unchanged), the redacted per-source config
(`redact`, `:565` — the same allowlist, so `credential_target` still never travels), and, per source,
**how much session material the device holds — names and expiries only, never values**, which is what
lets the cloud skip the login steps while a session is still good (§4).

### 2.2 The shape of a step

```json
{ "id": "zybooks.signin", "source": "zybooks", "method": "POST",
  "url": "https://zyserver.zybooks.com/v1/signin",
  "headers": [["Content-Type","application/json"], ["Accept","application/json, text/javascript, */*; q=0.01"],
              ["Origin","https://learn.zybooks.com"], ["Referer","https://learn.zybooks.com/"],
              ["User-Agent","Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu"]],
  "body": {"json": {"email":"{{credential:zybooks:username}}","password":"{{credential:zybooks:password}}"}},
  "capture": [{"name":"zybooks_token","from":"json","pointer":"/session/auth_token","redact":true,
               "persist":true,"ttl_s":43200}],
  "follow_redirects": true, "max_bytes": 2097152 }
```

- `method` is `GET` or `POST` and nothing else. **Plans are read-only at the vendor**: a POST may be
  a login or a query, never a submission (§8).
- `body` is exactly one of `{"json": …}`, `{"form": [[k,v],…]}` (ordered pairs, urlencoded by the
  device), `{"text": "…"}`, or `null`.
- **The cookie jar is never named.** A step uses its `source`'s jar, always; a plan cannot reach
  another source's jar and cannot ask for none.
- `follow_redirects` defaults to `true`, and the device follows the chain **itself** — bounded at 5
  hops, checking every hop against the allow-list and recording every hop's `Set-Cookie`. Hence the
  agent's `max_redirects(0)` and `max_redirects_will_error(false)` rather than `ureq`'s default 10:
  its own follower would neither consult our table nor let us see a `Set-Cookie` set on a 302 — and
  CAS sets one there.

### 2.3 Substitution, and what it may not do

Three placeholder forms, and only these: `{{credential:<source>:username}}`,
`{{credential:<source>:password}}`, `{{capture:<name>}}`. They are substituted only into string
leaves of `url`, header *values*, `body.json` strings, `body.form` values and `body.text`.

| Position | Encoding | Today's code it reproduces |
|---|---|---|
| URL (path or query) | percent-encode, unreserved set `A-Za-z0-9_.-~` | `zybooks::quote` (`zybooks.rs:454`), which is what puts `auth_token` in the query today |
| `body.form` value | `quote_plus` (space → `+`) | `vhl::quote_plus`/`urlencode` (`vhl.rs:518`, `:534`) |
| `body.json` string | the serializer's own escaping, ASCII-safe | `ledger::dumps_value` |
| header value | verbatim, and **refused** if the value would contain CR or LF | new; header injection is the one thing substitution could otherwise buy |

`{{credential:<source>:…}}` resolves through `wincred::read_credential` (`engine/src/wincred.rs:145`)
against the vault's `coursework.<source>.credential_target` — the same read `fetch_zybooks` does
today, and the only place a password is touched. `{{capture:<name>}}` resolves from the run's capture
table: device-side memory, plus the session store (§4) for entries the plan marked `persist`. Both
forms are namespaced by source, so a value captured on a zyBooks step can never be substituted into a
VHL step.

**C5-D5, argued.** zyBooks' `session.auth_token` is password-equivalent for its lifetime — the reason
the legal note's *Avoid* list names "storing session cookies server-side". So the device captures it
by JSON pointer, keeps it, and (because `redact: true`) replaces it in the body it returns with
`"<captured:zybooks_token>"`. The cloud still reads `success` and `user.user_id` out of that body, as
it must; it never sees the token. Without this the first thing the relay would do is hand our servers
a bearer token for the student's zyBooks account.

### 2.4 The shape of the reply

```json
{ "id":"zybooks.signin", "ok":true, "status":200,
  "final_url":"https://zyserver.zybooks.com/v1/signin",
  "redirects":[{"status":302,"location":"https://…"}],
  "headers":[["content-type","application/json"]],
  "body":"…", "bytes":54874, "captured":["zybooks_token"], "elapsed_ms":412 }
```

- **`Set-Cookie` is removed from `headers`** (C5-D6), as is any header the source's table marks
  secret. The device's jar does the sending; the cloud has no use for a cookie value and no business
  holding one.
- **Body**: valid UTF-8 comes back as `body`, byte for byte, **including a leading BOM** — today the
  device strips it (`zybooks::decode_json`, `zybooks.rs:519`) and the server parser's test says "no
  BOM ever travels on the wire" (`parse_zybooks_test.ts:23`). After C5 one does, so the plan's own
  JSON decode strips it and that comment is corrected. Bytes that are not valid UTF-8 come back as
  `body_b64`; the caps are on the raw bytes. Nothing is transcoded and nothing is truncated — **with
  one stated exception, and it is a security one.** The body is run through `scrub` against the
  request's own secrets before it is returned. A vendor that rejects a login commonly **serves the
  login page again** (`vhl.rs:1077`) with the submitted username echoed back in a `value=`
  attribute, and a portal username is a credential half; without this the relay would carry it to
  our servers verbatim. `Filled::secrets` is what `scrub` is given. Fidelity therefore breaks only
  when a secret is literally present in the page, which is exactly when breaking it is right.
  Likewise a `redact: true` capture is replaced **at its JSON pointer and the value re-serialised**,
  never by a text replace — a text replace misses an escaped token and corrupts a body on a short
  one — and the bytes pass through untouched when no capture fired, which is what keeps the BOM
  above true.
- **Too large** is a refusal, not a truncation: over `max_bytes` (default 2 MiB; the captured zyBooks
  payload is 54,874 bytes and the VHL dashboard 7,495) the device returns
  `{"ok":false,"error":{"code":"too_large","bytes":N}}` and no body. A half page parses silently
  wrong, which is the failure this module exists to avoid.
- A failure is `{"ok":false,"error":{"code":…,"detail":…,"host":…}}`, `code` from a closed set —
  `host_not_allowed`, `unknown_source`, `no_credential`, `bad_step`, `transport`, `timeout`,
  `too_large`, `budget` — and `detail` through `scrub` (§7), so no credential and no captured value
  rides out in a third party's error text. The device stops the batch at the first failure and
  returns the results it has.

### 2.5 Budgets — *decided*

| Bound | Value | Derivation |
|---|---|---|
| steps per run | 40 | today's worst realistic run is 2 + *n* zyBooks calls + 3 VHL calls; 40 leaves room for a third portal and still bounds abuse |
| steps per reply (a batch) | 1–8 | one round trip per book would double a slot's latency on student wifi; 8 covers a full shelf in one |
| round trips per run | 24 | 40 steps ÷ a small batch, plus the final `done` |
| per-step wall clock | 60 s | today's `timeout_global` (`zybooks.rs:402`, `vhl.rs:353`) — but **a bound on the whole step, its redirect chain included**, not on one HTTP request. Today's is per request because the device makes one; here it makes up to six, so a per-request 60 s would be a six-minute step (plan review **C4**). **Two deadlines, each enforced once** (re-review **R4**): the step opens its own at `now + 60 s`, the run holds one at `now + 10 min`, and every request gets the *smaller remainder* of the two — `min(step, run)` — which is what makes the chain bound a mechanism rather than a constant named nearby |
| redirect hops per step | 5 | CAS uses two; five is slack, ten (ureq's default) is a loop |
| bytes per response | 2 MiB | ~38× the largest payload measured |
| bytes per run | 8 MiB | four such responses; past it the run ends with a warning |
| bytes per `results` POST | 16 MiB | `MAX_RUN_BYTES × 2`, and **the factor is JSON escaping, not base64**: the server measures the JSON *text*, and a quote-dense page approaches 2×. The same number is the server's `readJson` cap, so a 413 is unreachable rather than unlikely; a batch that would exceed it is **split into two posts**, never trimmed, because trimming loses a book the vendor served. **The platform's own ceiling is measured on staging** (the plan's Task 5 step 9) rather than assumed, and if it refuses below this, `bytes per response` drops to 1 MiB — still ~19× the largest payload measured — and this falls with it |
| run wall clock | 10 min | half of `scheduler::CHILD_TIMEOUT` (`app/src/scheduler.rs:27`, 20 min), so the relay can never be the thing the scheduler kills |
| server-side run TTL | 15 min | the `relay_runs` row expires past it and the next slot starts a new run |

Every bound exceeded ends the run as a **named warning**, never a 500 and never a non-zero exit — and
**the clocks are enforced, not compared**: the run's deadline is monotonic and is checked before
every step, every redirect hop and every round trip, with each request given what is left of it
rather than a fixed 60 s (plan review **C4**). Each warning is also **phrased to contain one of
`coursework::FAILURE_MARKERS`**, because only one warning survives into `state/runner-log.md` and
`rank_warnings` is what chooses it; a budget line that sorted below a benign per-item note would
hide the fact that the run did nothing (plan review **I14**).

### 2.6 An unreachable cloud, and a retried slot

`coursework` already always exits 0 and already has the vocabulary: `post_coursework` warns
`coursework: the service is unavailable (…); nothing changed`, and `the service is unavailable` is
already one of `FAILURE_MARKERS` (`coursework.rs:1437`), so it sorts to the top of the run log. C5
adds nothing here. A run that dies mid-way writes **nothing** — no note, no journal record, no partial
ingest — because the only write path is `sync_coursework` on the `done` reply; the abandoned
`relay_runs` row expires. The tray stays green: a step skipped for want of a service is not a failed
step (cloud design §5.1, the shape `judge (skipped: no entitlement)` already has).

**Idempotency.** The relay writes nothing anywhere, and a retry is a **new run** with a new token —
there is no resume. The vault write is unchanged and already idempotent (`sync_coursework` keys on
`source_uid`, `zybooks:…`/`vhl:…`, and creates-or-updates); map cards are already guarded against
re-proposal by `asked_map_keys` (`:952`), so reaching `done` twice files no second card. A replayed
`results` for a run whose cursor has advanced is answered `409 relay run out of step`, and the device
abandons it with one warning rather than re-driving a half-finished plan.

---

## 3. The host allow-list

```rust
pub struct PortalSource { pub name: &'static str, pub hosts: &'static [&'static str] }

pub const PORTAL_SOURCES: &[PortalSource] = &[
    PortalSource { name: "zybooks", hosts: &["*.zybooks.com"] },
    PortalSource { name: "vhl",     hosts: &["www.vhlcentral.com", "m3a.vhlcentral.com"] },
];
```

Exactly the two rows ruling 4 names, and the pattern `SUPPORTED_RUNTIMES` (`app/src/inference.rs:219`)
established: compiled in, so it cannot be edited by whatever also edited the thing being checked, and
adding a row is a deliberate diff in a release.

**The rule, precisely.** Before a socket is opened, for every request and **every redirect hop**:

1. the URL parses, the scheme is `https`, and the port is absent or `443`;
2. there is no userinfo (`https://x@evil.example@zybooks.com/` is refused), the host is not an IP
   literal, and the host is pure ASCII (no IDN, no punycode confusables);
3. the host is ASCII-lowercased and one trailing `.` is stripped;
4. it matches a pattern of the step's `source`: either `host == pattern`, or, for `*.d`,
   `host == d || host.ends_with(".d")`. No other wildcard form exists; a pattern that is neither is a
   bug the static test catches.

A request that fails any of these is refused **on the device, before the credential is read**, and
returned as `{"code":"host_not_allowed","host":"<the offending host>"}`. The host is named; the
credential never is, in any message, anywhere.

**Two deliberate strengthenings of ruling 4's words**, both recorded in §12. (1) Ruling 4 refuses "any
other request **carrying a placeholder**"; this spec refuses *every* relayed request off the list,
placeholder or not, because a request with no placeholder still travels with the source's cookie jar,
and a session cookie sent to a host of the server's choosing is the same leak by a slower route.
(2) Ruling 4 puts the table "in the app"; it goes in the **engine** (C5-D2), because the enforcement
point must be the process that reads Credential Manager and opens the socket, and a table handed from
the app to the engine over a command line is data, not a guarantee. `app/` already depends on
`knowlu_engine`, so the wizard's panel imports the same constant — one table, two crates, no copy.

**Why this is the one portal-specific thing the client keeps.** It is the only claim in the privacy
policy that does not rest on trusting our servers: a password goes to the site it was given for or
nowhere, and that holds if our project is compromised, if a plan is wrong, and if someone replaces a
plan. Everything else about a portal is server-side and fixable without a release. The price,
plainly: **a vendor that moves to a new host costs an app release** — a markup change does not, a
host change does. §10 asks whether VHL's row should be widened to `*.vhlcentral.com` to buy back the
`m3a`-renumbering case.

**The static test** (`engine/tests/relay_allowlist.rs`, an *integration* test, so it compiles the lib
without `cfg(test)`) pins: the table is exactly those two rows with exactly those hosts; every pattern
is lowercase ASCII with no scheme, port or path and is either an exact host or `*.` + host;
`*.zybooks.com` matches `zyserver.zybooks.com` and `learn.zybooks.com` and rejects
`zybooks.com.evil.example`, `evilzybooks.com` and `notzybooks.com`; every `name` is a legal
credential-target suffix; and the wizard offers no source the table does not carry.

---

## 4. Sessions

**Where.** `%LOCALAPPDATA%\knowlu\profiles\<profile_id>\sessions\<source>.bin`; `slot_argv` gains
`"--session-dir", <data_dir>\sessions` on the `coursework` step, the way it already passes
`--log-dir` to `judge`. Without the flag the store is in-memory only, so a hand-typed
`knowlu-engine coursework` logs in fresh and leaves nothing behind. **Never in the vault** — the vault
is plain text and, under ruling 2, syncs to the account; a cookie must not.

**What, and how protected.** Per source: the raw `Set-Cookie` lines the run observed, each with the
origin URI it came from, in order; and the captured values the plan marked `persist`, each with its
TTL. Cookies and session values — **never a password**, which is read from Credential Manager at the
moment of substitution and written nowhere. The record is DPAPI-sealed
(`CryptProtectData`/`CryptUnprotectData`, `CRYPTPROTECT_UI_FORBIDDEN`, current-user scope), so the
file is useless copied to another machine or another Windows user. Credential Manager is not used
here: its 2,560-byte blob cap is below a realistic jar.

**Why raw `Set-Cookie` lines and not `ureq`'s own jar serialisation.** `CookieJar::save_json` writes
only *persistent* cookies — one with no `Expires`/`Max-Age` is dropped, and CAS session cookies are
exactly that — and `CookieJar::iter` exposes only `name` and `value`, losing the `Domain` attribute
that makes one jar span `www.` and `m3a.vhlcentral.com` (`vhl.rs:342-344`, the module's own "single most
important line"). Replaying the vendor's own bytes with `Cookie::parse(line, &origin_uri)` +
`jar.insert` puts the RFC 6265 rules — expiry, host-only vs domain, path, overwrite, `Max-Age=0`
deletion — back where they were, and needs no `json` feature.

**And the device parses two attributes off that line itself** (ruling **R-C5-plan-5**, from the
re-review's R3). The session report below has to say *when* a cookie dies, and `ureq::Cookie` cannot
say: it is a newtype over `cookie_store::Cookie` with a private inner, and its whole public surface
is `parse`, `name`, `value` and a `Display` of `name=value` (`ureq-3.4.0/src/cookies.rs:44-92`).
`cookie_store` as a direct dependency is refused by this stream's own dependency test. So `Expires`
and `Max-Age` are read off the raw line under RFC 6265 §5.2.1 and §5.2.2 — `Max-Age` winning where
both appear, an unreadable value ignored rather than fatal, and a cookie whose expiry cannot be read
counted as a **session** cookie, which is the conservative answer. Twenty lines and a test each; the
replay keeps the job `ureq::Cookie` is good at.

**Re-authentication only on expiry.** The cloud decides it, from two inputs: the first call's report
of which session slots exist and when they expire (names and expiries, never values), which lets a
plan skip its login steps outright; and the vendor's own answer — a 401/403, or a page the plan
recognises as the login page again (`parse_user_session_form`'s failure, `vhl.rs:404`, and
`require_success`'s `success: false` on zyBooks) — which makes the cloud compose the login steps and
retry that step **once**. Not twice: a second failure is a credential problem, not a session one.

**The one card.** A login the vendor rejects mints one info note through `info::open_info` —
`kind: "notice"`, `close_key: "login:<source>"`, `opened_by: "agent:knowlu.coursework"`, title *"Your
zyBooks password no longer works"* — after checking `list_info` for an open item with the same
`close_key`, the way `propose_map_cards` checks `asked_map_keys`, so there is exactly one. A later run
that authenticates calls `close_info(key = "login:<source>")`.

**And the pause has an exit the student can reach** (ruling **R-C5-plan-4**, from the re-review's
R1). The card says *"Open **Settings → Logins** and save the password again"*, and C5 builds that
screen: one row per source in `relay::PORTAL_SOURCES`, backed by one console-window command that
writes the same `knowlu/<profile_id>/<source>` credential the wizard writes and closes
`login:<source>` on success. Without it the only way to save a portal password is the onboarding
wizard, which runs on a vault that does not exist yet — so the sentence would be false, the student's
only exit would be dismissing the card by hand, and dismissing it un-pauses the source with the same
wrong password, which is the loop this whole section exists to prevent. It is also a product gap on
its own: a student who changes a portal password has no way to tell Knowlu. The source is **paused**
until the
student saves a new password (§10 Q2) — the legal note's "stop on the first sign of a vendor block and
never retry through a change of identity". Never a retry loop.

---

## 5. Data handling

- **Raw pages are the student's data** (ruling 2). They live in the edge function's memory for one
  call, are parsed, and are dropped — never into `relay_runs.cursor`, Storage, a table or a log.
  Retention beyond the run is **zero**; §10 Q1 asks whether a diagnostic window is wanted instead and
  recommends no, because the issue report already carries a page to a human with consent and scrubbing.
- **What is logged.** Server-side, the existing pattern — the error's *class*, never its message
  (`console.error("relay: " + e.constructor.name)`) — plus per-run counters (step id, status, bytes,
  ms); a step id is ours (`zybooks.signin`), not a vendor URL with a token in its query. Device-side,
  the run log keeps its shape, every borrowed string through `judge::one_line` and `scrub`.
- **Never logged, either side:** a page body, a credential, a captured value, a cookie, a URL carrying
  a token.
- **The privacy page.** `site/privacy.html`'s *Your coursework logins* definition is rewritten. Its
  current sentence — "Knowlu signs in to zyBooks and VHL from your PC, with your credentials, and
  reads the assignment list that comes back; that list — never the password — may be read on our
  servers so that one parser can serve everyone" — becomes, in substance: *our servers compose each
  request and your PC makes it; your password is filled in on your machine, at the last moment, and
  only ever into a request to that site itself — Knowlu refuses to send it anywhere else, including to
  us. The page that comes back is read on our servers so one parser can serve everyone, and thrown
  away as soon as it has been read.* The *What stays on your machine* bullet gains the sign-in
  cookies; *Security*'s "Portal passwords … are never sent to us" stands and is now stronger. The
  exact wording is drafted in the C5 plan and read by Quinn before merge, as C3′'s privacy task
  already requires. The one-line pin at the top of that page (`engine/tests/site.rs:15`,
  `app/tests/static_assets.rs:627`) is about note bodies and belongs to C3′, not here.

---

## 6. Server side

**Layout.** `cloud/supabase/functions/relay/` — `index.ts` (`Deno.serve(relayHandler(requireActiveEntitlement))`,
the four-line shape every function has), `handler.ts` (protocol, budgets, run state), and
`plans/zybooks.ts`, `plans/vhl.ts`. `config.toml` gains `[functions.relay] verify_jwt = false` for
the reason the file already gives: each handler answers 401/402 in *our* shape.

**A plan** is a versioned module exporting `{ name, version, start(ctx), next(ctx, results) }`,
returning either steps or a result. It is pure over its cursor and the step results — no fetch of its
own, no clock beyond the request's, no database read except the account's own rows.

**One run drives every source, and the registry is keyed by job** (ruling **R-C5-plan-3**). A run row
holds one plan, the handler selects by job, and a `coursework` run has to drive zyBooks and VHL with
interleaved steps and a shared batch budget — so a registry keyed by *source* has nothing to answer
`PLANS[job]` with. The shape, precisely:

- `PLANS` is `{ "coursework": courseworkPlan, "coursework-discover": discoverPlan }`; the per-source
  modules live behind `SOURCE_PLANS = { zybooks, vhl }` and know nothing about each other.
- `cursor.sources = { zybooks: {…}, vhl: {…} }` — one sub-cursor each, plus the timezone and each
  source's redacted config from the first call's `client` block.
- A batch is filled **round-robin** across the sources that still have steps, inside one shared
  `MAX_BATCH`, so a full shelf never starves VHL's three sequential steps and VHL never starves the
  shelf.
- A per-source `{kind: "failed", source, warning}` **retires that source and lets the other finish**;
  the run ends when every source is done or retired. This is `collect`'s oldest rule — *one source's
  failure never stops another* — and discovery has always needed it.
- `{kind: "reauth", source}` **names its source**, and the handler answers it by writing
  `reauthed: true` into that source's sub-cursor and re-entering `plan.start(ctx)`: the module
  composes its own login steps, because the handler names no source and could not. Its `next` then
  re-issues the step that failed, and answers `failed` rather than `reauth` the second time — once,
  never twice (§4).
- `plan_version` is a stable **composite string** of the source modules' versions
  (`zybooks@1+vhl@1`, in table order), so a deploy that changes either module mid-run changes it,
  the 409 fires, and the device starts a new run next slot.

**Run state.** One row, deleted at `done` and swept by a `delete from relay_runs where expires_at <
now` at the top of every call (no cron needed — and the filter value is the datetime input `now`,
which PostgREST passes through as a literal; `now()` is a string Postgres will not take):

```sql
create table relay_runs (
  id uuid primary key default gen_random_uuid(),
  account_id uuid not null references accounts(id) on delete cascade,
  plan text not null,               -- the JOB: 'coursework' or 'coursework-discover'
  plan_version text not null,       -- the composite: 'zybooks@1+vhl@1'
  cursor jsonb not null,            -- per source: indices, zybook codes, scraped form fields, the dashboard link
  -- {assignments, own, proposals}, and ALL THREE are MAPS of source name to that source's rows, in
  -- arrival order within a source. `finishSource` turns `own[source]` into the reply's prefixed
  -- `warnings`; `finishRun` flattens the other two in PORTAL_SOURCES order. The partition is what
  -- lets a source that fails on a later round trip lose the rows it contributed on earlier ones,
  -- which is what a single `ingestHandler` call did and what the relay must reproduce. Its size is
  -- bounded by `MAX_RUN_BYTES` (8 MiB of raw body per run), which bounds everything that can ever
  -- be parsed into it.
  parsed jsonb not null default '{"assignments": {}, "own": {}, "proposals": {}}'::jsonb,
  seq int not null default 0, steps_used int not null default 0, bytes_used bigint not null default 0,
  started_at timestamptz not null default now(),
  expires_at timestamptz not null default now() + interval '15 minutes');
```
RLS: the account's own rows only. **Neither `cursor` nor `parsed` ever holds a page body, a cookie or
a captured value** — one Deno test asserts the cursor shape and another the parsed shape, after every
step of both plans.

**Why `parsed` exists, and why a shelf is not capped** (ruling **R-C5-plan-2**). An edge function
holds nothing between invocations; a coursework run spans several round trips; and §5 forbids
retaining a raw page anywhere. A twelve-book shelf therefore has eight payloads that must survive a
round trip with nowhere to be — and it bites below eight books too, because VHL's three steps are
strictly sequential, so its dashboard HTML lands in a later batch than the books on any shelf that
fills a batch. **So the cloud parses each raw body the moment it arrives and keeps only the parsed
rows**, which is exactly what ruling 2 and R4-19 permit to persist: *raw pages are received for the
run, parsed, and not retained; the parsed rows are what persists.* The alternative considered and
**refused** was requiring every payload-bearing step to land in one batch, which caps a shelf at
`MAX_BATCH` minus VHL's one and makes a thirteenth book a product limit; a student is not to
discover that in week one.

**The zyBooks plan**, step for step from `engine/src/zybooks.rs`:

| # | Request | From today's code |
|---|---|---|
| 1 | `POST https://zyserver.zybooks.com/v1/signin`, `Content-Type: application/json` + the four `zybooks_headers` (`Accept`, `Origin: https://learn.zybooks.com`, `Referer: https://learn.zybooks.com/`, `User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu`), body `{"email":"{{credential:zybooks:username}}","password":"{{credential:zybooks:password}}"}`; captures `/session/auth_token` | `SIGNIN_URL` (`:34`), `USER_AGENT` (`:42`), `zybooks_headers` (`:525`), `signin` (`:574`) |
| 2 | `GET {BASE}/user/<user_id>/items?items=%5B%22zybooks%22%5D&auth_token={{capture:zybooks_token}}`, `Authorization: Bearer {{capture:zybooks_token}}` + the four headers | `fetch_zybook_codes` (`:603`), `get_json` (`:556`) — the token in **both** query and header, deliberately |
| 3 | one `GET {BASE}/zybook/<code>/assignments?auth_token=…` per code the plan routed, batched | `fetch_assignments` (`:629`), `fetch_payloads` (`:651`) |

`user_id` comes from step 1's `/user/user_id`; `success: false` is a dead session, not an empty
shelf (`require_success`, already ported to `parse_zybooks.ts`); routing (`routeZybook`, mapped /
ignored / unmapped) already lives in `handler.ts` and decides which books step 3 asks for.

**The VHL plan**, step for step from `engine/src/vhl.rs`:

| # | Request | From today's code |
|---|---|---|
| 1 | `GET https://www.vhlcentral.com/`, `User-Agent` | `login_and_fetch_dashboard` (`:549`) |
| 2 | `POST https://www.vhlcentral.com/user_session`, `Content-Type: application/x-www-form-urlencoded`, `User-Agent`; body `form` = **every** named input and button scraped from the `id="user_session"` form, in order, with `user_session[username]`/`[password]` replaced by placeholders | `parse_user_session_form` (`:404`) — generic on purpose, because a missing `lt` ticket fails *silently* with HTTP 200 |
| 3 | `GET <dashboard_link>` on `m3a.vhlcentral.com`, `User-Agent` | `first_dashboard_link` (`:468`) — read out of `data-schools-payload`, never constructed |

Steps 2 and 3 depend on the same jar spanning both hosts (§4). The dashboard link comes from the
vendor's own payload and is therefore checked against the allow-list like any other URL — which is
precisely where `m3a.vhlcentral.com` earns its row.

**The parse hand-off, per arrival** (ruling **R-C5-plan-2**). The parsers do not move and
`/ingest-coursework` does not change what it answers; what changes is that the relay cannot hand it a
whole request, because the bodies of one run do not exist at the same moment. So the endpoint's
**per-source policy layer is extracted** — three functions out of `ingest-coursework/handler.ts`,
with `ingestHandler` rewritten in terms of them, so there is one copy and not two:

- `ingestZybook(book, cfg, timeZone, warnings, proposals)` — `pickZybooks`'s loop body:
  `requireSuccess` first (a session can die between the item list and one book's fetch), then
  `routeZybook`, then `parseAssignments`, with an unmapped code becoming a proposal and an ignored
  one staying silent.
- `ingestVhl(html, cfg, warnings, proposals)` — `parseDashboard`, then the unmapped-section warnings
  converted to one proposal per section with the existing de-duplication.
- `finishSource(name, items, own, warnings)` — the `{name}: ` prefixing and the
  *0 assignments parsed; treating as failure* rule.

The relay calls the first two as each body arrives, appends their rows to `parsed`, and calls the
third once per source at the end. **`done` is byte-identical to what one `ingestHandler` call
produced from the same bodies**, and three things make that true rather than hoped: the functions are
moved code, not re-derived code; the endpoint's own suite and the two **frozen parsed references**
(`zybooks-parsed-reference.json`, `vhl-parsed-reference.json`) are unchanged and are the oracles; and
the final assembly concatenates sources in table order (zyBooks, then VHL) and, within a source, by
arrival index — never by completion time, which round-robin would otherwise scramble. A twelve-book
shelf spanning round trips is tested against the one-shot path for equality. The endpoint stays
public through C5 and is retired only when no shipped client posts to it.

**`coursework-discover` as a relayed job.** The wizard runs before the vault exists, so there is no
`config/cloud.yaml` to resolve a client from: the command gains `--cloud-base`, `--anon-key` and
`--session-target`, which the app fills from its own constants and `knowlu/pending/session` — a public
key and a target *name*, never a secret, exactly as `discovery_argv` already passes target names
(`app/src/onboarding.rs:74`). The job runs zyBooks steps 1–2 and VHL steps 1–3; the server shapes the
rows (`zybooks_rows`/`vhl_rows`, `coursework.rs:1231`/`:1259`, moved into `plans/`) with `mapped`
computed from the redacted config, or `false` when there is no vault. `discoverPlan` is a **job
plan like `courseworkPlan`** — same composite driver, same per-source sub-cursors, same round-robin
batch, same rule that a failed source is retired with its sentence in `errors` while the other
finishes. `routeZybook` and `sectionMapping` are the **one** predicate each caller shares: the first
is already exported by `parse_zybooks.ts`; the second does not exist yet and C5 adds it to
`parse_vhl.ts`, with `parseDashboard`'s two inline lookups calling it, so the parser and the row
shaper cannot drift (plan review **I8**; the Rust original is `vhl::section_mapping`, which C5
deletes). Still read-only, still exit 0, still one JSON object on stdout.

**Adding a portal.** The flow, URLs, order, parser, oracle and reconcile are a deploy — no release, no
user action. What still costs a release is the allow-list row and the wizard panel that captures that
source's login. Ruling 4's "adding a portal … is a cloud change with no release" is therefore narrowed
to *repairing* a portal and to everything about one except its hosts and its login panel; §12 records
the narrowing and §3 argues why the table is worth it.

---

## 7. What leaves the engine

**Deleted:** `engine/src/zybooks.rs` and `engine/src/vhl.rs` in full except the three helpers below;
`coursework::fetch_zybooks`, `fetch_vhl`, `collect`, `Fetcher`, `main_with_fetchers`'s fetcher seam,
`route_zybook`/`BookRouting`, `zybooks_rows`, `vhl_rows`, and `discover_json`'s credentialed halves.
The parsers left in C2; the fetch halves leave now; the frozen `zybooks-parsed-reference.json` and
`vhl-parsed-reference.json` **stay exactly where they are** as the server parsers' oracles (CLAUDE.md
rule 2), read by the Deno tests by relative path as they already are.

**Moves, not deleted:** `scrub`, `quote` and `json_escape_ascii` (`zybooks.rs:434`, `:454`, `:471`)
move to `engine/src/relay.rs` with their tests. `cloudmodel::scrub` calls `crate::relay::scrub`;
`quote`/`quote_plus` become the relay's URL and form encoders (§2.3). `coursework::parse_duration_hours`
follows the VHL parser server-side.

**Stays:** `Assignment`, `SourceError`, `sync_coursework`, `load_coursework_config`,
`resolve_timezone`, `redact`, `yaml_to_json_for_request`, `assignment_from_row`,
`coursework_request`, `post_coursework`, the map-card machinery, `FAILURE_MARKERS`, `rank_warnings`.

**New:** `engine/src/relay.rs` — `PORTAL_SOURCES`, the host check, the substitution encoders, the
agent and jar, the session store, `run()`. **No new command** (C5-D7): a `knowlu-engine relay`
subcommand would be a general-purpose *make this request with my saved password* tool sitting on the
student's machine and in their process list, and the slot's four-step vocabulary is a contract with
`slot_argv`, the run records and the tray. The relay is a library the two commands call.

**`ureq`'s `cookies` feature stays, for a rewritten reason:** it is no longer VHL's module that needs
the jar but the relay, for the same property — one jar scoped to `.vhlcentral.com` spanning two hosts,
one agent per source per run. `engine/Cargo.toml:38-42`'s comment and CLAUDE.md's toolchain bullet are
rewritten to name `relay.rs`; the `json` feature is **not** added (§4).

**Config.** `credential_target` and `enabled` stay device-side (the student's own switch, and the
address of a password is not the server's business); `base_url` leaves the vault, because the URL is
the plan's.

**Entitlement (ruling 3).** C3′ puts the check in the engine — a slot refuses to run past the 72-hour
grace the app caches — and C5 inherits it, so a relay run is not started without one. `/relay` is
`requireActiveEntitlement`-gated like every other function, so a 402 mid-run is `CloudError::fatal()`
and ends the run with `no entitlement` and nothing changed. With the sequence, the parsers, the rules
and every judgment server-side and gated, an orphaned binary ranks a hand-made folder and nothing
else — ruling 3's stated purpose.

---

## 8. Legal posture — unchanged from D11

§9's portal-scraping row and the briefing's §6 turn on one question: *who is the accessor?* Under the
relay it is still the student, on their own machine, with their own credentials, at their own
instruction, and the vendor still sees their own IP and session — every row of the briefing's
comparison table still lands in its "on-device" column. Knowlu holds no portal password (there is no
column for one), no session cookie server-side (the briefing's *Avoid* list names that explicitly, and
C5-D6 honours it), touches no portal content with a model, runs read-only at the vendor, and is
rate-limited by the two slots a day it has always run in. What moved is the *instruction* — which URL,
in which order — and an instruction is not access.

**One correction of record** (plan review **I10**). This section said the wizard's one-sentence
disclosure of the ToS tension "stays", gaining a clause. **There is no such sentence in the
repository.** The coursework panel carries a credential-*storage* lede — *"Optional. Stored in
Windows Credential Manager on this machine — never in the vault, never in a backup, and never sent to
us."* — and nothing about the vendor's terms; no test pins a ToS literal, and neither `app/static/`
nor `site/` contains one. So C5 **writes** that disclosure rather than extending it, which makes it
legal copy: it goes to Quinn and the lawyer with the privacy wording (§5, §11) and is not appended by
a controller. If Quinn rules that the existing lede is the disclosure, that is a legitimate outcome
and this paragraph is what records it — what is not acceptable is a spec describing a sentence the
product does not have. The honest long-term route the briefing names — an official integration once there are
users to justify the ask — is unchanged by C5.

---

## 9. Testing and oracles

- **Relay, end to end, offline.** In `engine/src/relay.rs`'s own `#[cfg(test)] mod tests`, two
  loopback servers on `127.0.0.1:0` — one standing in for `/relay`, one for the portal — with a
  `#[cfg(test)]`-only table row `{name:"loopback", hosts:&["127.0.0.1"]}`. The precedent is
  `coursework.rs:3095-3160`, which duplicates exactly this harness in-module for `post_coursework`.
  Both listener threads are joined before the test returns; no DNS, no route off the machine.
- **The table is really two rows in a real build**: `engine/tests/relay_allowlist.rs` is an
  *integration* test, so it links the lib without `cfg(test)` and asserts `PORTAL_SOURCES.len() == 2`
  with exactly the ruled hosts — `SUPPORTED_RUNTIMES`'s own guard shape, and the reason the test-only
  row cannot escape.
- **The allow-list decisions, the substitution encoders, the header-injection refusal, the scrub and
  the reply shaping** are pure and unit-tested exhaustively, every rejection case in §3 included.
- **Deno, per plan.** `plans/zybooks_test.ts` and `plans/vhl_test.ts` drive each plan through scripted
  step results and assert the exact requests it composes — method, URL, every header, the body, the
  placeholders — and its failure branches. Every network-layer test in `zybooks.rs::tests` and
  `vhl.rs::tests` ports one-for-one (`signin_returns_the_token_and_user_id`,
  `a_dead_session_on_the_item_list_is_not_an_empty_shelf`,
  `login_post_body_carries_lt_and_service_from_the_parsed_form`,
  `login_rejected_serves_login_page_again_raises_not_logged_in`,
  `payload_with_no_open_enrollment_raises_not_logged_in`, and the rest), and the synthetic
  `login_page()`/`landing_page()` builders port with them, so no new fixture capture is needed.
- **The frozen references** stay the parse oracles they became in C2, untouched and never regenerated.
- **No test reaches the network**, either side: Rust loopback-only and joined, Deno already under
  `deno test --allow-net=127.0.0.1` in `ci.yml`.

---

## 10. Quinn-owned decisions, asked when the plan reaches them

| # | Question | Recommendation |
|---|---|---|
| Q1 | **Raw-page retention.** Zero beyond the run, or a short diagnostic window (say 24 h on a parse failure) so a vendor markup change can be fixed from the page that broke? | **Zero.** The diagnostic path already exists and is consented: the issue report, with its preview-and-scrub screen. |
| Q2 | **Does a rejected login pause the source** until the student saves a new password, or does every slot try again? | **Pause**, after one vendor rejection, with the card of §4. It is the legal note's "stop on the first sign of a block", and a daily retry with a wrong password is how an account gets locked. |
| Q3 | **VHL's host row**: the two exact hosts ruling 4 names, or `*.vhlcentral.com`? | Ship the two exact hosts as ruled. Widening is a one-line diff if VHL ever renumbers `m3a`; flagged so the choice is made knowingly rather than discovered on a broken slot. |
| Q4 | **The session store's TTL** for a captured value (zyBooks' bearer token): 12 h as proposed, or shorter? | 12 h — two slots a day, so a token is used at most twice before it is re-minted. |
| Q5 | **The run budget** of 10 minutes against the scheduler's 20-minute child timeout. | Confirm 10; it is the number that guarantees the relay is never what the scheduler kills. |

---

## 11. What this spec does not decide

The wording of the privacy sentences (drafted in the C5 plan, read by Quinn and the lawyer before
merge, as C3′'s task already requires); whether `/ingest-coursework` is retired as a public endpoint
once no shipped client posts to it; a third portal (nothing here names one, and §6 says what adding
one costs); reading grades from the signed-in LMS session (still §13's, still wanted, still
unscheduled); anything about C4, which follows C5 unchanged; and everything ruling 6 parked — web,
mobile, a server-run engine.

---

## 12. Fidelity ledger — against the amendment's ruling 4, and the parts of 2, 3 and 5 it touches

| Row | The ruling's sentence | Where it lands | Faithful? |
|---|---|---|---|
| R4-1 | credentials never leave the machine; the cloud never holds a portal password | §2.3 (substitution is the only touch), §7 (no column), §8 | yes |
| R4-2 | the fetch **sequence** moves — the login flow, the URLs, the order, out of `coursework.rs`/`zybooks.rs`/`vhl.rs` | §6 (both plans, step for step), §7 (deletions) | yes |
| R4-3 | "execute our fetch scripts … without having to give the client access" | §6 (plans are server-side modules; the device sees one step at a time) | yes |
| R4-4 | the device becomes a credential-substituting HTTPS relay | §2 | yes |
| R4-5 | the cloud composes method, URL, headers, body with `{{credential:<source>:username\|password}}` | §2.2, §2.3 | yes |
| R4-6 | the device substitutes from Credential Manager, sends with a per-source cookie jar, returns status, headers, body | §2.3, §2.4, §4 | yes, **minus `Set-Cookie`** (C5-D6) and minus captured values (C5-D5) — both strictly less exposure than the ruling requires |
| R4-7 | the cloud parses and composes the next step | §6 (`ingestHandler` in process; parsers untouched) | yes |
| R4-8 | a short sequence inside one slot, driven by the device's scheduler; nothing fetches while the laptop is closed | §2.1, §2.5 (40 steps, 10 min) | yes |
| R4-9 | host allow-list per credential, enforced on the device | §3 | yes |
| R4-10 | zyBooks `*.zybooks.com`; VHL `www.` and `m3a.vhlcentral.com` | §3 (the table, verbatim) | yes |
| R4-11 | any other request **carrying a placeholder** is refused and reported as a run error | §3 | **widened**: every relayed request off the list is refused, placeholder or not — the jar rides on all of them |
| R4-12 | a compiled-in table in the app, the way `SUPPORTED_RUNTIMES` pins runtimes | §3, C5-D2 | **moved to the engine**, imported by the app — the enforcement point must hold the credential and open the socket; one table, no copy |
| R4-13 | the one portal-specific thing the client keeps; it holds even against our own servers | §3 (the argument), §5 (the policy sentence it buys) | yes |
| R4-14 | the device knows no login flow, no URL and no parser | §7 (what is deleted) | yes, with the host table as the named exception ruling 4 itself carves out |
| R4-15 | adding or repairing a portal is a cloud change with no release and no user action | §6 | **narrowed**: repair, and everything but hosts and the login panel, is a deploy; a new host or a new login panel is a release. §3 argues the price |
| R4-16 | the frozen parsed references stay as the server parsers' oracles | §7, §9 | yes |
| R4-17 | the device keeps each source's jar between slots; re-authenticate only on expiry | §4 | yes, by recorded `Set-Cookie` replay — `ureq`'s own serialisation would drop the session cookie. **Restated 2026-09-17 (plan review I11):** the session report carries per-cookie **expiry** (a count, how many are session cookies with no expiry at all, and the earliest expiry among the rest) with no name, no value and no domain, and expired entries are pruned on load and on observe. A bare count could not have supported this row for VHL, where the session *is* the cookie and there is no capture; `earliest_expiry: null` is the honest "this may be alive", and the supervised run (§9, and the plan's exit gate 15(c), now blocking) is what turns it into a number. |
| R4-18 | a failed login is one card, never a retry loop | §4 (`info::open_info`, `close_key: login:<source>`), Q2 | yes — **and since ruling R-C5-plan-4 the card's instruction is true**: the console gains a *Settings → Logins* row and one command that writes the same credential the wizard writes and closes the card on success. The re-review's R1 found that the first round's exit was a wizard-only command with no post-onboarding caller, which left "never a retry loop" resting on a student dismissing a card — which un-pauses with the same wrong password, i.e. the loop. |
| R4-19 | raw pages: received for the run, parsed, not retained; the parsed rows persist; the policy says so | §5, §6, Q1 | yes — and **§6 now says how** (ruling R-C5-plan-2, from plan review C3): each body is parsed the moment it arrives and only the rows are kept, in `relay_runs.parsed`, deleted with the row. The sentence's second clause ("the parsed rows persist") is what makes the whole run possible; the first draft of the plan had nowhere for them and would have had to cap a shelf. |
| R4-20 | `coursework-discover` becomes a relayed cloud job over the same contract | §6 | yes, plus the three pre-vault flags the wizard needs |
| R4-21 | legal posture unchanged: the request originates from the device, with the user's credentials, at the user's instruction; the vendor sees the student's IP and session | §8 | yes for the posture, which nothing about C5 moves. **Corrected 2026-09-17 (plan review I10):** §8's claim that the wizard's ToS disclosure "stays" was wrong — there is no such sentence in the product. C5 **writes** it, which makes it legal copy: it goes to Quinn and the lawyer with §5's privacy wording rather than into a controller hand-off. |
| R2-a | raw pages are the student's data under ruling 2; the policy gains one sentence | §5 | yes; the note-bodies pin stays C3′'s |
| R3-a | the engine refuses a slot without entitlement past the 72-hour grace (C3′'s task) | §7 | inherited, not re-decided |
| R3-b | the fetch plans are among what moves to the cloud "as data" | §6 (versioned plan modules) | yes |
| R3-c | with the sequence, parsers, rules and judgments server-side and gated, an orphaned binary ranks a hand-made folder | §7 | yes — C5 is the sentence's "sequence" clause |
| R5-a | C5 is "the contract above, replacing the on-device fetchers", after C3′ and before C4 and the pilot | status line, §7 | yes; C5 does not start before C3′ merges |
