# Two desktops on one account: design

**Status: Draft for Quinn's review.** Written 2026-09-25 on branch `two-desktop-spec` at main `510a88c`; revised
the same day after the spec review (`docs/reports/2026-09-25-two-desktop-spec-review.md`) and the controller's
rulings on it. **Authority:** `docs/specs/2026-09-09-knowlu-cloud-design.md` and its *Amendment 2026-09-17* (ruling 2: the
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
- **Paths and config.** Each producer picks `<folder>/<slug>.md` and walks `-2`, `-3` past a taken name. The
  slug is not machine-independent: ingest's is `<course>-<title>` with the course from the vault's own
  `course_map`, and coursework's title carries a label from the same `config/ingest.yaml` (`zybooks.rs:296`).
  `config/` never syncs (`sync.rs:366-368`), so each desktop's config also decides which books it fetches at all
  (`coursework.rs:664-675`). Map cards (`map-<source>-<key>`), rule cards (`rule-<id>`) and the day's events
  digest (`events-digest-<date>`) have fixed paths and random ids, so two desktops put two ids at one path.
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
| **D2** | It covers the nine producers of §2.2, identified by `ids::import_key` over a closed vendor list, with every key prefixed by its producer. Hand-made and judgment notes keep random ids. | These are the notes an automatic step creates because an outside item exists: the ones two desktops create independently. | A producer left out keeps doubling in the backstop case; a hand-made note wrongly keyed would merge two different notes. |
| **D3** | Producers create through a new `write::create_imported`, which puts the import id where `create` would have minted one and refuses an id the vault already holds. | One path; the journal record and the frontmatter keep today's shape; only the id's value changes. | A key that repeats legitimately (a re-proposal) would be refused, so every re-proposable key carries its date. |
| **D4** | Paths keep today's naming and are **not** identity. Sync reconciles by id. | A path depends on per-desktop config and on collision order; fixing that costs readable names and still leaves the config. | The account can hold two text rows for one note in the rare backstop case; a restore keeps one. |
| **D5** | `apply` and restore work **by id**: records, moves and deletes find the local note by its id (or its alias, D6), never by a path that holds a different id; a pulled text whose id is already held elsewhere is not written; a tombstone settles the note at its path unless other desktops placed a different id there and never this one. | It is what makes "sync merges it" true when paths differ, and it closes E5 without regressing a hand delete. | One id index per apply. Without it, a path difference becomes a second file and ids diverge again. |
| **D6** | One imported item under two ids is joined by an **alias pre-pass** before step 4: the lower id wins, both desktops group records under it, and `Journal::records_for` follows the alias on both. The desktop holding the higher id then re-ids its note, which only tidies the id line. | It arises from doubles made before this ships and from the mixed-version rollout; a total order needs no coordination, and the journal already holds every `create` it needs. | Without it, a field set only on one copy before convergence would never reach the other: the silent divergence I2 named. |
| **D7** | A delete beats a concurrent edit (M5): the note is archived on both desktops and the edit lands on the archived copy. | Falls out of D5; nothing is lost. | A student who edits on one desktop what they deleted on the other finds it in Archive (§9 Q4). |
| **D8** | Event proposals and verdicts hold across desktops: a uid named in any events-digest note is already proposed; `rank --no-digest` (new) runs on a desktop that did not hold `feeds`; and every desktop's `sync` pulls the account's event verdicts, verdict and strength only, from `judgments` into its own ledger. | The event ledger `state/events-seen.md` is device-local. Without this, a moved turn re-proposes and re-judges, and a desktop that never holds `feeds` shows no *Coming up* events. | Pulled lines carry no reason, so a digest line for one shows none (§9 Q11). |
| **D9** | The account keeps a registry of computers: `devices(account_id, device, name, logins, first_seen, last_seen)`, written only by `/turn`, pruned daily by `pg_cron` 90 days after last seen. | The token is the one sync already sends; the name is already in every journal record; `logins` is what makes the turn useful. | Two Windows users on one PC share `COMPUTERNAME` and so one token (§3.2). |
| **D10** | The privacy page gains the words of §3.3. `PRIVACY_VERSION` moves only if 2026-09-24 was published before they land. | Precedent: R-C3′-exec-41's N4/N5 folded into an unpublished version. | A consent log that names the wrong version. |
| **D11** | The turn is **one lease per job**: `feeds` (ingest and judge) and one per portal (`zybooks`, `vhl`), claimed together in one single-statement compare-and-set on the server's `now()`, for 20 minutes, renewed before each machine step, released after the holder's trailing push. | Portal logins never leave a computer, so a computer without one cannot fetch it. In the common case one computer gets every job, which is Quinn's single holder. | If Quinn wants one lease (§9 Q1), a computer with no logins can win the turn and leave coursework unfetched for that slot. |
| **D12** | Holder: `sync → turn → coursework → ingest → judge → rank → sync (push) → release`. The others: `sync → turn → rank --no-digest`, recording the named skips of §4.3 at exit 0. | The results reach the account as soon as the holder is done, not at its next slot. | Without the trailing push, the others see the fetch two slots late. |
| **D13** | **Fail open**: a claim the service cannot answer (a 400 included) runs the machine steps as today, named `turn (unavailable: …; running anyway)`. | D1–D6 make a double run safe, and failing closed would stop all fetching during a lease outage. | While the lease is unavailable, judgments may be charged twice (§9 Q12). |
| **D14** | A holder that sleeps mid-slot loses the turn when its lease lapses. On waking, a refused renewal, or an unanswered one after the holder's own monotonic time since its last grant passes that grant's `seconds_left`, skips its remaining machine steps as `(skipped: the turn passed to another computer)`. | The server's clock decides; the device measures only its own elapsed time, never compares clocks. | The step in flight may be killed at `CHILD_TIMEOUT` on resume (an amber tray until the retry); D1–D6 absorb the overlap. |
| **D15** | A desktop that was refused a turn runs one **catch-up** `sync → rank` when the holder's lease ends, looking at most three times and never later than 60 minutes after its slot. | Otherwise "the others get the results through sync" means at their next slot, up to 18 hours later. | One extra pull per refused slot. |
| **D16** | The first slot at Finish and *Run now* claim like any slot. *Sync now* never runs a machine step and never claims, and answers "a slot is running" during a slot (§5.4, M6). The quit push releases nothing; expiry does. | *Sync now* is transport only today (`commands.rs:303`), and the lease must not change that. | None beyond D13. |
| **D17** | `config/` is split by owner. In this stream every desktop applies approved coursework-map cards to its own config at every slot, and a restoring wizard writes the account's calendar entries. Account-level config held by the account is a recommended companion stream (§9 Q10). | The turn makes the holder's config decide what is fetched and how it is labelled; these two fixes cover what the student approves after onboarding. | A mapping typed differently in two wizards still flips a title when the turn moves, until the companion lands. |

## 2. Deterministic ids for imported notes (D1–D8)

### 2.1 The derivation (D1)

```rust
pub fn import_id(kind: &str, vendor: &str, key: &str) -> String   // new, engine/src/ids.rs
// kind + "_" + hex(sha1("knowlu/import-id/1\n" + kind + "\n" + vendor + "\n" + key))[..10]
```

- **Inputs.** `kind` is what `ids::kind_for` answers for the note (`task`, `appr`), so the prefix is the one
  `write::create` would have minted. `vendor` is the note's own `created_by:` word. `key` is the producer's word
  from §2.2, a colon, and the note's `source_uid:` (or, where it has none, the producer's key), so no two
  producers can share a key (review M14). No input is per-desktop: all three come from frontmatter the producer
  writes from the item itself.
