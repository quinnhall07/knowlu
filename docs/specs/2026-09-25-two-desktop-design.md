# Two desktops on one account: design

**Status: Draft for Quinn's review.** Written 2026-09-25 on branch `two-desktop-spec` at main `510a88c`.
**Authority:** `docs/specs/2026-09-09-knowlu-cloud-design.md` and its *Amendment 2026-09-17* (ruling 2: the
account is the source of truth and each desktop keeps a mirror; ruling 4 and C5: fetch on device). **Implements:**
Quinn's decision of 2026-09-24 on the C3′ whole-branch review's I2, "BOTH": deterministic ids for imported notes
as the backstop, and a device fetch turn. It does not relitigate that decision. Where a detail was open, it is
decided here with a recommendation and listed for Quinn in §9.

## 0. Where this comes from

With C3′ merged, every signed-in desktop runs the machine steps (`coursework`, `ingest`, `judge`) in its own slots.
The review (`docs/reports/2026-09-25-c3-account-vault-whole-branch-review.md`, I2 and row 35) showed two awake
desktops creating one new portal item as two notes: A with `id: a`, B with `id: b`. `apply` builds `mine` from
the **foreign** id (`engine/src/sync.rs`, step 4), so each desktop loses sight of its own history for that note,
`reconcile::resolve` falls back to the file-mtime stand-in, and a field set differently on each desktop diverges
with no card. Both desktops also judge the item (the daily cap is charged twice), and coursework-map cards are
filed twice. The review deferred four rows "with I2": N17 (row 11), E5 (row 30), M5 (row 40) and M6. §5.4 rules
each one in or out.

What the code does today, read for this spec (all at `510a88c`):

- **Ids.** `write::create` (`engine/src/write.rs:379`) keeps an `id:` the text carries and otherwise mints
  `ids::new_id` (5 random bytes, 10 hex; `ids.rs:68`). None of the nine producers in §2.2 supplies one.
  `ids::derived_id` (`:59`, SHA-1 of the path, 10 hex) exists for id repair only.
- **Paths.** Each producer picks `<folder>/<slug>.md` and walks `-2`, `-3` past a taken name. The slug is not
  machine-independent: ingest's is `<course>-<title>` with the course from the vault's own `course_map`
  (`config/`, which never syncs), and coursework's title carries a label from the same config. Map cards
  (`map-<source>-<key>`), rule cards (`rule-<id>`) and the day's events digest (`events-digest-<date>`) have
  fixed paths and random ids, so two desktops put two different ids at one path.
- **Apply.** A foreign `create` is journalled verbatim. Step 4 finds the local note by the **foreign record's
  path**. Step 3a performs a foreign `move` on whatever sits at its `old` path. A pulled text is written only
  when nothing is at that exact path (`create_new`). A tombstone (a `sync_notes` row with no body and no id)
  archives whatever sits at its path. A foreign `delete` record has no effect of its own: deletion reaches
  another desktop only through the tombstone and the archived text, both keyed by path.
- **The device token.** Three identifiers exist. `journal::device_name()` (`KNOWLU_DEVICE`, then `COMPUTERNAME`)
  is stamped in every journal record and in `config/runners.yaml`'s `device:`. `sync::device_token(account_id)`
  (`sync.rs:159`) is `sha256(account_id + "\n" + device_name())[..16]`, the opaque `device` column of
  `sync_records` and `sync_notes`. Telemetry carries no device. "The existing device token" below is the second.
- **The slot.** `scheduler::slot_argv` (`app/src/scheduler.rs:398`) is `sync → coursework → ingest → judge → rank`.
  `run_slot_inner` (`:614`) refreshes the session with a 45-minute floor (R-C1c-13), refreshes a missing or stale
  entitlement cache (R-C1c-11), and records every skip as `<step> (skipped: <why>)` at exit 0. *Sync now*
  (`commands::sync_inner`, `commands.rs:303`) runs `state::run_sync` and nothing else: it never runs a machine step.

## 1. Decisions

| # | Decision | Reason | Cost if wrong |
|---|---|---|---|
| **D1** | An imported note's id is `ids::import_id(kind, vendor, key)`: `<kind>_` + the first 10 hex of SHA-1 over `knowlu/import-id/1\n<kind>\n<vendor>\n<key>`. | Same shape as every id, so `ID_RE`, the journal and the read model cannot tell it apart; `sha1` is already linked for `derived_id`. | Frozen once shipped: a second derivation would re-open doubles between desktops on different builds. |
| **D2** | It covers the nine producers of §2.2, identified by `ids::import_key` over a closed vendor list. Hand-made and judgment notes keep random ids. | These are the notes an automatic step creates because an outside item exists: the ones two desktops create independently. | A producer left out keeps doubling in the backstop case; a hand-made note wrongly keyed would merge two different notes. |
| **D3** | Producers create through a new `write::create_imported`, which puts the import id where `create` would have minted one and refuses an id the vault already holds. | One path; the journal record and the frontmatter keep today's shape; only the id's value changes. | A key that repeats legitimately (a re-proposal) would be refused, so every re-proposable key carries its date. |
| **D4** | Paths keep today's naming and are **not** identity. Sync reconciles by id. | A path depends on per-desktop config and on collision order; fixing that costs readable names and still leaves the config. | The account can hold two text rows for one note in the rare backstop case; a restore keeps one. |
| **D5** | `apply` and restore work **by id**: records, moves and deletes find the local note by its id; a pulled text whose id is already held elsewhere is not written; a tombstone settles only a note the sender's history placed at that path. | It is what makes "sync merges it" true when paths differ, and it closes E5. | One id index per apply. Without it, a path difference becomes a second file and ids diverge again. |
| **D6** | Ids on disk never change, except that one imported item under two ids converges on the **lower** id: the desktop holding the higher one re-ids its note, and `Journal::records_for` follows that link. | It arises only from doubles made before this ships and from the mixed-version rollout; a total order needs no coordination. | A field set on the losing copy before convergence surfaces as at most one sync card, never silently. |
| **D7** | A delete beats a concurrent edit (M5): the note is archived on both desktops and the edit lands on the archived copy. | Falls out of D5; nothing is lost. | A student who edits on one desktop what they deleted on the other finds it in Archive (§9 Q4). |
| **D8** | Event proposals hold across desktops: a uid named in any events-digest note is already proposed, `rank --no-digest` (new) runs on a desktop that did not hold the `feeds` turn, and `/judge-event` answers a uid the account has already judged from that judgment. | The event ledger `state/events-seen.md` is device-local, so a turn that moves would re-propose and two desktops would each emit today's digest. | Without the account-level verdict of §2.8, a moved turn would spend the account's 80-a-day event budget re-judging. |
| **D9** | The account keeps a registry of computers: `devices(account_id, device, name, sources, first_seen, last_seen)`, written only by `/turn`, dropped 90 days after last seen. | The token is the one sync already sends; the name is already in every journal record; `sources` is what makes the turn useful. | Two Windows users on one PC share `COMPUTERNAME` and so one token (§3.2). |
| **D10** | The privacy page gains the words of §3.3. `PRIVACY_VERSION` moves only if 2026-09-24 was published before they land. | Precedent: R-C3′-exec-41's N4/N5 folded into an unpublished version. | A consent log that names the wrong version. |
| **D11** | The turn is **one lease per job**: `feeds` (ingest and judge) and one per portal (`zybooks`, `vhl`), claimed together in one single-statement compare-and-set on the server's `now()`, for 20 minutes, renewed before each machine step, released after the holder's trailing push. | Portal logins never leave a computer, so a computer without one cannot fetch it. In the common case one computer gets every job, which is Quinn's single holder. | If Quinn wants one lease (§9 Q1), a computer with no logins can win the turn and leave coursework unfetched for that slot. |
| **D12** | Holder: `sync → turn → coursework → ingest → judge → rank → sync (push) → release`. The others: `sync → turn → rank --no-digest`, recording the named skips of §4.3 at exit 0. | The results reach the account as soon as the holder is done, not at its next slot. | Without the trailing push, the others see the fetch two slots late. |
| **D13** | **Fail open**: a turn the service cannot answer runs the machine steps as today, named `turn (unavailable: …; running anyway)`. | D1–D5 make a double run safe, and failing closed would stop all fetching during a lease outage. | While the lease is unavailable, judgments may be charged twice. |
| **D14** | A holder that sleeps mid-slot loses the turn when its lease lapses; on waking, its next renewal is refused and its remaining machine steps are skipped as `(skipped: the turn passed to another computer)`. | The server's clock decides; nothing on the device compares times. | The step in flight when it slept still finishes; D1–D5 absorb the overlap. |
| **D15** | A desktop that was refused a turn runs one **catch-up** `sync → rank` when the holder's lease ends, looking at most three times and never later than 60 minutes after its slot. | Otherwise "the others get the results through sync" means at their next slot, up to 18 hours later. | One extra pull per refused slot. |
| **D16** | The first slot at Finish and *Run now* claim like any slot. *Sync now* never runs a machine step and never claims. The quit push releases nothing; expiry does. | *Sync now* is transport only today (`commands.rs:303`), and the lease must not change that. | None beyond D13. |

## 2. Deterministic ids for imported notes (D1–D8)

### 2.1 The derivation (D1)

```rust
pub fn import_id(kind: &str, vendor: &str, key: &str) -> String   // new, engine/src/ids.rs
// kind + "_" + hex(sha1("knowlu/import-id/1\n" + kind + "\n" + vendor + "\n" + key))[..10]
```

- **Inputs.** `kind` is what `ids::kind_for` answers for the note (`task`, `appr`), so the prefix is the one
  `write::create` would have minted. `vendor` is the note's own `created_by:` word. `key` is its `source_uid:`
  where it has one, else the producer's key from §2.2. No input is per-desktop: all three are frontmatter values
  the producer writes from the item itself.
- **Encoding.** Lowercase hex, 10 characters, exactly `ID_RE` (`^(task|appr|info|iss|course)_[0-9a-f]{10}$`).
  SHA-1 and the 10-character cut are `derived_id`'s, for its reason: an identity, not a security primitive. The
  `knowlu/import-id/1` line separates this hash from `derived_id`'s (a path cannot contain a newline) and
  versions it. 40 bits leave a vault of 10,000 imported notes under a one-in-10⁴ chance of any collision, and D3
  refuses a collision rather than merging it.