- **Encoding.** 10 lowercase hex characters, exactly `ID_RE`; SHA-1 and the cut are `derived_id`'s (an identity,
  not a security primitive). The `knowlu/import-id/1` line separates the hash from `derived_id`'s and versions it.
  10,000 imported notes carry under a one-in-10⁴ chance of any collision, and D3 refuses one rather than merging.
- **Reference values**, pinned by a test in `ids.rs` (computed with Python's `hashlib` and `sha1sum`):
  `import_id("task", "zybooks", "coursework:zybooks:1839992")` = `task_18734fe8b7`;
  `import_id("task", "blackboard", "lms:_blackboard.platform.gradebook2.GradableItem-_4732722_1")` =
  `task_d8891a504b`; `import_id("task", "gmail", "gmail:gmail:m1")` = `task_3bc4bec4a9`;
  `import_id("appr", "events", "events-digest:2026-09-25")` = `appr_40ab7c3a10`.

### 2.2 What it covers (D2)

`ids::import_key(path, meta) -> Option<(kind, vendor, key)>` (new) is the one function that says whether a
note is imported and under which key. It answers only for kind `task` or `appr` and a `created_by:` in the closed
list `ids::IMPORT_VENDORS` = `zybooks`, `vhl`, `blackboard`, `gmail`, `events`, `coursework`, `rules`. Never for
`quinn`, `dashboard`, an `agent:` actor, or any other word.

| Producer (today's random mint) | Note | vendor | key |
|---|---|---|---|
| `coursework::sync_coursework` (`coursework.rs:528`, `:573`) | `tasks/<slug>.md`; `archive/` when imported-past | `zybooks` / `vhl` | `coursework:<source_uid>` |
| `ingest::sync_tasks` (`ingest.rs:750`, `:781`) | `tasks/<course>-<slug>.md`; `archive/` when imported-past | `blackboard` | `lms:<source_uid>` (the feed's `UID`) |
| `enrich::write_gmail_note` (`:729`) | `tasks/<slug>.md` | `gmail` | `gmail:<source_uid>` |
| `enrich::write_gmail_card` (`:783`) | `approvals/task-<slug>.md` | `gmail` | `gmail:<source_uid>` (kind `appr`, so never the note's id) |
| `approvals::materialize` (`:1473`) | `tasks/<slug>.md` from an approved `kind: task` card | the payload's `created_by` | as that vendor's row keys it; random when the payload has no `source_uid` |
| `approvals::calendar_note` (`:1014`) | `approvals/calendar-event-<slug>.md` | `events` | `calendar-event:<source_uid>` |
| `eventemit::emit_digest` (`:240`) | `approvals/events-digest-<date>.md` | `events` | `events-digest:<proposed_at>` |
| `coursework::write_map_card` (`:1071`) | `approvals/map-<source>-<slug>.md` | `coursework` | `map:<source>:<map_key>:<first_proposed_at>` |
| `enrich::write_rule_card` | `approvals/rule-<rule_id>.md` | `rules` | `rule:<rule_id>` |

- **The date in two keys is load-bearing.** A map card is re-proposed 30 days after it expires, into a free
  `approvals/` path while the old card sits in `archive/`. Without the date, the new card would carry the old
  card's id, and D3 would refuse it for ever. Today "one digest a day" is only a path check
  (`eventemit.rs:263-267`), and a digest settled the same day frees the path; D3 makes it true, because a second
  digest that day is `IdHeld`.
- **Not covered.** The console's `create_task`, issues, info notices, amend cards, C5's login notice and the
  wizard's seeds keep `ids::new_id`: a person or a local judgment makes them, and seeds have C3′'s N2 pre-pass.
  Calendar busy time is not a note (`rank` reads `calendars:` into capacity), so it needs no id. The commitment
  model's notes are §5.6's.

### 2.3 Minting (D3)

`write::create_imported(vault, rel, text, ctx, journal, held: &mut BTreeSet<String>)` (new) reads the text's
frontmatter and asks `import_key`. With a key it puts `import_id` exactly where `create` puts a minted id (the
same `apply_frontmatter_fields_to_text` call) and journals the same `create` record; without one it is `create`.
It refuses an id already in `held` (the vault's ids from `ids::build_index`, built once per producer run and
extended as it creates) as `WriteError::IdHeld(<id>)`, logged `skipped (already held as <id>): <stem>`; every
producer already dedups by `source_uid` first, so this is the belt. The templates, the `id:` line's position,
the journal, the run records and the ledgers are unchanged; only the id's value differs.

### 2.4 Paths (D4)

Paths are **reconciled by id, not made deterministic**. A path built only from the item (`tasks/<title>-<hex>.md`)
would give every imported note a hex tail and would still differ wherever the title carries the per-desktop
label; a collision suffix derived from the id stays order-dependent. D5 makes a path difference harmless.

### 2.5 Apply and restore work by id (D5)

`sync::apply` builds `ids::build_index` (id → path) once, after its record pass, runs §2.6's alias pre-pass
before step 4, and follows these rules:

- **(a) Reconcile by id.** Step 4 finds the local note by the **id** of the foreign records, or by an id the
  alias map (§2.6) joins to it. It never reconciles against a note at the record's path that carries a
  different id; the path is consulted only for a local note with no `id:` line. The `write_literals` and
  `live_sync_cards` calls use the local path. A foreign `create` for an id held here is journalled and writes no
  file.
- **(b) Moves and deletes by id** (E5's move half). Step 3a performs a foreign `move` on the note holding the
  record's id, from wherever it is, to the record's `new` path, keeping today's `Exists` rule for a taken
  destination. A foreign `delete` (new here: today it has no effect of its own) settles the note holding its id
  through `write::delete` under `sync::ACTOR`, unless that note is already in `archive/`.
- **(c) One id, one file, and every id gets one.** Step 6 skips a pulled text whose own `id:` is already held
  here at another path (`sync: <path> is <id>, already held here as <local path>`), and one whose `import_key`
  names an item held here under another id, which the alias joins instead (§2.6). It writes a text whose id is
  new here at its own path when that path is free, and at the next free `-N` name (`write.rs`'s `free_slot` rule)
  when a note with a different id holds it; today that second text is dropped with no line
  (`sync.rs:2273-2275`). Restore's `materialise` applies the same rule against the ids it has already written,
  and for one id under two paths the row with the **highest** `rev` wins, E2's rule (`sync.rs:1062-1067`).
- **(d) A tombstone names a note, not a place** (E5's tombstone half). A pulled tombstone for `P` settles the
  local note at `P` as today, **unless** other desktops' records place a different id at `P` and none of them
  ever placed this note's id there. That tombstone is about another note, and this one stays:
  `sync: <P> — the account settled a different note there; this one stays`. "Other desktops' records" are
  journal records whose `device` is not this computer's name (§3.2's caveat applies). So a note created on B
  and deleted by hand in Explorer on A, with no `delete` record, is still archived on B.

The two cases the brief asks about are (a), a foreign `create` for an id held here, and (c), a text for a path
held by a different id.

### 2.6 One item under two ids: the alias pre-pass (D6)

The upgrade re-ids nothing by itself, and on one desktop an old random-id note never meets a new id (producers
find it by `source_uid`). Two ids for one item arise across desktops: I2's doubles made before this ships, and
the rollout, where an old build mints a random id for an item an updated desktop mints deterministically.

- **The pre-pass.** Before step 4, beside the N2 seed pre-pass, `apply` builds `import_key → ids` from the vault's
  notes and from every `create` record in the journal, this page's included. A `create` record's `new` is the
  whole frontmatter (`write.rs:416-418`), so `import_key` reads it as it reads a note. A key with two or more ids
  is an **alias group**, and its lowest id (as strings; the shared `kind_` prefix makes this compare the hex) is
  the winner. Both desktops compute the same group and winner, with no coordination and no knowledge of which id
  is deterministic. The map is saved as `state/id-aliases.json`: generated, device-local, never synced, rebuilt by
  every apply.
- **Records follow the alias, on both desktops.** Step 4 groups foreign records under their group's winner, and
  `Journal::records_for(id)` returns the records of every id in the group. On the desktop holding the winner, the
  loser's `set` records are foreign chains like any other: a field set only on the losing copy applies cleanly,
  and a field set on both becomes one sync card. On the loser's desktop its own records under the old id are
  `mine`. `journal.human_set` reads through the same map, so judge-once protects a field the student set under
  either id, and `ids::resolve_target` answers an old id with the group's note, so an issue's `target_id`
  (`issues.rs:217`) and anything else naming the old id keep working (review M3).
- **A group found for the first time is reconciled in full**, against every other desktop's records for all of
  its ids, not only this page's. `sync` runs the pre-pass on every run, pulled rows or not, so doubles made before
  this ships are found at the first sync after the upgrade. This is §9 Q9's mechanism.
- **Then the id line is tidied.** The desktop whose note carries a losing id re-ids it to the winner by one
  `write_literals` of `id` under `sync::ACTOR` (journalled, never sent): the repair `ids::ensure_ids` makes for a
  duplicate, through `write`. File, body and path stay, and nothing changes that the alias had not joined.
- **Rollout.** An old build has no pre-pass. The updated desktop joins the group at once; the old one converges
  when it updates (the updater checks daily), and shows the item twice until then, exactly as today.

### 2.7 Delete against edit (D7, review M5)

Under D5, whatever the push order: on B, A's `delete` record archives B's copy, edit and all (b); on A, B's
`set` records find A's archived copy by id (a); and B's live text cannot resurrect it, because its id is held in
`archive/` (c). The note ends archived on both, the edit visible in Archive, nothing lost, no card (§9 Q4).

### 2.8 Event proposals across desktops (D8)

Not in the review. The events pass writes verdicts to `state/events-seen.md` (device-local), and `rank`, on every
desktop, emits `approvals/events-digest-<date>.md` from it (`cli.rs:391`). Once `feeds` has moved, each desktop's
`rank` would emit its own digest for one day, and re-propose what the other proposed or the student declined,
because beyond its own ledger `eligible_events` excludes only uids in a **pending** digest (`eventemit.rs:75`).
`rank` also builds *Coming up* and `state/events.md` from that ledger (`relevant_events`, `write_roster`), so a
desktop that never holds `feeds` would show no events at all.

- **Proposed means named in any digest note**, in `approvals/` or `archive/`, whatever its status. A settled
  digest keeps its `events:` payload in `archive/`, and that note syncs.
- **Only the `feeds` holder emits.** `rank --no-digest` (new; the default is unchanged, so `oracle.rs` and the
  frozen references are untouched) runs on a desktop that did not hold `feeds` this slot (§4.3).
- **Verdicts travel through the account, without text.** The server already holds each event verdict: a
  `judgments` row, `kind = 'event'`, whose `fields` keep `verdict` and `strength` and, by design, no `why`
  (`fieldsOf`, `_shared/judge_pipeline.ts:77-78`). Every desktop's `sync` step, after its pull, calls
  `GET /judge-event?after=<cursor>` (a new route on the existing function) for the account's `answered` event rows
  judged after its cursor (`item_id`, `verdict`, `strength`, `title_prefix`, `judged_at`, 500 a page). For a uid
  with no ledger line it appends one through `eventledger::record_verdict` with an empty `why` (legal: it is then
  omitted) and the title prefix, or `(untitled)`. The cursor is a new, device-local `Cursor` field.
  - **It buys:** a new `feeds` holder judges only what no desktop has judged, so the account's 80-a-day
    `DAILY_CAP.event` (`_shared/judge_caps.ts:39`) is never spent twice on one event, and every desktop shows the
    same *Coming up*.
  - **It keeps:** no free text reaches the server (`judge_log.ts:4-7`), and `/judge-event`'s judging is unchanged.
    "Delete a line to force a re-judge" (`eventledger.rs:46-47`) still works: the cursor has passed that verdict,
    so the pull never restores the line, and this desktop re-judges when it next holds `feeds`. A pulled verdict
    never replaces a line the ledger has.
  - **It costs:** one GET per slot, and a digest line for a pulled verdict shows no reason (§9 Q11).

## 3. The device registry (D9, D10)

### 3.1 What the account stores

```sql
create table if not exists devices (                       -- new migration, <date>000100_fetch_turns.sql
  account_id uuid not null references public.accounts (id) on delete cascade,
  device     text not null check (device ~ '^[0-9a-f]{16}$'),   -- sync::device_token(account_id)
  name       text not null check (char_length(name) between 1 and 64),
  logins     text[] not null default '{}',                     -- portal logins held here, e.g. {zybooks,vhl}
  first_seen timestamptz not null default now(),
  last_seen  timestamptz not null default now(),
  primary key (account_id, device)
);
-- the daily prune: plain SQL run by pg_cron as postgres, like knowlu-sync-prune; no function, no Vault token
select cron.schedule('knowlu-devices-prune', '47 4 * * *',
  $$ delete from public.devices where last_seen < now() - interval '90 days' $$);
```

- **`device`** is the existing token, `sync::device_token(account_id)`, already the `device` column of every
  `sync_records` and `sync_notes` row this computer sent.
- **`name`** is `journal::device_name()` (`COMPUTERNAME`, or `KNOWLU_DEVICE`), cut to 64 characters with control
  characters removed. The same string is already inside every journal record the account holds.
- **`logins`** is the load-bearing column (not `sources`, which `public.sources`, the account's feeds, already
  names; review M11): the portal sources enabled in the vault's `coursework:` config with a credential present
  at their `credential_target` (`app/src/credentials.rs::exists`) and, after C5, not paused. Only the source's
  name leaves the machine. The turn grants a portal job only to a computer that lists it.
- **Written only by `/turn`**, on every call (`name`, `logins`, `last_seen`). RLS: select own, no client write
  policy, the C3′ shape.
- **Retention.** The daily `knowlu-devices-prune` job deletes a row 90 days after `last_seen` whether or not the
  account still calls `/turn`, so the privacy page's 90 days hold for an idle or lapsed account. `DELETE /account`
  purges `devices` and `fetch_turns` by name (`functions/account/index.ts:59`), and `GET /account/export` gains
  `devices: [{name, logins, first_seen, last_seen}]`, without the token, which means nothing to the student.

### 3.2 A limit shared with C3′

`COMPUTERNAME` is per machine. Two Windows users on one PC and one account share a token, so both get the turn
and D1–D6 merge the double run. They also share the journal's `device`, which C3′ reads as "this device"
(`sync.rs:1961-1968`, `reconcile.rs:55-57`), so this is a C3′ limit too (review M6). Rare, accepted; §6.4
works around it.

### 3.3 The privacy page (D10)

In *Your tasks and notes* (`site/privacy.html:36`), the sentence *"Each change in the journal, and each issue you
raise, also carries the name Windows gives the computer it was made on, so that Knowlu can tell your computers
apart."* becomes:

> Each change in the journal, and each issue you raise, also carries the name Windows gives the computer it was
> made on. Your account also keeps a list of the computers you use Knowlu on: each one's Windows name, when it was
> first and last seen, and which of your coursework sites it holds a login for, never the login itself. Knowlu
> uses that list so that your computers take turns fetching your coursework, school calendar and email and having
> them judged, instead of each doing the same work. Each computer still reads your event and calendar feeds for
> itself when it plans your day. A computer leaves the list 90 days after it was last seen, and deleting your
> account deletes the list.

In *Your coursework logins* (`:40`), after "there is no column in our database for one", add:

> Your account does know which of your computers holds a login for which site, so that the computer with the
> login is the one that takes its turn.

The *Export* bullet (`:103`) adds "and the list of your computers". `engine/tests/site.rs` pins all three, in the
shape of `the_review_amendments_i2_i3_and_i4_are_on_the_page`. Each clause holds under per-job turns, under D13
(a purpose, "so that", not a promise), and beside `rank`'s own feed reads (the fourth sentence).

**`PRIVACY_VERSION`** (`app/src/account.rs:25`, `2026-09-24`): if that version is still unpublished when this
merges, the words fold into it, as R-C3′-exec-41 did for N4 and N5; otherwise the constant moves to the new
publish date and `record_consent_at` records it at each student's next sign-in (§9 Q6).

## 4. The fetch turn (D11–D17)

### 4.1 One lease per job, one statement per claim (D11)

A **job** is a unit of machine work that one computer at a time should do: `feeds` (the `ingest` and `judge` steps,
meaning the LMS feed, the four cloud passes `enrich::run_lines_with` hosts, events, Gmail and rule decisions), and
one job per portal (`zybooks`, `vhl`, and after C5 whatever `relay::PORTAL_SOURCES` names). `feeds` needs nothing
device-held, so any signed-in computer can hold it. A portal job needs that portal's login, so only a computer
listing the source in `logins` can hold it. In the common case, one awake computer with every login, one computer
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
-- inside fetch_turn(p_account, p_device, p_name, p_logins, p_claim, p_release): the claim is ONE statement
insert into fetch_turns as t (account_id, job, device, claimed_at, expires_at)
  select p_account, j, p_device, now(), now() + interval '20 minutes'
    from unnest(p_claim) as j where j = 'feeds' or j = any (p_logins)
    order by j                                   -- one lock order for every claimant (review M1)
on conflict (account_id, job) do update
  set device = excluded.device, claimed_at = excluded.claimed_at, expires_at = excluded.expires_at
  where t.expires_at <= now() or t.device = excluded.device;
```

- **Atomic.** `on conflict … do update … where` takes the row lock; a concurrent claimant waits and its `where`
  is evaluated against the committed row (READ COMMITTED), so two claims of one expired job grant exactly one,
  row or no row. The holder's own device always passes, which is the renewal (`claimed_at` records the last
  one). `order by j` gives every claimant one lock order, so overlapping claims cannot deadlock.
- **`fetch_turn`** (new, plpgsql, not SECURITY DEFINER; revoked from `public, anon, authenticated`) runs the
  `devices` upsert; the release (`update fetch_turns set expires_at = now() where account_id = p_account and
  device = p_device and job = any(p_release) and expires_at > now()`); the claim; and returns every job row of
  the account as `(job, mine, seconds_left)`.
- **The clock is the server's.** Only `now()` decides; the reply carries `seconds_left`, never a timestamp, and
  the device never compares clocks (the review's I1 lesson). The client cannot ask for a longer lease.
- **20 minutes, renewed before each machine step.** C5's run budget (10 minutes), `enrich::BATCH_BUDGET`
  (15) and `scheduler::CHILD_TIMEOUT` (20) bound every step, so a renewal outlives its step unless the step is
  killed at its cap (D13, D14, D1–D6).
- **Released** after the holder's trailing push (§4.3); a failed release costs at most 20 minutes.

### 4.2 The endpoint

`POST /turn` is a new function, `cloud/supabase/functions/turn/{index,handler,handler_test}.ts` with
`_shared/turn_db.ts`; `index.ts` wraps the handler with `requireActiveEntitlement` in `sync-push/index.ts`'s
try/catch shape, and `config.toml` gets `[functions.turn] verify_jwt = false`. The body is
`{"device", "name", "logins", "claim", "release"}`; the account id comes only from the verified JWT. `device` must
pass `_shared/sync_rows.ts::isDeviceToken`, `name` is 1–64 characters after control characters are stripped, and
the three lists hold at most 8 words matching the `job` check. The reply is
`{"turns": [{"job": "feeds", "mine": true, "seconds_left": 1200}, …]}`; refusals are 400, 401, 402 and 405 in
`_shared/http.ts`'s shape. **A peek** (empty `claim` and `release`) refreshes `last_seen` and reads the turns.
The device side is `app/src/turn.rs` (new; no Tauri command), authenticating through
`account::valid_access_token_at` under `SESSION_REFRESH_LOCK` like every account call.

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

A computer that does not run `coursework` this slot still applies approved map cards to its own config first
(§4.8).

The exact steps recorded, every one at exit 0 (so never retry backoff and never an amber tray):

| Case | Step line in `RunSummary` |
|---|---|
| granted one or more jobs | `turn (held: feeds, zybooks)` (jobs in the order `feeds`, then portal-table order) |
| granted nothing | `turn (another computer has it)` |
| a machine step whose job another computer holds | `coursework (skipped: another computer has the turn)`, and the same words for `ingest` and `judge` |
| some of this computer's portals held elsewhere | `coursework (zybooks only; another computer has vhl)`, then the step runs `--only zybooks` |
| a coursework site set up in the vault, but no login for it here | `coursework (skipped: no coursework login on this computer)` |
| no coursework site set up in the vault at all | `coursework (skipped: no coursework site set up)` |
| a renewal refused (§4.5) | `<step> (skipped: the turn passed to another computer)` |
| the service cannot answer (§4.4) | `turn (unavailable: <cause>; running anyway)` |
| not entitled | `turn (skipped: no entitlement)` |
| the release failed | `turn (release failed: <cause>)` |

The runner-log filter (`scheduler.rs:792-808`) adds the `coursework (skipped:` prefix (the `ingest` and `judge`
lines already pass) and the two failures without their cause, `turn (unavailable; running anyway)` and
`turn (release failed)` (R-C1c-final-3), all at status `ok` (R-C1c-plan-4). `FIRST_RUN_SAYS` gains
`turn: "Checking which computer fetches"`. `slot_argv` takes the claim's outcome as a new `TurnPlan`; whether it
splits into pre-turn and post-turn halves is the plan's choice, but this order is fixed.

### 4.4 Fail open (D13)

A claim that meets a transport failure, a timeout, a 5xx, a 404 (the function not yet deployed, as on production
before parity), a 401 the pre-flight could not prevent, a 400 (a request of ours refused, such as a name empty
once stripped: our bug must not stop fetching), or an unparseable reply runs the slot **as today**: every machine
step, every source, `rank` without `--no-digest`, and the trailing push; nothing to release. The step is
`turn (unavailable: <cause>; running anyway)`. A renewal failing the same way follows §4.5. Offline, the machine
steps name their own skips anyway.

**A cost to name** (review M5): under D13, or two claims racing past a lapsed lease, both desktops can emit
today's digest under one id with different payloads, and D5 (c) keeps each desktop's own file (a body never
overwrites a held file, `sync.rs:2273-2275`): one card, two lists, until the day ends. Nothing is lost.

### 4.5 A holder that sleeps (D14)

A lid closed mid-slot freezes the children. The lease lapses 20 minutes after the last renewal, and the next slot
(or *Run now*) on another computer claims the job. On waking:

- **The renewal** is the same claim statement: granted if nobody took the job meanwhile, refused otherwise. On
  resume the network is often still coming up, so a renewal the service **cannot answer** counts as granted only
  while the elapsed time since the last grant, on a local `std::time::Instant` taken when it arrived, is under
  that grant's `seconds_left`; past that it counts as refused. One local duration against one the server gave:
  no clocks compared.
- **A refusal** skips the remaining steps for the lost jobs as `(skipped: the turn passed to another computer)`.
  `rank` still runs, with `--no-digest` if `feeds` was lost. The trailing push still runs, since it carries what
  the sleeper wrote, and the release touches only rows the sleeper still holds.
- **The step in flight** is timed on the app's own `Instant` (`CHILD_TIMEOUT`, `scheduler.rs:28`, `:561-569`).
  After a suspension, or a sleep that `Instant` counts, it is killed on resume as `-2`, which sets
  `engine_ok = false` (`:821-823`): an amber tray and retry backoff. The retry is a new slot that claims afresh,
  records the skip lines and turns the tray green. Whether `Instant` counts a real Windows sleep is unverified;
  `pssuspend` (§6.4) stops the app too, which makes the kill certain. What the killed step wrote, D1–D6 merge.

### 4.6 The catch-up (D15)

A computer refused any job it asked for records the largest refused `seconds_left`, and the scheduler's tick runs
a **catch-up** 60 seconds after it: a peek, waiting again while a refused job is still held (at most three looks,
never past 60 minutes after the slot began), then the session pre-flight, `sync` both ways and `rank --no-digest`,
recorded as their own `RunSummary` with `reason: "catch-up"`. It never claims, so it can never become a second
fetch, and a slot that starts first supersedes it.

### 4.7 The first slot, *Run now*, *Sync now*, quit (D16)

- **The first slot at Finish** and ***Run now*** claim like any slot. On a restored computer a refusal costs
  nothing: the restored day is the account's, and the catch-up brings the holder's run.
- ***Sync now*** stays transport only (`commands::sync_inner` → `state::run_sync` → `sync`): no machine step, no
  claim. A test pins it.
- **Quit.** `state::quit_flush` pushes as today and releases nothing; the lease lapses within 20 minutes.

### 4.8 Whose config the holder uses (D17)

Under a moving turn the holder's `config/ingest.yaml` decides which books it fetches (`coursework.rs:664-675`) and
each title's label (`zybooks.rs:296`), which the update branch rewrites whenever it differs (`:424-429`). An
approved mapping reaches only the config of the computer running `coursework` (`apply_map_cards`, `:1170`), so
another computer skips that book whenever it wins and files a second card once the first card's 30 days lapse.
The C3′ review's I2 named this, and a restored desktop whose `calendars:` lacks `- name: personal`.

| Owner | Values in `config/` |
|---|---|
| **the account** (one value for every computer) | `course_map`; each coursework source's `courses:`/`sections:` mappings, and the labels derived from them; `timezone`; the `calendars:` entries that name account-held feeds (`cloud:personal`, `cloud:google`); `events.yaml`'s sources; `campus.yaml`; `planning.yaml`, `week_template.yaml` |
| **the computer** | each source's `credential_target` and `enabled` (C5 §7 keeps both device-side); `runners.yaml`'s `device:` and `scheduler:`; `cloud.yaml`, whose `session_credential_target` names the profile id; a vault-held `ics_url` fallback |

**This stream's two fixes**, the smallest that make every holder fetch and label what the student approves:

- **Approved map cards apply everywhere.** A new `coursework::apply_approved_mappings(vault)` inserts the mapping
  of every approved `coursework-map` card, in `approvals/` or `archive/`, that this config lacks: `write_mapping`'s
  text insertion, idempotent, config only (archiving the card stays with `apply_map_cards`). `coursework` calls it
  first; on a computer not running `coursework`, the slot calls it in-process under `vault_io`. Cost: one read of
  `approvals/` and `archive/` per slot.
- **A restoring wizard writes the account's calendars.** At Finish, a cloud vault's `calendars:` gets
  `cloud:personal` when the account holds a `calendar_ics` source (`GET /account/sources`, a new device helper)
  and `cloud:google` when `account::google_connected` says so, whether or not this wizard pasted one.

**What stays per computer until a companion stream:** a value typed differently in two wizards (a course code, a
label, a timezone). That book is fetched only where config maps it, and its title flips when the turn moves between
labels. The backstop stays honest: D1 keys on `source_uid`, never config, so a flip changes one note's `title` and
never makes a second note. The full answer, account-level config held by the account and written into `config/`
by text insertion (ruling 3 already names `config/` parameters as cloud data), is a stream of its own, recommended
as a **companion finished before any student is told two desktops work** (§9 Q10).

## 5. Interactions

### 5.1 The entitlement gate

`/turn` is gated by `requireActiveEntitlement`. The slot never claims past the grace (§4.3), the engine's gate
(`entitle.rs`) still names the machine steps' skips, and `rank`, never gated, runs with `--no-digest` on a computer
that claimed nothing. A 402 from `/turn` is `turn (skipped: no entitlement)`, never the fail-open line.

### 5.2 The session refresh

C1c's pre-flight (`ensure_session_for_at(.., 45 * 60)`, R-C1c-13) runs before `sync` and so before the claim.
Renewals, the release and the catch-up's peek, up to an hour later, go through `valid_access_token_at` under
`SESSION_REFRESH_LOCK`; the catch-up runs its own pre-flight first.

### 5.3 The first-day cap

**Per account**, unchanged (`charge_call`, R-C1c-7). A later computer restores notes already judged, judges only
what is new while it holds `feeds`, and D8's verdict pull keeps a moved turn from re-judging events.

### 5.4 The rows the review deferred to I2

| Row | Ruling | Why |
|---|---|---|
| 11, N17: a late, older third-desktop write settles a card under a false warning | **Out** | Reconcile's ordering across three desktops, which ids and the lease do not touch; the lease only makes it rarer. Its recipe (R-C3′-exec-19) stands for a later stream. |
| 30, E5: a foreign `move` does not check the note's id at the old path; tombstones lack the same check | **In** | D5 (b) and (d). Once paths may differ, a path-keyed move or tombstone can hit a different note. |
| 40, M5: delete against edit resolves by push order | **In** | D7. By id, the delete wins on both desktops and the edit is kept on the archived copy. |
| 41, M6: *Sync now* beside a slot's later child | **In, narrowly** | Widened by one case (review M13): a *Sync now* `apply` beside a producer child can put one deterministic id in two local files, which `ensure_ids` (`cli.rs:314`, `ids.rs:171-188`) re-ids at random and D6 does not merge. So this stream takes the review's cheap guard: *Sync now* answers "a slot is running" while `sch.running`. |

### 5.5 C5, the relay fetch

C5 (`2026-09-17-c5-relay-fetch-design.md`) keeps the device as the credential relay, so the turn keeps its shape:

- The portal jobs are C5's sources. `coursework --only <held portals>` becomes the source set the relay run
  offers in its first call's `client` block. A source C5 has **paused** after a rejected login is left out of
  `logins`, so a computer with a dead password never keeps the turn from one with a good one.
- **Recommended for C5's plan:** `/relay` carries the device token and answers `409 not this computer's turn` for
  a source whose job the caller does not hold, recorded as the ordinary skip line. That enforces the lease for
  coursework. It stays advisory elsewhere, because old builds must keep working through the rollout, but `/relay`
  has no old clients.
- A moved portal job logs in once on the new holder (C5's session store is per computer). R-C1c-2's pre-judging
  makes `feeds` cheaper, not different. A server-run engine would be one more holder.

### 5.6 The commitment model

Its `commitments/` notes carry a `source_uid` (`gcal-series:…`, `registrar:ua:<term>-<crn>`) but **no
`created_by:`** (its §2's field table), so `import_key` keys them by that prefix under kind `cmt` (its R3), not
through `IMPORT_VENDORS`; its §2.5 already keeps the lowest id among cross-desktop duplicates, which is D6's rule.
Its registrar fetch becomes a portal job and its Google series pull rides `feeds`. Both programs edit `sync.rs`'s
`apply` and `ids.rs` (`KINDS`, `ID_RE`, `NOTE_FOLDERS`), so whichever lands second makes a shared-file merge.

## 6. Tests and proof

### 6.1 Engine (unit, and `engine/tests/sync_contract.rs`)

- `ids.rs`: the four reference values of §2.1; `import_key` is `None` for `created_by: quinn`, `dashboard`, an
  `agent:` actor, and kinds other than `task` and `appr`.
- `sync_contract.rs`, two vaults exchanging pages the way `two_desktops_with_a_card` and the N2x tests already do:
  - (i) the review's I2 scenario: both create item x, exchange, set `importance` 5 on A and 2 on B, then an
    unrelated `coursework` update on A. The result is one card, never silent divergence;
  - (ii) differing course maps put x at two paths: one file each side, and a later edit travels;
  - (iii) delete on A against edit on B: archived on both, the edit on the archived copy;
  - (iv) E5 and I3: a tombstone for a path holding a different id leaves it; a foreign move acts on the id; a note
    created on B and deleted by hand on A (no record) is archived on B;
  - (v) a pre-existing pair under two random ids, created and pulled **before** the upgrade, is joined at the
    first sync after it: the lower id on both, a field set only on the losing copy ends **equal on both**, and
    a field set on both becomes one card;
  - (vi) a restore given two rows for one id writes one file, from the highest `rev`.
- The verdict pull: a pulled verdict fills a missing ledger line with no `why`, never overwrites one, and a line
  the student deleted is not put back.
- `apply_approved_mappings`: an approved card in `archive/` maps a book in this config, once.
- `eventemit`: a uid in an archived digest is never proposed again; `rank --no-digest` writes no digest; `oracle.rs`
  and `surface_oracle.rs` are unchanged.

### 6.2 App (`app/tests/scheduler.rs`, plus a loopback test for `turn.rs`)

- The exact lines of §4.3, and the order session < `sync` < `turn` < `coursework` < `ingest` < `judge` < `rank` <
  `sync (push)` < release.
- An unanswered renewal is granted inside the last grant's `seconds_left` of local `Instant` time and refused past
  it; a refused renewal skips the rest.
- The catch-up is scheduled, peeks, never claims, and a slot supersedes it.
- `turn.rs` against a loopback server: full grant, partial grant, refusal, 400, 402, 5xx, 404 and transport.
- *Sync now* runs no machine step, makes no `/turn` call, and answers "a slot is running" during a slot.

### 6.3 Cloud: the migration and RLS guards

- `turn/handler_test.ts`: validation, the account taken from the JWT only, and the reply shape.
- `judge-event`'s new GET: only this account's `answered` event rows after the cursor, `verdict`, `strength`,
  `title_prefix` and `judged_at` only, paged at 500; a POST judges exactly as before.
- The `account` function: `devices` and `fetch_turns` are in the purge list and `devices` is in the export, each
  pinned by a test.
- `migrations_test.ts`:
  - the function pin moves from **28 to 29** (`fetch_turn`), with its comment; the prune is plain SQL in
    `cron.schedule`, so it adds no function;
  - `fetch_turn` is revoked from `public, anon, authenticated`, and the `knowlu-devices-prune` job exists;
  - the view pin stays 5.
- **The RLS guard is widened first.** It reads only C2's `20260911*` files (`ours()`, `:8`) and matches only
  `create table if not exists <unqualified name>` (`:274`), so nothing holds C1's or C3′'s tables to RLS (they do
  enable it). The widened guard runs over `everyMigrationFile()`, takes an optional `public.` and a bare
  `create table`, and allows any whitespace before `enable row level security` (`20260912000100_sync.sql:205-208`
  aligns it). Today's corpus and the two new tables must pass.
- **A staging check by the controller** (never a subagent): two concurrent claims of one expired job in two
  database sessions grant exactly one.

### 6.4 The live proof that gates release

**The gate.** No student or release note is told that two desktops work until this passes on staging (HANDOFF §4's
production-parity row already waits on it).

**Two Windows users on one machine are acceptable.** Everything this design keeps per computer is per Windows
user: `%LOCALAPPDATA%\knowlu\`, the vault under `%USERPROFILE%\Knowlu\` (journal, sync cursor, event ledger),
Credential Manager, and the scheduler with its tray (the single-instance plugin is per session). The one
per-machine input, `COMPUTERNAME`, is replaced by a per-user `KNOWLU_DEVICE`, which `journal::device_name()` reads
first, so the two users are two tokens as two machines would be (one negative check without it shows §3.2's
limit). The stand-ins: clock skew does not matter, since only the server's clock decides (I1's clamp has its own
proof); step 4's 503 stub replaces a partition; suspending the holder's whole app (Sysinternals `pssuspend`) is
the same silence to the server as a lid close.

**The procedure.**

- Creating a second Windows user on Quinn's machine is **Quinn's to approve**, and Quinn signs it in once: a
  Windows password the controller never holds.
- Each user's `KNOWLU_DEVICE` is set **before its wizard runs**; otherwise `runners.yaml`'s `device:` names
  `COMPUTERNAME` and `device_ok` refuses every slot (`scheduler.rs:456-461`, `:618-620`).
- Step 5's "previous release" is a dev build of that release's commit pointed at staging (the published build
  talks to production).
- Both users stay signed in by fast user switching; the controller drives each app through its own WebView2
  remote-debugging port, DOM only (the autonomous-proof harness).

**What it must show** (staging, one account; both scratch profiles removed afterwards, per the standing rule):

1. Both computers in `devices`, with their names and `logins`, read back by the controller.
2. *Run now* on both within seconds. Each job is held by exactly one computer. The other's `RunSummary` carries
   `turn (another computer has it)` and the three skip lines, and its catch-up brings the holder's notes within
   about 20 minutes.
3. A new event on the staging test feed appears once on both computers, with the same id and path, no second
   file, and one `judgments` row for it.
4. **Fail open.** With `/turn` deployed as a 503 stub for this step, and restored afterwards, both computers
   fetch, and the new item still ends as one note with one id on both.
5. **Mixed versions.** One user on the previous release and one on the candidate. A doubled item is joined on the
   lower id, with a field set only on one copy equal on both and at most one card: first on the candidate, then
   on both once the old one updates.
6. **Sleep.** The holder's whole app is suspended mid-`coursework` for more than 20 minutes, and the other
   computer takes the jobs on its next *Run now*. Once resumed, the holder records `coursework` with code `-2`
   (killed at `CHILD_TIMEOUT`), then `ingest` and `judge` as `(skipped: the turn passed to another computer)`,
   and its tray turns amber. Its retry slot records `turn (another computer has it)` and the skip lines, and
   the tray returns to green (§4.5).
7. **Delete against edit.** A note deleted on one computer and edited on the other ends archived on both, with the
   edit on the archived copy.
8. **Sync now** on the computer that does not hold the turn runs no machine step, and `/turn`'s log shows no call
   from it.
9. **Config and events.** A map card approved on the computer without the turn maps the book in both configs by
   the next slot, and both computers' pages show the same *Coming up* events.

## 7. Out of scope

- **Three-desktop ordering** beyond what falls out: N17 (§5.4). A third computer is simply one more claimant.
- **Mobile and web** (the amendment's ruling 1), and a server-run engine (ruling 6) beyond noting that it fits (§5.5).
- **Moving the device-local ledgers into the account.** `state/ingest-seen.md` stays local: notes are never
  unlinked, so a moved turn finds every earlier item by `source_uid`. `state/events-seen.md` stays local, filled
  by D8's verdict pull.
- **Account-level config held by the account** (§4.8): the recommended companion stream (§9 Q10).
- **Note bodies** stay last-writer-wins per path (the review's row 19). A Backups restore of an account vault (M4)
  is not addressed. An agent-against-agent field conflict in the rare backstop case still files a card.
- **A Settings list of computers, a *Remove this computer* action, and a head start** for the last holder (§9).
- Whatever the cloud design parks (§13, ruling 6).

## 8. Fidelity: Quinn's decision, sentence by sentence

| Quinn's words (2026-09-24) | Carried by | Faithful? |
|---|---|---|
| "Is there a way we can do 1 AND make it so that we can track devices and try each one, one at a time?" DECISION: BOTH. | D1–D8 and D9–D17 | yes |
| "(1) Deterministic ids for imported notes, derived from (vendor, source uid)," | D1, D2 | yes, with the kind added (so a Gmail card and its task stay two notes), the key prefixed by its producer, and a producer key where a note has no `source_uid` |
| "so every computer creates the same note" | D1, D3 | yes |
| "and sync merges it." | D5, D6 | yes, by id and, for an item already under two ids, by the alias pre-pass; paths are not made deterministic (D4) |
| "This is the backstop." | D13, D5 | yes: the turn fails open onto it |
| "(2) Device tracking with a fetch TURN: the account lists the student's computers" | D9 | yes |
| "(by the existing device token;" | D9 | yes: `sync::device_token` |
| "named by the Windows name, which the privacy page now discloses)." | D9, D10 | yes; the page also discloses the list itself (§3.3) |
| "At slot time a computer claims a short lease (~20 min) from the account." | D11 | **refined**: one 20-minute lease per job, claimed together at slot time. One computer holds them all in the common case (§9 Q1) |
| "The holder runs coursework, ingest and judge;" | D11, D12, D17 | yes, per job held; every computer's config carries the mappings the student approved, so any holder fetches them |
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
- **Q6. The privacy words (D10).** Two parts. If a new version is needed, does a new data category, the list of
  computers, need a re-ask screen? *Recommend no screen.* The list adds each computer's login presence and dates,
  not a new fact about the machine, and the version is recorded at the next sign-in. And the page prints a
  retention promise: a computer leaves the list **90 days** after it was last seen, kept true for idle and lapsed
  accounts by the daily prune (§3.1). *Recommend 90 days*, a semester's gap with room to spare.
- **Q7. The proof on two Windows users on one machine (§6.4) as the release gate?** *Recommend yes*, with the
  procedure §6.4 lists (Quinn approves and signs in the second Windows user), plus one confirmation run on a
  second physical computer if one is at hand (not a gate).
- **Q8. A read-only *Your computers* list in Settings?** *Recommend after the pilot.* The export already carries
  the list.
- **Q9. Join doubles made before this ships (D6), or leave them?** *Recommend join*, by the alias pre-pass, which
  finds them at the first sync after the upgrade. The affected vaults are test profiles and any early two-desktop
  user, and leaving them keeps I2's silent divergence alive for those notes.
- **Q10. Config under a moving turn (D17, §4.8).** Hold account-level config (course map, source mappings,
  labels, timezone, account-held calendars, event sources) in the account now, as a companion stream, or accept
  per-desktop config, with approved map cards applied everywhere and the calendar fix, until C5 or later?
  *Recommend the companion, finished before any student is told two desktops work.* This stream's two fixes cover
  what the student approves after onboarding. A value typed differently in two wizards still flips a title
  whenever the turn moves, and that is visible.
- **Q11. Event verdicts (D8, §2.8).** Should every desktop pull the account's verdicts, verdict and strength only
  and never text, from `judgments`? Or should verdicts stay device-local, accepting that a moved turn re-judges
  within the 80-a-day cap and that a desktop that never holds `feeds` shows no *Coming up* events? *Recommend the
  pull.* It stores nothing new on the server; its one cost is a digest line with no reason.
- **Q12. Fail open (D13).** While `/turn` cannot answer, every computer fetches and judges, so judgments can be
  charged twice, bounded by the daily caps. *Recommend open*, because closing would stop all fetching on every
  computer of the account for the length of an outage.