- **Reference values**, pinned by a test in `ids.rs` (computed for this spec with Python's `hashlib`):
  `import_id("task", "zybooks", "zybooks:1839992")` = `task_f8074297d0`;
  `import_id("task", "blackboard", "_blackboard.platform.gradebook2.GradableItem-_4732722_1")` = `task_b7d69d1870`;
  `import_id("task", "gmail", "gmail:m1")` = `task_b97a310bf0`;
  `import_id("appr", "events", "events-digest:2026-09-25")` = `appr_40ab7c3a10`.

### 2.2 What it covers (D2)

`ids::import_key(path, meta) -> Option<(kind, vendor, key)>` (new) is the one function that says whether a
note is imported and under which key. It answers only for kind `task` or `appr` and a `created_by:` in the closed
list `ids::IMPORT_VENDORS` = `zybooks`, `vhl`, `blackboard`, `gmail`, `events`, `coursework`, `rules`. Never for
`quinn`, `dashboard`, an `agent:` actor, or any other word.

| Producer (today's random mint) | Note | vendor | key |
|---|---|---|---|
| `coursework::sync_coursework` (`coursework.rs:528`, `:573`) | `tasks/<slug>.md`; `archive/` when imported-past | `zybooks` / `vhl` | `source_uid` |
| `ingest::sync_tasks` (`ingest.rs:750`, `:781`) | `tasks/<course>-<slug>.md`; `archive/` when imported-past | `blackboard` | `source_uid` (the feed's `UID`) |
| `enrich::write_gmail_note` (`:729`) | `tasks/<slug>.md` | `gmail` | `source_uid` (`gmail:<message-id>`) |
| `enrich::write_gmail_card` (`:783`) | `approvals/task-<slug>.md` | `gmail` | `source_uid` |
| `approvals::materialize` (`:1473`) | `tasks/<slug>.md` from an approved `kind: task` card | the payload's `created_by` | the payload's `source_uid`; random when it has none |
| `approvals::calendar_note` (`:1014`) | `approvals/calendar-event-<slug>.md` | `events` | `source_uid` (the event uid) |
| `eventemit::emit_digest` (`:240`) | `approvals/events-digest-<date>.md` | `events` | `events-digest:<proposed_at>` |
| `coursework::write_map_card` (`:1071`) | `approvals/map-<source>-<slug>.md` | `coursework` | `map:<source>:<map_key>:<first_proposed_at>` |
| `enrich::write_rule_card` | `approvals/rule-<rule_id>.md` | `rules` | `rule:<rule_id>` |

- **The date in two keys is load-bearing.** A map card is re-proposed 30 days after it expires, into a free
  `approvals/` path while the old card sits in `archive/`. Without the date, the new card would carry the old
  card's id, and D3 would refuse it for ever. The digest is one per day by construction.
- **Not covered, and why.** The console's `create_task`, issues, info notices, amend cards (judge-once and sync),
  C5's login notice, and the wizard's seeds keep `ids::new_id`. A person or a local judgment makes them, or, for
  seeds, C3′'s seed-hash pre-pass already reconciles them (`sync.rs`, the N2 pre-pass). **Calendar busy time is
  not a note**: `calendars:` feeds are read into capacity by `rank` and never written to a note folder, so they
  need no id.
- **The commitment model** (another session's program) interacts in one place, recorded in §5.6.

### 2.3 Minting (D3)

`write::create_imported(vault, rel, text, ctx, journal, held: &mut BTreeSet<String>)` (new) reads the text's
frontmatter, asks `import_key`, and:

- with a key, puts `import_id` exactly where `create` puts a minted id (the same
  `apply_frontmatter_fields_to_text` call), and journals the same `create` record;
- without a key, behaves exactly like `create`;
- refuses an id already in `held` (the vault's ids, from `ids::build_index`, built once per producer run and
  extended as it creates), as `WriteError::IdHeld(<id>)`, which the producer logs as
  `skipped (already held as <id>): <stem>`. Every producer already dedups by `source_uid` before creating
  (`existing_by_uid`, `existing_source_uids`, `asked_map_keys`, `existing_rule_ids`), so this is the belt.

Nothing else changes. The templates are untouched, the `id:` line lands where it lands today, and the journal,
the run records and the ledgers keep their bytes. Only the id's value differs, and an id is opaque.

### 2.4 Paths (D4)

Paths are **reconciled by id, not made deterministic**. The alternatives were both worse. A path built only
from the item (`tasks/<title>-<hex>.md`) would give every new imported note a hex tail and would still differ
wherever the title carries the per-desktop course label. A path whose collision suffix derives from the id stays
order-dependent: two desktops that create two same-slug items in opposite orders swap the base name. A path
difference arises only in the backstop case (two desktops create one item before either has pulled it) and only
when their course config differs. D5 makes it harmless, and D4 accepts it.

### 2.5 Apply and restore work by id (D5)

`sync::apply` builds `ids::build_index` (id → path) once, after its record pass, and follows five rules:

- **(a) Reconcile by id.** Step 4 finds the local note by the **id** of the foreign records, falling back to the
  record's path only for an id this vault does not hold. The `write_literals` and `live_sync_cards` calls use
  the local path. A foreign `create` for an id held here, at any path, is journalled and writes no file.
- **(b) Moves and deletes by id** (E5's move half). Step 3a performs a foreign `move` on the note holding the
  record's id, from wherever it is, to the record's `new` path, keeping today's `Exists` rule for a taken
  destination. A foreign `delete` (new here: today it has no effect of its own) settles the note holding its id
  through `write::delete` under `sync::ACTOR`, unless that note is already in `archive/`.
- **(c) One id, one file, and every id gets one.** Step 6 skips a pulled text whose own `id:` is already held
  here at another path (`sync: <path> is <id>, already held here as <local path>`), and one whose `import_key`
  names an item held here under another id, which D6 settles instead. It writes a text whose id is
  new here at its own path when that path is free, and at the next free `-N` name (`write.rs`'s `free_slot` rule)
  when a note with a different id holds it. Today that second text is dropped for good. Restore's `materialise`
  applies the same rule against the ids it has already written, so the first row by `rev` wins.
- **(d) A tombstone names a note, not a place** (E5's tombstone half). A pulled tombstone for `P` settles the
  local note at `P` only when another desktop's records place that note's id at `P` (its `create`, a `move` into
  `P`, or any record whose `path` is `P`), or when no record anywhere names that id (a hand-made note, whose only
  history is its text). Otherwise: `sync: <P> — the account settled a different note there; this one stays`.
- **(e) Convergence** of one item under two ids is D6's. It is detected by `import_key` on every foreign `create`
  in step 4 and on every pulled text in step 6.

**The two cases the brief asks about.** A foreign `create` for an id that already exists here: the record is
journalled, no file is written at any path, and the note's later records reconcile against the local file by
id (a). A foreign `create` or text at a path already taken by a **different** id: the local note is not touched,
and the foreign note lands beside it at the next free name (c), where its records find it by id (a). If both
notes are the same imported item under two ids, D6 merges them instead.

### 2.6 Existing vaults, and one item under two ids (D6)

Ids are a contract, so **no note on disk is re-id'd by this change**: the derivation applies only to creates
made after it ships. On one desktop an old random-id note never meets a new id, because the producers find it by
`source_uid` and update it. The meeting happens across desktops, in two ways: doubles two desktops made before
this ships (I2's), and the rollout, where a desktop on the old build mints a random id for an item an updated
desktop mints deterministically. Either way `apply` sees a note arrive whose `import_key` matches a local note
under a different id. The rule:

- **The lower id wins**, compared as strings (same `kind_` prefix, so it compares the hex). Both desktops
  compute the same winner from the same pair, with no coordination and no knowledge of which id is
  deterministic.
- **The desktop holding the higher id re-ids its note** in the same `apply`: one `write_literals` of `id` under
  `sync::ACTOR` (journalled; never sent). This is the same repair `ids::ensure_ids` makes for a duplicate, but
  through `write`. The file, its body and its path stay; only the id line changes.
- **History follows the link.** `Journal::records_for(id)` also returns the records of any id that a `set` of
  `id` renamed to it, one hop. So the step-4 reconcile that follows sees this desktop's own edits made under the
  old id. A field the two copies hold differently then becomes **one sync card** on this desktop, the ordinary
  C3′ card. Approving it or rejecting it (6b's re-assert) travels under the winning id and converges both
  desktops. Judge-once's `human_set` reads through the same link, so a field the student set under the old id
  stays theirs.
- **The desktop already holding the lower id does nothing.** The loser's pushed records carry an id it has
  never held, so they stay inert in its journal.
- **Rollout.** An old build never re-ids. If it holds the higher id, the pair converges when it updates (the
  updater checks daily). The note is doubled on that desktop meanwhile, exactly as today.

### 2.7 Delete against edit (D7, review M5)

Under D5, a delete on A and an edit on B resolve the same way whatever the push order:

- On B, A's `delete` record settles B's copy into `archive/` (D5 b), and B's edit is already on that copy.
- On A, B's `set` records find A's archived copy by id and apply to it (D5 a).
- B's live text can no longer resurrect the note on A, because its id is held there in `archive/` (D5 c).

The note ends archived on both desktops, the edit is visible in Archive, and nothing is lost. No card is filed.
§9 Q4 asks whether Quinn wants one.

### 2.8 Event proposals across desktops (D8)

This hazard was not in the review. The events pass writes verdicts to `state/events-seen.md` (device-local,
never synced), and `rank`, which runs on every desktop, emits `approvals/events-digest-<date>.md` from that
ledger (`eventemit::emit_digest`, called from `cli.rs`). Once the `feeds` turn has moved between desktops, both
ledgers hold verdicts, and each desktop's `rank` would emit a different digest for the same day at the same path.
It would also re-propose events the other desktop proposed or the student declined, because beyond its own
ledger `eligible_events` excludes only uids in a **pending** digest (`pending_digest_uids`, `eventemit.rs:75`).

- **Proposed means named in any digest note.** `pending_digest_uids` becomes "every uid in an events-digest note
  in `approvals/` or `archive/`, whatever its status". A settled digest keeps its `events:` payload in
  `archive/` (`write::delete` moves it whole), and that note syncs, so a proposal made or declined on one desktop
  is never made again on another.
- **Only the `feeds` holder emits.** `rank` gains `--no-digest` (new; default unchanged, so `oracle.rs` and the
  frozen references are untouched). The slot passes it on a desktop that did not hold `feeds` this slot (§4.3).
- **A verdict is the account's, not the desktop's.** A desktop taking `feeds` for the first time has no verdict
  in its own ledger for events the other desktop judged. At `DAILY_CAP.event` = 80 a day, shared by the account
  (`_shared/judge_caps.ts:39`), re-judging a roster would starve new events for days. So `/judge-event` answers a
  uid this account already has an `answered` judgment for from that row (the `judgments_item` index), with no
  model call and no `charge_call`, and the device records it in its own ledger as usual. "One verdict per uid,
  forever" (`eventledger.rs`) then holds per account, which is what it always meant.

## 3. The device registry (D9, D10)

### 3.1 What the account stores

```sql
create table if not exists devices (                       -- new migration, <date>000100_fetch_turns.sql
  account_id uuid not null references public.accounts (id) on delete cascade,
  device     text not null check (device ~ '^[0-9a-f]{16}$'),   -- sync::device_token(account_id)
  name       text not null check (char_length(name) between 1 and 64),
  sources    text[] not null default '{}',                     -- portal logins held here, e.g. {zybooks,vhl}
  first_seen timestamptz not null default now(),
  last_seen  timestamptz not null default now(),
  primary key (account_id, device)
);
```

- **`device`** is the existing token: `sync::device_token(account_id)`, which is already the `device` column of
  every `sync_records` and `sync_notes` row this computer sent. Nothing new identifies the machine.
- **`name`** is `journal::device_name()` (`COMPUTERNAME`, the name Windows gives the computer), cut to 64
  characters with control characters removed. The same string is already inside every journal record the
  account holds, so the registry adds no new fact about the machine; it only lists it. `KNOWLU_DEVICE`
  overrides it exactly as it already overrides the journal's.
- **`sources`** is the load-bearing column. It lists the portal sources whose login this computer holds: enabled
  in the vault's `config/ingest.yaml` `coursework:` section, with a credential present at its
  `credential_target` (`app/src/credentials.rs::exists`), and, after C5, not paused by a rejected login. The
  login itself never leaves the machine; only the source's name does. The turn (§4) grants a portal job only
  to a computer that lists it.
- **Written only by `/turn`**, on every call: an upsert that sets `name`, `sources` and `last_seen`. RLS: select
  own, no client write policy, the C3′ shape.
- **Retention.** A row is deleted 90 days after `last_seen`, by the claim function itself for the calling account
  (no cron job). `DELETE /account` purges `devices` and `fetch_turns` by name, beside the sync tables in the
  `account` function's purge list (`functions/account/index.ts:59`), and `GET /account/export` gains
  `devices: [{name, sources, first_seen, last_seen}]`. The token is left out of the export because it identifies
  nothing to the student.

### 3.2 Where the name comes from, and a limit

`COMPUTERNAME` is per machine, not per Windows user. Two Windows users on one PC, signed in to **one** Knowlu
account, therefore share a token and are one computer to the turn: both are granted it, both fetch, and D1–D5
merge the result. This is rare (one student, one account, two Windows logins on one PC) and is accepted. The
live proof works around it deliberately (§6.4).

### 3.3 The privacy page (D10)

In *Your tasks and notes* (`site/privacy.html:36`), the sentence *"Each change in the journal, and each issue you
raise, also carries the name Windows gives the computer it was made on, so that Knowlu can tell your computers
apart."* becomes:

> Each change in the journal, and each issue you raise, also carries the name Windows gives the computer it was
> made on. Your account also keeps a list of the computers you use Knowlu on: each one's Windows name, when it was
> first and last seen, and which of your coursework sites it holds a login for, never the login itself. Knowlu
> uses that list so that only one of your computers at a time fetches your coursework, school calendar and email
> and has them judged, and the others take the results from your account instead of doing the same work twice.
> A computer leaves the list 90 days after it was last seen, and deleting your account deletes the list.

The *Export* bullet (`:103`) adds "and the list of your computers" to what the file carries. A test in
`engine/tests/site.rs` pins both sentences, the shape of `the_review_amendments_i2_i3_and_i4_are_on_the_page`.

**`PRIVACY_VERSION`** (`app/src/account.rs:25`, now `2026-09-24`): if 2026-09-24 is still unpublished on
knowlu.com when this text merges, the words fold into it and the constant stays, as R-C3′-exec-41 did for N4 and
N5. If it has been published, the constant moves to the date the new text is published, and `record_consent_at`
records that version at each student's next sign-in. There is no re-ask screen; §9 Q6 asks whether a new data
category needs one.

## 4. The fetch turn (D11–D16)

### 4.1 One lease per job, one statement per claim (D11)

A **job** is a unit of machine work that one computer at a time should do: `feeds` (the `ingest` and `judge` steps,
meaning the LMS feed, the four cloud passes `enrich::run_lines_with` hosts, events, Gmail and rule decisions), and
one job per portal (`zybooks`, `vhl`, and after C5 whatever `relay::PORTAL_SOURCES` names). `feeds` needs nothing
device-held, so any signed-in computer can hold it. A portal job needs that portal's login, so only a computer
listing the source in `sources` can hold it. In the common case, one awake computer with every login, one computer
gets every job, which is Quinn's single holder.

```sql
create table if not exists fetch_turns (                  -- same migration as devices; RLS select own
  account_id uuid not null references public.accounts (id) on delete cascade,
  job        text not null check (job ~ '^[a-z][a-z0-9_-]{0,31}$'),
  device     text not null check (device ~ '^[0-9a-f]{16}$'),
  claimed_at timestamptz not null default now(),
  expires_at timestamptz not null,
  primary key (account_id, job)
);
-- inside fetch_turn(p_account, p_device, p_name, p_sources, p_claim, p_release): the claim is ONE statement
insert into fetch_turns as t (account_id, job, device, claimed_at, expires_at)
  select p_account, j, p_device, now(), now() + interval '20 minutes'
    from unnest(p_claim) as j where j = 'feeds' or j = any (p_sources)
on conflict (account_id, job) do update
  set device = excluded.device, claimed_at = excluded.claimed_at, expires_at = excluded.expires_at
  where t.expires_at <= now() or t.device = excluded.device;
```

- **Atomic.** `on conflict … do update … where` takes the row lock. A concurrent claimant waits, then its `where`
  is evaluated against the committed row (READ COMMITTED), so two claims of one expired job grant exactly one.
  With no row yet, one insert wins and the other takes the conflict path against it. The same statement renews:
  the holder's own device passes the `where` whatever the expiry.
- **`fetch_turn`** (new, plpgsql, not SECURITY DEFINER; `revoke execute … from public, anon, authenticated`, the
  guard's rule) runs, in order: the `devices` upsert and the 90-day prune (§3.1); the release,
  `update fetch_turns set expires_at = now() where account_id = p_account and device = p_device and job =
  any(p_release) and expires_at > now()`; the claim; then it returns every job row of the account as
  `(job, mine, seconds_left)`.
- **The clock is the server's.** Only `now()` in that statement decides. The reply carries `seconds_left`,
  never a timestamp, and nothing on the device compares its clock with the server's (the lesson of the review's
  I1). The length is server-side too: the client cannot ask for a longer lease.
- **20 minutes, renewed before each machine step.** `coursework` is bounded by C5's 10-minute run budget and
  today by its per-request timeouts. `judge` is bounded by `enrich::BATCH_BUDGET` (15 minutes), and
  `scheduler::CHILD_TIMEOUT` (20 minutes) caps any child. So a lease renewed at a step's start outlives that
  step, except when the step is killed at its cap, and D13 and D1–D5 cover that.
- **Released** after the holder's trailing push (§4.3). A failed release costs at most 20 minutes of lease.

### 4.2 The endpoint

`POST /turn` is a new function, `cloud/supabase/functions/turn/{index,handler,handler_test}.ts` with
`_shared/turn_db.ts`. Its `index.ts` has `sync-push`'s four-line shape with `requireActiveEntitlement`, and
`config.toml` gets `[functions.turn] verify_jwt = false`, for the reason the file gives. The body is
`{"device", "name", "sources", "claim", "release"}`. The account id comes from the verified JWT, never from the
body. `device` must pass `_shared/sync_rows.ts::isDeviceToken`. `name` is 1–64 characters after control
characters are stripped. `sources`, `claim` and `release` are lists of at most 8 words, each matching the `job`
check. The reply is `{"turns": [{"job": "feeds", "mine": true, "seconds_left": 1200}, …]}`. Refusals are 400, 401,
402 and 405, in `_shared/http.ts`'s shape. **A peek** is a call with empty `claim` and `release`: it refreshes
`last_seen` and reads the turns.

The device side is `app/src/turn.rs` (new; no Tauri command). It builds the body from
`knowlu_engine::sync::device_token`, `journal::device_name()` and the held sources (§3.1), and it authenticates
through `account::valid_access_token_at` under `SESSION_REFRESH_LOCK`, like every other account call. Its timeout
is the account agent's.

### 4.3 The slot: what the holder runs, and the lines the others record (D12)

`run_slot_inner` keeps everything before its child loop: the ingest skip line, the session pre-flight
(R-C1c-13), and the entitlement refresh (R-C1c-11). Then:

1. **`sync`**, both ways, as today. The pull comes first so that this computer sees the last holder's results
   before it claims anything, and a coursework run it may then make finds those notes by `source_uid`.
2. **`turn`**: claim `feeds` plus every portal source this computer holds a login for. A vault with no
   `config/cloud.yaml` has no turn and runs today's slot unchanged. A cloud vault whose `entitlement_state` is not
   `Entitled` does not claim (`turn (skipped: no entitlement)`), and the engine gate names the machine steps'
   skips itself (`entitle.rs`).
3. **A computer holding a job:** `coursework --only <held portals>` (new flag; skipped when it holds none), then
   `ingest` and `judge` if it holds `feeds`, each preceded by a renewal of the jobs it holds; then `rank` (with
   `--no-digest` unless it holds `feeds`); then **`sync --direction push`**, recorded as `sync (push)`; then the
   release. The push is what lets the others take the results at their next pull instead of two slots later.
4. **A computer holding nothing:** `rank --no-digest`, and the catch-up of §4.6.

The exact steps recorded, every one at exit 0 (so never retry backoff and never an amber tray):

| Case | Step line in `RunSummary` |
|---|---|
| granted one or more jobs | `turn (held: feeds, zybooks)` (jobs in the order `feeds`, then portal-table order) |
| granted nothing | `turn (another computer has it)` |
| a machine step whose job another computer holds | `coursework (skipped: another computer has the turn)`, and the same words for `ingest` and `judge` |
| no portal login on this computer | `coursework (skipped: no coursework login on this computer)` |
| a renewal refused (§4.5) | `<step> (skipped: the turn passed to another computer)` |
| the service cannot answer (§4.4) | `turn (unavailable: <cause>; running anyway)` |
| not entitled | `turn (skipped: no entitlement)` |
| the release failed | `turn (release failed: <cause>)` |

The runner-log filter (`scheduler.rs:792-808`) adds the `coursework (skipped:` prefix; the `ingest` and `judge`
skip lines already pass it. It also adds the two failures, without their cause, as
`turn (unavailable; running anyway)` and `turn (release failed)`, under R-C1c-final-3's sanitising rule. They are
all written with status `ok`, per R-C1c-plan-4, since a skip that repeats every slot is routine. `console.js`'s
`FIRST_RUN_SAYS` gains `turn: "Checking which computer fetches"`. `sync (push)` already reads as `sync`.
`slot_argv` takes the claim's outcome as a new `TurnPlan` argument; whether it splits into a pre-turn and a
post-turn half is the plan's to decide, but this order is fixed here.

### 4.4 Fail open (D13)

A claim that meets a transport failure, a timeout, a 5xx, a 404 (the function not yet deployed, as on
production before parity), a 401 the pre-flight could not prevent, or a reply that does not parse runs the
slot **as today**: every machine step, every source, and `rank` without `--no-digest`. It also runs the
trailing push. It releases nothing, because it holds nothing. The step is
`turn (unavailable: <cause>; running anyway)`. A renewal the service cannot answer counts as granted, for the
same reason.

Why open, not closed: D1–D5 make a double run converge on one note per item, and closing would stop every fetch
on every desktop of the account for as long as the lease service is down. Offline, the machine steps already
name their own skips (since C2 every one of them is a cloud call), so opening costs nothing there. The price
is double judgment charges while the service is down, bounded by the daily caps.

### 4.5 A holder that sleeps (D14)

A lid closed mid-slot freezes the children. The lease lapses 20 minutes after the last renewal, and the next slot
(or *Run now*) on another computer claims the job. On waking, the step in flight finishes or fails on its own
timeouts, and whatever it wrote is D1–D5's to merge. The sleeper's next renewal is the same claim statement:
it is granted if nobody took the job meanwhile (an expired row, the same device), and refused otherwise. A refusal
skips the remaining steps for the lost jobs as `(skipped: the turn passed to another computer)`. `rank` still
runs, with `--no-digest` if `feeds` was lost. The trailing push still runs, since it carries what the sleeper
wrote, and the release touches only rows the sleeper still holds.

### 4.6 The catch-up (D15)

A computer that was refused any job it asked for records the largest `seconds_left` among the refused jobs. The
scheduler's tick then runs a **catch-up** at that time plus 60 seconds:

- a peek, and if a refused job is still another computer's with time left, it waits for that. It looks at most
  three times and never later than 60 minutes after its slot began;
- then the session pre-flight, `sync` both ways, and `rank --no-digest`, recorded as their own `RunSummary`
  with `reason: "catch-up"`.

A catch-up never claims, so it can never become a second fetch. A slot that starts first supersedes it.

### 4.7 The first slot, *Run now*, *Sync now*, quit (D16)

- **The first slot at Finish** claims like any slot. On an account's first computer nothing contests it. On a
  later computer the vault has just been restored from the account, so a refusal costs nothing: the restored day
  is the account's day, and the catch-up brings the holder's run. C1c's first-run view shows the named lines as
  it shows every step.
- ***Run now*** (the tray) is a full slot and claims.
- ***Sync now*** stays transport only: `commands::sync_inner` → `state::run_sync` → the engine's `sync`. It
  never runs a machine step and never claims. A test pins it.
- **Quit.** `state::quit_flush` pushes as today and releases nothing. An app that quits mid-slot leaves its lease
  to lapse within 20 minutes.

## 5. Interactions

### 5.1 The entitlement gate

`/turn` is gated by `requireActiveEntitlement` like every function. The slot never claims past the grace
(§4.3, step 2), and the engine's own gate (`engine/src/entitle.rs`) still names `coursework`, `ingest` and `judge`
as skipped. `rank` is never gated; a computer that claimed nothing runs it with `--no-digest`, because the digest
is made from cloud verdicts. A 402 from `/turn` is `turn (skipped: no entitlement)`, never the fail-open line.

### 5.2 The session refresh

C1c's pre-flight (`ensure_session_for_at(.., 45 * 60)`, R-C1c-13) runs before `sync` and therefore before the
claim. The renewals, the release and the catch-up's peek, which can come up to an hour later, go through
`valid_access_token_at` (free while more than 120 seconds remain) under `SESSION_REFRESH_LOCK`. The catch-up runs
the pre-flight itself before its `sync`.

### 5.3 The first-day cap

The cap is **per account** and stays so. `charge_call` doubles each daily cap on an account's first two UTC judging
days (R-C1c-7, `20260923000100`), and those days belong to the account's first computer. A later computer restores
notes that are already judged (`needs_enrichment: false`), and under the turn it judges only when it holds `feeds`
and only what is new. D8's account-level event verdict keeps a moved turn from spending the cap again.

### 5.4 The rows the review deferred to I2

| Row | Ruling | Why |
|---|---|---|
| 11, N17: a late, older third-desktop write settles a card under a false warning | **Out** | It is reconcile's ordering across three desktops, which ids and the lease do not touch. The lease makes it rarer (one writer of machine fields per slot). Its recipe stands for a later stream: settle on a clean apply only when the applied write is later than the one the card offers (R-C3′-exec-19). |
| 30, E5: a foreign `move` does not check the note's id at the old path; tombstones lack the same check | **In** | D5 (b) and (d). Once paths may differ, a path-keyed move or tombstone can hit a different note. |
| 40, M5: delete against edit resolves by push order | **In** | D7. By id, the delete wins on both desktops and the edit is kept on the archived copy. |
| 41, M6: *Sync now* beside a slot's later child | **Out** | Not widened. The new `sync (push)` step takes `RunLock` like every sync, `/turn` calls write no note, and the race class (a console write beside a child) is unchanged. |

### 5.5 C5, the relay fetch

C5 (`2026-09-17-c5-relay-fetch-design.md`) keeps the device as the credential relay, so the turn keeps its shape:

- The portal jobs are C5's sources. `coursework --only <held portals>` becomes the source set the relay run
  offers in its first call's `client` block. A source C5 has **paused** after a rejected login is left out of
  `sources`, so a computer with a dead password never keeps the turn from one with a good one.
- **Recommended for C5's plan:** `/relay` carries the device token and answers `409 not this computer's turn` for
  a source whose job the caller does not hold, recorded as the ordinary skip line. That enforces the lease for
  coursework. It stays advisory elsewhere, because old builds must keep working through the rollout, but `/relay`
  has no old clients.
- C5's session store is per computer, so a portal job that moves logs in once on the new holder, which C5 §4
  already allows. R-C1c-2's server-side pre-judging makes `feeds` cheaper, not different. A server-run engine
  (ruling 1's door) would be one more holder with its own token.

### 5.6 The commitment model

The commitment specs (`2026-09-23-commitment-model-design.md`, `…-phase2-design.md`) add imported notes but do
not change how this spec identifies them. Their `commitments/` notes carry a `source_uid` (`gcal-series:…`,
`registrar:ua:<term>-<crn>`), their decline markers sit at a fixed path keyed by it, and their §2.5 keeps **the
lowest id** among cross-desktop duplicates, which is D6's rule. Whichever program lands second adds: their
`created_by` words to `IMPORT_VENDORS` and kind `cmt` (their R3) to `import_key`; the registrar fetch, which uses a
device-held login, as a portal job; and the Google series pull under `feeds`. Neither blocks the other.

## 6. Tests and proof

### 6.1 Engine (unit, and `engine/tests/sync_contract.rs`)

- `ids.rs`: the four reference values of §2.1; `import_key` over every producer's frontmatter; `None` for
  `created_by: quinn`, `dashboard`, an `agent:` actor, and kinds other than `task` and `appr`.
- Each producer: two scratch vaults fed the same input mint the same id, and a second create of a held id is
  `skipped (already held as <id>)`.
- `sync_contract.rs`, two vaults exchanging pages the way `two_desktops_with_a_card` and the N2x tests already do:
  - (i) the review's I2 scenario: both create item x, exchange, set `importance` 5 on A and 2 on B, then an
    unrelated `coursework` update on A. The result is one card, never silent divergence;
  - (ii) differing course maps put x at two paths: one file each side, and a later edit travels;
  - (iii) delete on A against edit on B: archived on both, the edit on the archived copy;
  - (iv) E5: a tombstone for a path holding a different id leaves it, and a foreign move acts on the id;
  - (v) a pre-existing pair under two random ids converges on the lower id, with at most one card;
  - (vi) a restore given two rows for one id writes one file.
- `eventemit`: a uid in an archived digest is never proposed again; `rank --no-digest` writes no digest; `oracle.rs`
  and `surface_oracle.rs` are unchanged.

### 6.2 App (`app/tests/scheduler.rs`, plus a loopback test for `turn.rs`)

- The argv for every `TurnPlan`, and the exact lines of §4.3.
- The order: session < `sync` < `turn` < `coursework` < `ingest` < `judge` < `rank` < `sync (push)` < release.
- The fail-open line and argv; a renewal refused mid-slot skips the rest; the runner-log filter carries the new
  lines, without causes.
- The catch-up: it is scheduled, it peeks, it never claims, and a slot supersedes it.
- `turn.rs` against a loopback server: full grant, partial grant, refusal, 402, 5xx, 404 and transport.
- *Sync now* runs no machine step and makes no `/turn` call.

### 6.3 Cloud: the migration and RLS guards

- `turn/handler_test.ts`: validation, the account taken from the JWT only, and the reply shape.
- The `account` function: `devices` and `fetch_turns` are in the purge list and `devices` is in the export, each
  pinned by a test.
- `migrations_test.ts`:
  - the function pin moves from **28 to 29** (`fetch_turn`), with its comment;
  - `fetch_turn` is revoked from `public, anon, authenticated`;
  - the view pin stays 5.
- **The RLS guard needs widening first.** It reads only C2's `20260911*` files (`ours()`, `:8`) and matches only
  `create table if not exists <unqualified name>` (`:274`), so no test holds C1's or C3′'s tables to RLS.
  `public.sync_records` and `public.sync_notes` do enable it, but nothing would catch one that did not. The
  widened guard runs over `everyMigrationFile()` and accepts an optional `public.` and a `create table` without
  `if not exists`. Today's corpus and the two new tables must pass it.
- **A staging check by the controller** (never a subagent): two concurrent claims of one expired job in two
  database sessions grant exactly one.

### 6.4 The live proof that gates release

**The gate.** No student is told that two desktops work, and no release notes say so, until this proof passes on
staging. It sits in HANDOFF §4's production-parity row, where "a two-desktop live proof" already waits on this
stream.

**Two Windows users on one machine are acceptable, for this reason.** Everything this design keeps per computer is
per Windows user: `%LOCALAPPDATA%\knowlu\` (profiles, the catch-up, C5's sessions), the vault under
`%USERPROFILE%\Knowlu\` (journal, `state/sync-cursor.json`, `state/events-seen.md`), Credential Manager (the
session and the portal logins), and the scheduler with its tray (one per signed-in session; the single-instance
plugin is per session). The one per-machine input is `COMPUTERNAME`, from which the token derives, so the proof
sets `KNOWLU_DEVICE` as a per-user environment variable, which `journal::device_name()` reads first. The two users
are then two tokens, exactly as two machines would be; one negative check without it shows §3.2's limit. What one
machine cannot show has a stand-in: clock skew does not matter, because the lease reads only the server's clock
(I1's clamp has its own proof); a partition between the two is replaced by step 4's 503 stub; and a real lid close
is replaced by suspending the holder's process tree (Sysinternals `pssuspend`), which is the same silence from the
server's side.

Both users stay signed in at once, by fast user switching. **Quinn signs the second Windows user in once**,
because that is a Windows password, which the controller never holds. The controller drives each app through its
own WebView2 remote-debugging port, DOM only (the autonomous-proof harness).

**What it must show** (staging, one account; both scratch profiles removed afterwards, per the standing rule):

1. Both computers in `devices`, with their names and `sources`, read back by the controller.
2. *Run now* on both within seconds. Each job is held by exactly one computer. The other's `RunSummary` carries
   `turn (another computer has it)` and the three skip lines, and its catch-up brings the holder's notes within
   about 20 minutes.
3. A new event on the staging test feed appears once on both computers, with the same id and path, no second
   file, and one `judgments` row for it.
4. **Fail open.** With `/turn` deployed as a 503 stub for this step, and restored afterwards, both computers
   fetch, and the new item still ends as one note with one id on both.
5. **Mixed versions.** One user on the previous release and one on the candidate. A doubled item converges on the
   lower id, with at most one card: first on the candidate, then on both once the old one updates.
6. **Sleep.** The holder is suspended mid-`coursework` for more than 20 minutes, and the other computer takes the
   jobs on its next *Run now*. Once resumed, the holder records `ingest` and `judge` as
   `(skipped: the turn passed to another computer)`.
7. **Delete against edit.** A note deleted on one computer and edited on the other ends archived on both, with the
   edit on the archived copy.
8. **Sync now** on the computer that does not hold the turn runs no machine step, and `/turn`'s log shows no call
   from it.

## 7. Out of scope

- **Three-desktop ordering** beyond what falls out: N17 (§5.4). A third computer is simply one more claimant.
- **Mobile and web** (the amendment's ruling 1), and a server-run engine (ruling 6) beyond noting that it fits (§5.5).
- **Moving the device-local ledgers into the account.** `state/ingest-seen.md` stays local: notes are never
  unlinked, so a moved turn finds every earlier item by `source_uid`. `state/events-seen.md` stays local with
  D8's fixes.
- **Note bodies** stay last-writer-wins per path (the review's row 19). A Backups restore of an account vault (M4)
  is not addressed. An agent-against-agent field conflict in the rare backstop case still files a card.
- **A Settings list of computers, a *Remove this computer* action, and a head start** for the last holder (§9).
- Whatever the cloud design parks (§13, ruling 6).

## 8. Fidelity: Quinn's decision, sentence by sentence

| Quinn's words (2026-09-24) | Carried by | Faithful? |
|---|---|---|
| "Is there a way we can do 1 AND make it so that we can track devices and try each one, one at a time?" DECISION: BOTH. | D1–D8 and D9–D16 | yes |
| "(1) Deterministic ids for imported notes, derived from (vendor, source uid)," | D1, D2 | yes, with the kind added (so a Gmail card and its task stay two notes) and a producer key where a note has no `source_uid` |
| "so every computer creates the same note" | D1, D3 | yes |
| "and sync merges it." | D5, D6 | yes, by id; paths are not made deterministic (D4) |
| "This is the backstop." | D13, D5 | yes: the turn fails open onto it |
| "(2) Device tracking with a fetch TURN: the account lists the student's computers" | D9 | yes |
| "(by the existing device token;" | D9 | yes: `sync::device_token` |
| "named by the Windows name, which the privacy page now discloses)." | D9, D10 | yes; the page also discloses the list itself (§3.3) |
| "At slot time a computer claims a short lease (~20 min) from the account." | D11 | **refined**: one 20-minute lease per job, claimed together at slot time. One computer holds them all in the common case (§9 Q1) |
| "The holder runs coursework, ingest and judge;" | D11, D12 | yes, per job held |
| "others skip those steps as named lines" | D12 | yes: §4.3's lines, at exit 0 |
| "and get the results through sync." | D12, D15 | yes: the holder's trailing push and the others' catch-up |
| "An expired lease passes the turn to the next slot on any computer." | D11, D14 | yes: a compare-and-set on expiry, and no computer holds a reserved turn |

## 9. Quinn's open questions

Each has a recommendation, and the draft is written to it.

- **Q1. One lease per job, or one lease?** *Recommend per job (D11).* With one lease, a computer without a portal
  login can win and leave that portal unfetched for the slot. With the logins split (zyBooks here, VHL there), no
  slot ever fetches both. In the common case the two designs behave identically.
- **Q2. Keep the catch-up (D15) in this stream?** *Recommend yes.* Without it, a student at the computer that
  lost the race sees the fetch only at its next slot, up to 18 hours later.
- **Q3. A head start for the last holder?** A computer that did not hold the turn last slot would wait 60 seconds
  before claiming. That keeps the turn on one computer and spares portal re-logins. *Recommend not now.* D8 makes
  a moving turn cheap, and the run records will show how often it moves.
- **Q4. Delete against edit (D7): archive quietly, or also file a "restore this?" card?** *Recommend archive
  quietly.* Nothing is lost and the note is in Archive.
- **Q5. Enforce the lease on the server?** *Recommend advisory now*, because desktops on the old build must keep
  working through the rollout. Enforce it for coursework when C5's `/relay` lands (§5.5).
- **Q6. The privacy version (D10).** If a new version is needed, does a new data category, the list of computers,
  need a re-ask screen? *Recommend no screen.* The list adds each computer's login presence and dates, not a new
  fact about the machine, and the version is recorded at the next sign-in.
- **Q7. The proof on two Windows users on one machine (§6.4) as the release gate?** *Recommend yes*, with Quinn
  signing in the second Windows user once, plus one confirmation run on a second physical computer if one is at
  hand (not a gate).
- **Q8. A read-only *Your computers* list in Settings?** *Recommend after the pilot.* The export already carries
  the list.
- **Q9. Converge doubles made before this ships (D6), or leave them?** *Recommend converge.* The affected vaults
  are test profiles and any early two-desktop user, and leaving them keeps I2's silent divergence alive for those
  notes.
